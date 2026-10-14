//! Support shared by the integration tests: the C compiler exactly as the build script used
//! it, and scratch directories below cargo's target directory.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// The vendored Ruby tree.
pub(crate) fn source_dir() -> PathBuf {
    PathBuf::from(env!("VKRGSS_RUBY_SOURCE_DIR"))
}

/// A fresh, empty directory for one test, below cargo's directory for test files.
pub(crate) fn scratch_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    }
    fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    dir
}

/// The compiler with the flags and defines the interpreter was built with, the include paths
/// of `config.h`, the vendored tree and what its `re.h` includes, and a temporary directory
/// that is surely writable.
pub(crate) fn compiler(scratch: &Path) -> Command {
    let mut command = Command::new(env!("VKRGSS_RUBY_CC"));
    command
        .args(
            env!("VKRGSS_RUBY_CFLAGS")
                .split(' ')
                .filter(|arg| !arg.is_empty()),
        )
        .arg("-I")
        .arg(env!("VKRGSS_RUBY_CONFIG_DIR"))
        .arg("-I")
        .arg(source_dir())
        .arg("-I")
        .arg(env!("VKRGSS_RUBY_COMPAT_DIR"))
        .arg("-I")
        .arg(env!("VKRGSS_RUBY_ONIG_DIR"))
        .env("TMPDIR", scratch);
    command
}

/// Runs a command to completion and returns its standard output; panics with both output
/// streams if it fails.
pub(crate) fn run(command: &mut Command) -> String {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("could not start {command:?}: {error}"));
    assert!(
        output.status.success(),
        "{command:?} ended with {}\n--- stdout\n{}\n--- stderr\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

/// Compiles one C file to an object file in `scratch` and returns the object's path.
pub(crate) fn compile_object(scratch: &Path, source: &Path) -> PathBuf {
    let stem = source.file_stem().expect("a file name");
    let object = scratch.join(stem).with_extension("o");
    run(compiler(scratch)
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(&object));
    object
}

/// Writes the `Init_ext` of Ruby's test executable into `scratch` and returns its path: it
/// registers `Zlib` and the `fcntl` and `stringio` extensions that Ruby's tests load.
pub(crate) fn write_init_ext(scratch: &Path) -> PathBuf {
    write_init_ext_with(scratch, "", "")
}

/// As [`write_init_ext`], with more C definitions in front of `Init_ext` and more statements
/// at its end.
pub(crate) fn write_init_ext_with(scratch: &Path, definitions: &str, statements: &str) -> PathBuf {
    let init_ext = scratch.join("extinit.c");
    fs::write(
        &init_ext,
        format!(
            "#include \"ruby.h\"\n\
             void ruby_init_ext _((const char *name, void (*init)(void)));\n\
             void Init_zlib _((void));\n\
             void Init_fcntl _((void));\n\
             void Init_stringio _((void));\n\
             __attribute__((visibility(\"default\"))) int vkrgss_export_probe = 1;\n\
             {definitions}\
             void Init_ext _((void))\n\
             {{\n\
             \x20   ruby_init_ext(\"zlib.so\", Init_zlib);\n\
             \x20   ruby_init_ext(\"fcntl.so\", Init_fcntl);\n\
             \x20   ruby_init_ext(\"stringio.so\", Init_stringio);\n\
             {statements}\
             }}\n"
        ),
    )
    .expect("extinit.c is writable");
    init_ext
}

/// Builds Ruby's own command-line executable from the archives the build script produced:
/// `main.c`, the `fcntl` and `stringio` extensions that Ruby's tests load, and an `Init_ext`
/// that registers them and `Zlib`. `-rdynamic` exports every symbol of default visibility, as
/// a real `ruby` does, which is also what makes the executable a probe for leaked symbols:
/// `vkrgss_export_probe` is the one symbol given default visibility on purpose, to show that.
pub(crate) fn build_ruby_executable(scratch: &Path) -> PathBuf {
    let source = source_dir();
    let init_ext = write_init_ext(scratch);

    let objects = [
        compile_object(scratch, &source.join("main.c")),
        compile_object(scratch, &init_ext),
        compile_object(scratch, &source.join("ext/fcntl/fcntl.c")),
        compile_object(scratch, &source.join("ext/stringio/stringio.c")),
    ];

    // The objects come first so that their `Init_ext` wins over the empty one of `dmyext.o`.
    let executable = scratch.join("ruby");
    let ruby_libs = Path::new(env!("VKRGSS_RUBY_LIB_DIR"));
    let zlib_libs = Path::new(env!("VKRGSS_RUBY_ZLIB_LIB_DIR"));
    run(compiler(scratch)
        .arg("-rdynamic")
        .args(&objects)
        .arg(ruby_libs.join("libvkrgss-ruby-zlib.a"))
        .arg(ruby_libs.join("libvkrgss-ruby-static.a"))
        .arg(ruby_libs.join("libvkrgss-ruby-onig.a"))
        .arg(zlib_libs.join("libz.a"))
        .args(["-lcrypt", "-lm"])
        .arg("-o")
        .arg(&executable));
    executable
}

/// The compiler of the instrumented builds (the sanitized run and the fuzz targets). A
/// distribution may package the sanitizer runtimes of clang only, as Fedora does with
/// `compiler-rt`, so this is not necessarily the compiler the build script used.
pub(crate) const INSTRUMENTED_COMPILER: &str = "clang";

/// One C file to compile: the command and the object file it writes.
pub(crate) struct Job {
    command: Command,
    object: PathBuf,
}

/// The instrumented compiler with the flags of the build script, the instrumentation flags
/// `flags` and the include paths given.
pub(crate) fn instrumented_compiler(scratch: &Path, includes: &[&Path], flags: &[&str]) -> Command {
    let mut command = Command::new(INSTRUMENTED_COMPILER);
    command
        .args(
            env!("VKRGSS_RUBY_CFLAGS")
                .split(' ')
                .chain(env!("VKRGSS_RUBY_CLANG_CFLAGS").split(' '))
                .filter(|arg| !arg.is_empty()),
        )
        .args(flags)
        .env("TMPDIR", scratch);
    for include in includes {
        command.arg("-I").arg(include);
    }
    command
}

/// The job that compiles `source` into the directory `objects` with the flags `flags`.
pub(crate) fn job(
    scratch: &Path,
    objects: &Path,
    source: &Path,
    includes: &[&Path],
    flags: &[&str],
) -> Job {
    let stem = source.file_stem().expect("a file name");
    let object = objects.join(stem).with_extension("o");
    let mut command = instrumented_compiler(scratch, includes, flags);
    command.arg("-c").arg(source).arg("-o").arg(&object);
    Job { command, object }
}

/// Runs the jobs, several at a time, and returns the object files in the order of the jobs.
pub(crate) fn compile_jobs(jobs: Vec<Job>) -> Vec<PathBuf> {
    let objects = jobs.iter().map(|job| job.object.clone()).collect();
    let queue = Mutex::new(VecDeque::from(jobs));
    let workers = thread::available_parallelism().map_or(4, |count| count.get());
    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let Some(mut job) = queue.lock().unwrap().pop_front() else {
                        break;
                    };
                    run(&mut job.command);
                }
            });
        }
    });
    objects
}

/// Puts object files into a static library in `scratch`.
fn archive(scratch: &Path, name: &str, objects: &[PathBuf]) -> PathBuf {
    let library = scratch.join(name);
    run(Command::new("ar").arg("rcs").arg(&library).args(objects));
    library
}

/// The interpreter and the engine compiled with instrumentation, and what a C file that
/// uses them is compiled with.
pub(crate) struct InstrumentedLibraries {
    /// The interpreter with its `Zlib` extension.
    pub(crate) ruby: PathBuf,
    /// The regular expression engine.
    pub(crate) onig: PathBuf,
    /// The include paths of the interpreter's files.
    pub(crate) includes: Vec<PathBuf>,
}

impl InstrumentedLibraries {
    /// What follows the objects of an executable on the linker's command line.
    pub(crate) fn link_arguments(&self) -> Vec<PathBuf> {
        vec![
            self.ruby.clone(),
            self.onig.clone(),
            Path::new(env!("VKRGSS_RUBY_ZLIB_LIB_DIR")).join("libz.a"),
            PathBuf::from("-lcrypt"),
            PathBuf::from("-lm"),
        ]
    }
}

/// Compiles the files of the build script, with its include paths, into two static
/// libraries in `scratch`. Every file gets `flags`; the files of the engine get
/// `onig_flags` after them.
pub(crate) fn build_instrumented_libraries(
    scratch: &Path,
    flags: &[&str],
    onig_flags: &[&str],
) -> InstrumentedLibraries {
    let source = source_dir();
    let config = Path::new(env!("VKRGSS_RUBY_CONFIG_DIR"));
    let onig = Path::new(env!("VKRGSS_RUBY_ONIG_DIR"));
    let onig_config = config.join("oniguruma");
    let directory = |name: &str| {
        let dir = scratch.join(name);
        fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
        dir
    };
    let (core_dir, onig_dir) = (directory("core"), directory("onig"));

    let core_includes = [
        config,
        &source,
        Path::new(env!("VKRGSS_RUBY_GENERATED_DIR")),
        Path::new(env!("VKRGSS_RUBY_COMPAT_DIR")),
        onig,
    ];
    let zlib_includes = [
        config,
        &source,
        Path::new(env!("VKRGSS_RUBY_ZLIB_INCLUDE_DIR")),
    ];
    let zlib_flags: Vec<&str> = flags
        .iter()
        .copied()
        .chain(["-DHAVE_ZLIB_H", "-DOS_CODE=OS_UNIX"])
        .collect();
    let onig_flags: Vec<&str> = flags.iter().chain(onig_flags).copied().collect();

    let mut core_jobs: Vec<Job> = env!("VKRGSS_RUBY_CORE_SOURCES")
        .split(' ')
        .map(|name| source.join(format!("{name}.c")))
        .map(|file| job(scratch, &core_dir, &file, &core_includes, flags))
        .collect();
    core_jobs.push(job(
        scratch,
        &core_dir,
        &source.join("ext/zlib/zlib.c"),
        &zlib_includes,
        &zlib_flags,
    ));
    let onig_jobs: Vec<Job> = env!("VKRGSS_RUBY_ONIG_SOURCES")
        .split(' ')
        .map(|name| onig.join(format!("{name}.c")))
        .map(|file| {
            job(
                scratch,
                &onig_dir,
                &file,
                &[&onig_config, onig],
                &onig_flags,
            )
        })
        .collect();

    let core_count = core_jobs.len();
    let mut objects = compile_jobs(core_jobs.into_iter().chain(onig_jobs).collect());
    let onig_objects = objects.split_off(core_count);
    InstrumentedLibraries {
        ruby: archive(scratch, "libruby-instrumented.a", &objects),
        onig: archive(scratch, "libonig-instrumented.a", &onig_objects),
        includes: core_includes
            .iter()
            .map(|path| path.to_path_buf())
            .collect(),
    }
}

/// The number of tests in `test/ruby` of the release.
pub(crate) const CORE_TESTS: u64 = 625;

/// The number of tests in `test/zlib` of the release.
pub(crate) const ZLIB_TESTS: u64 = 45;

/// The number of checks `sample/test.rb` of the release makes.
pub(crate) const SAMPLE_CHECKS: usize = 912;

/// The output and exit status of one finished test process.
pub(crate) struct Finished {
    pub(crate) success: bool,
    pub(crate) output: String,
}

/// The totals `test/unit` prints at the end of a run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Totals {
    pub(crate) tests: u64,
    pub(crate) assertions: u64,
    pub(crate) failures: u64,
    pub(crate) errors: u64,
}

/// Finds the last `N tests, N assertions, N failures, N errors` line.
fn parse_totals(output: &str) -> Option<Totals> {
    output.lines().rev().find_map(|line| {
        let mut numbers = [0_u64; 4];
        let mut parts = line.split(", ");
        for (number, label) in numbers
            .iter_mut()
            .zip(["tests", "assertions", "failures", "errors"])
        {
            let (count, name) = parts.next()?.split_once(' ')?;
            if name != label {
                return None;
            }
            *number = count.parse().ok()?;
        }
        Some(Totals {
            tests: numbers[0],
            assertions: numbers[1],
            failures: numbers[2],
            errors: numbers[3],
        })
    })
}

/// The `test_*.rb` files of a suite directory, sorted.
fn test_files(suite: &str) -> Vec<String> {
    let dir = source_dir().join("test").join(suite);
    let mut files: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").file_name())
        .map(|name| name.into_string().expect("UTF-8 file name"))
        .filter(|name| name.starts_with("test_") && name.ends_with(".rb"))
        .collect();
    files.sort();
    files
}

/// An executable of Ruby's own command line and what its test processes are run with.
pub(crate) struct Suites {
    /// The scratch directory the executable was built in; the runs get directories below it.
    pub(crate) scratch: PathBuf,
    pub(crate) ruby: PathBuf,
    /// Environment variables added for every test process.
    pub(crate) env: Vec<(&'static str, String)>,
    /// How long one test process may run.
    pub(crate) timeout: Duration,
}

impl Suites {
    /// Runs the executable in a working directory of its own, the way `make test-all` sets it
    /// up: `./ruby` and `./miniruby` in the current directory, `RUBY` pointing at the former, a
    /// scratch `HOME` and `TMPDIR`, and a load path that ends at the vendored `lib` and `ext`.
    /// The search path is that directory, `/usr/bin` and `/bin`.
    ///
    /// SIGINT gets its default disposition whatever this process inherited: the suites start
    /// interpreters that send it to themselves, and an ignored disposition would be inherited
    /// by all of them.
    pub(crate) fn run_ruby(&self, name: &str, args: &[&str]) -> Finished {
        let source = source_dir();
        let dir = self.scratch.join("run").join(name);
        let home = dir.join("home");
        let tmp = dir.join("tmp");
        fs::create_dir_all(&home).expect("scratch home");
        fs::create_dir_all(&tmp).expect("scratch tmp");
        let ruby = dir.join("ruby");
        symlink(&self.ruby, &ruby).expect("link to the executable");
        symlink(&self.ruby, dir.join("miniruby")).expect("link to the executable");

        // Not the caller's `PATH`: Ruby warns about a directory in it that everybody may
        // write whenever it starts a program, and the suites compare error streams.
        let search = [dir.as_path(), Path::new("/usr/bin"), Path::new("/bin")];
        let log = fs::File::create(dir.join("output.log")).expect("log file");
        let mut command = Command::new(&ruby);
        // SAFETY: the closure runs in the child between `fork` and `exec` and only calls
        // `signal`, which is async-signal-safe.
        unsafe {
            command.pre_exec(|| {
                if libc::signal(libc::SIGINT, libc::SIG_DFL) == libc::SIG_ERR {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command
            .args(args)
            .current_dir(&dir)
            .env("HOME", &home)
            .env("TMPDIR", &tmp)
            .env("RUBY", &ruby)
            .env(
                "PATH",
                std::env::join_paths(search).expect("a joinable PATH"),
            )
            .env(
                "RUBYLIB",
                format!(
                    "{}:{}:-",
                    source.join("lib").display(),
                    source.join("ext").display()
                ),
            )
            .env_remove("RUBYOPT")
            .envs(self.env.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::null())
            .stdout(log.try_clone().expect("second handle to the log"))
            .stderr(log)
            .spawn()
            .unwrap_or_else(|error| panic!("could not start {}: {error}", ruby.display()));

        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().expect("wait for the test process") {
                break Some(status);
            }
            if started.elapsed() > self.timeout {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            thread::sleep(Duration::from_millis(5));
        };
        let mut output =
            String::from_utf8_lossy(&fs::read(dir.join("output.log")).expect("the log"))
                .into_owned();
        if status.is_none() {
            output.push_str("\n(killed: the process ran into the time limit)\n");
        }
        Finished {
            success: status.is_some_and(|status| status.success()),
            output,
        }
    }

    /// Runs one file of a `test/unit` suite through `test/runner.rb`.
    pub(crate) fn run_unit_file(&self, suite: &str, file: &str) -> Result<Totals, String> {
        self.run_unit_file_without(suite, file, &[])
    }

    /// As [`run_unit_file`](Self::run_unit_file), leaving out the test methods named.
    fn run_unit_file_without(
        &self,
        suite: &str,
        file: &str,
        methods: &[&str],
    ) -> Result<Totals, String> {
        let source = source_dir();
        let runner = source.join("test/runner.rb");
        let basedir = format!("--basedir={}", source.join("test").display());
        let target = format!("{suite}/{file}");
        let name = format!("{suite}-{}", file.trim_end_matches(".rb"));
        let finished = self.run_ruby(
            &name,
            &[
                "-rpurelib.rb",
                runner.to_str().expect("UTF-8 path"),
                &basedir,
                "--runner=console",
                // Every method but those named. Without one the pattern matches every name.
                &format!("--name=/\\A(?!(?:{})\\z)/", methods.join("|")),
                &target,
            ],
        );
        match parse_totals(&finished.output) {
            Some(totals) if finished.success && totals.failures == 0 && totals.errors == 0 => {
                Ok(totals)
            }
            _ => Err(format!("--- {target}\n{}", finished.output)),
        }
    }

    /// Runs every file of a suite as its own process, several at a time, and sums the totals.
    pub(crate) fn run_unit_suite(&self, suite: &str) -> Totals {
        self.run_unit_suite_without(suite, &[])
    }

    /// As [`run_unit_suite`](Self::run_unit_suite), leaving out what `skip` names: a file as
    /// `<suite>/<file>`, one test method as `<suite>/<file>#<method>`.
    pub(crate) fn run_unit_suite_without(&self, suite: &str, skip: &[String]) -> Totals {
        let skipped = |file: &str| -> Vec<&str> {
            let prefix = format!("{suite}/{file}");
            skip.iter()
                .filter_map(|entry| entry.strip_prefix(&prefix))
                .filter_map(|rest| rest.strip_prefix('#').or(rest.is_empty().then_some("")))
                .collect()
        };
        let mut files = test_files(suite);
        files.retain(|file| !skipped(file).contains(&""));
        assert!(!files.is_empty(), "no test files in test/{suite}");
        let queue = Mutex::new(VecDeque::from(files));
        let results: Mutex<Vec<Result<Totals, String>>> = Mutex::new(Vec::new());
        let workers = thread::available_parallelism().map_or(4, |count| count.get().min(8));
        thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| {
                    loop {
                        let Some(file) = queue.lock().unwrap().pop_front() else {
                            break;
                        };
                        let result = self.run_unit_file_without(suite, &file, &skipped(&file));
                        results.lock().unwrap().push(result);
                    }
                });
            }
        });

        let mut sum = Totals::default();
        let mut problems = Vec::new();
        for result in results.into_inner().unwrap() {
            match result {
                Ok(totals) => {
                    sum.tests += totals.tests;
                    sum.assertions += totals.assertions;
                    sum.failures += totals.failures;
                    sum.errors += totals.errors;
                }
                Err(problem) => problems.push(problem),
            }
        }
        assert!(
            problems.is_empty(),
            "{} file(s) of test/{suite} did not pass:\n{}",
            problems.len(),
            problems.join("\n")
        );
        sum
    }

    /// Runs `sample/test.rb` of the release and checks the number of checks it passes.
    pub(crate) fn run_sample(&self) {
        let source = source_dir();
        let lib = format!("-I{}", source.join("lib").display());
        let script = source.join("sample/test.rb");
        let finished = self.run_ruby("sample", &[&lib, script.to_str().expect("UTF-8 path")]);
        let lines = |prefix: &str| -> usize {
            finished
                .output
                .lines()
                .filter(|line| line.starts_with(prefix))
                .count()
        };
        let (passed, failed, ended) = (lines("ok "), lines("not ok"), lines("end of test"));
        assert!(
            finished.success && failed == 0 && ended == 1 && passed == SAMPLE_CHECKS,
            "sample/test.rb: {passed} ok, {failed} not ok, {ended} `end of test`, success {}\n{}",
            finished.success,
            finished.output
        );
    }
}

/// Evaluates Ruby source with the raw C API and returns `inspect` of the result, or of the
/// exception as `Err`. For tests that must not depend on the crate's own evaluation helpers.
///
/// Call it only inside a closure given to `Interpreter::run`.
pub(crate) fn eval_inspect(source: &str) -> Result<String, String> {
    use std::ffi::{CString, c_int};
    use vkrgss_ruby_sys::ffi;

    let source = CString::new(source).expect("no NUL in the source");
    // SAFETY: the caller is on the interpreter thread. Every call that can raise is protected
    // or is `inspect`/`to_s` of a value, which the tests only use on well-behaved objects.
    unsafe {
        let mut state: c_int = 0;
        let mut value = ffi::rb_eval_string_protect(source.as_ptr(), &mut state);
        let raised = state != 0;
        if raised {
            value = ffi::ruby_errinfo;
            ffi::ruby_errinfo = ffi::Qnil;
        }
        let text = ffi::rb_inspect(value);
        let string = &*ffi::rstring(text);
        let bytes = std::slice::from_raw_parts(string.ptr.cast::<u8>(), string.len as usize);
        let text = String::from_utf8_lossy(bytes).into_owned();
        if raised { Err(text) } else { Ok(text) }
    }
}

/// A script that allocates 100 objects and returns how many it made.
pub(crate) const HUNDRED_OBJECTS: &str = "made = 0; 100.times { Object.new; made += 1 }; made";

/// Evaluates `source` as [`eval_inspect`] does and returns with its result the number of
/// collections that ran meanwhile.
///
/// Call it only inside a closure given to `Interpreter::run`.
pub(crate) fn collections_during(source: &str) -> (u64, Result<String, String>) {
    use vkrgss_ruby_sys::ffi;

    // SAFETY: the caller is on the interpreter thread, the only one that writes the counter.
    let before = unsafe { ffi::ruby_vkxp_gc_count };
    let result = eval_inspect(source);
    // SAFETY: as above.
    let after = unsafe { ffi::ruby_vkxp_gc_count };
    (after - before, result)
}

/// The three ways a script recurses without end: a method that calls itself, a block that
/// yields to itself, and a string that evaluates itself.
pub(crate) const UNBOUNDED_RECURSIONS: [&str; 3] = [
    "def vkrgss_rec_method; vkrgss_rec_method; end; vkrgss_rec_method",
    "def vkrgss_rec_block; [1].each { vkrgss_rec_block }; end; vkrgss_rec_block",
    "$vkrgss_rec_source = 'eval($vkrgss_rec_source)'; eval($vkrgss_rec_source)",
];

/// Runs each unbounded recursion several times and checks that it raises `SystemStackError`
/// and that the interpreter still computes afterwards.
pub(crate) fn assert_recursion_is_bounded(interpreter: &vkrgss_ruby_sys::Interpreter) {
    for source in UNBOUNDED_RECURSIONS {
        for round in 0..3 {
            let outcome = interpreter.run(move |_| eval_inspect(source));
            // `rb_eval_string` prefixes the message with a position.
            let error = outcome.expect_err("unbounded recursion cannot return");
            assert!(
                error.starts_with("#<SystemStackError: ")
                    && error.ends_with("stack level too deep>"),
                "round {round} of {source:?} raised {error}"
            );
            let after = interpreter.run(|_| eval_inspect("1 + 1"));
            assert_eq!(
                after,
                Ok("2".to_owned()),
                "after round {round} of {source:?}"
            );
        }
    }
}

/// The current disposition of each signal Ruby takes, as (signal, handler, flags, mask).
pub(crate) fn dispositions() -> Vec<(libc::c_int, usize, libc::c_int, Vec<libc::c_int>)> {
    vkrgss_ruby_sys::SIGNALS_TAKEN_BY_RUBY
        .iter()
        .map(|&signal| {
            // SAFETY: a null new action only queries, and a zeroed `sigaction` is a valid
            // buffer for the old one. `sigismember` reads the mask that was just filled in.
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                assert_eq!(libc::sigaction(signal, std::ptr::null(), &mut action), 0);
                let mask = (1..=64)
                    .filter(|&other| libc::sigismember(&action.sa_mask, other) == 1)
                    .collect();
                (signal, action.sa_sigaction, action.sa_flags, mask)
            }
        })
        .collect()
}
