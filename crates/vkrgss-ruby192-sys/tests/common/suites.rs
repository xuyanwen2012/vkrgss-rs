//! The harness of Ruby's own suites: the list of what is left out, and the test processes.

use std::collections::VecDeque;
use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use super::{build_ruby_executable, scratch_dir, source_dir};

/// The test libraries of this crate: `rbconfig.rb`, which the release generates and the
/// suites read, and the two files loaded in front of a suite.
pub(crate) fn testlib_dir() -> PathBuf {
    super::manifest_dir().join("tests/testlib")
}

/// One entry of `tests/upstream.allow`: a file or a test method that is left out, with the
/// reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Allowed {
    /// `ruby/test_x.rb`, `ruby/test_x.rb#test_y` or `bootstraptest/test_x.rb`.
    pub(crate) test: String,
    pub(crate) reason: String,
}

/// The committed list of what the suites leave out.
pub(crate) const ALLOW_LIST: &str = include_str!("../upstream.allow");

/// The committed list of what the suites leave out on Android as well.
pub(crate) const ANDROID_ALLOW_LIST: &str = include_str!("../upstream.android.allow");

/// Parses a list in the format of `xtask/src/stress.skip` without its step: entries separated
/// by blank lines, each made of `#` lines that give the reason and of lines that each name a
/// file or a test method. A block of comment lines alone is not an entry.
pub(crate) fn parse_allow_list(text: &str) -> Vec<Allowed> {
    let mut allowed = Vec::new();
    for block in text.split("\n\n") {
        let mut reason = Vec::new();
        let mut tests = Vec::new();
        for line in block.lines().map(str::trim) {
            if let Some(comment) = line.strip_prefix('#') {
                reason.push(comment.trim());
            } else if !line.is_empty() {
                tests.push(line);
            }
        }
        let reason = reason.join(" ").trim().to_owned();
        for test in tests {
            allowed.push(Allowed {
                test: test.to_owned(),
                reason: reason.clone(),
            });
        }
    }
    allowed
}

/// What the suites leave out on macOS besides the committed lists, each with its reason. On
/// macOS `test_segv_test` finds the lines of the C backtrace in the format of
/// `backtrace_symbols` of macOS (`0   ruby   0x... rb_vm_bugreport + 136`), which its pattern,
/// written for glibc, does not match; only the text of a crash report differs.
const LEFT_OUT_ON_MACOS: &[&str] = &["ruby/test_rubyoptions.rb#test_segv_test"];

/// The names of the committed lists that apply to the target, for the functions that take
/// what to leave out, and those of [`LEFT_OUT_ON_MACOS`] on macOS.
pub(crate) fn allowed_tests() -> Vec<String> {
    let mut lists = vec![ALLOW_LIST];
    if cfg!(target_os = "android") {
        lists.push(ANDROID_ALLOW_LIST);
    }
    let mut tests: Vec<String> = lists
        .into_iter()
        .flat_map(parse_allow_list)
        .map(|entry| entry.test)
        .collect();
    if cfg!(target_os = "macos") {
        tests.extend(LEFT_OUT_ON_MACOS.iter().map(|&test| test.to_owned()));
    }
    tests
}

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
    pub(crate) skips: u64,
}

/// Finds the last `N tests, N assertions, N failures, N errors, N skips` line.
fn parse_totals(output: &str) -> Option<Totals> {
    output.lines().rev().find_map(|line| {
        let mut numbers = [0_u64; 5];
        let mut parts = line.trim().split(", ");
        for (number, label) in
            numbers
                .iter_mut()
                .zip(["tests", "assertions", "failures", "errors", "skips"])
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
            skips: numbers[4],
        })
    })
}

/// The `test_*.rb` files of a directory of the vendored tree, sorted.
pub(crate) fn test_files(directory: &str) -> Vec<String> {
    let dir = source_dir().join(directory);
    let mut files: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").file_name())
        .map(|name| name.into_string().expect("UTF-8 file name"))
        .filter(|name| name.starts_with("test_") && name.ends_with(".rb"))
        .collect();
    files.sort();
    files
}

/// The files of `test/ruby` that `skip` does not leave out whole, as `ruby/test_x.rb`.
pub(crate) fn selected_core_files(skip: &[String]) -> Vec<String> {
    test_files("test/ruby")
        .into_iter()
        .map(|file| format!("ruby/{file}"))
        .filter(|file| !skip.contains(file))
        .collect()
}

/// An executable of Ruby's own command line and what its test processes are run with.
pub(crate) struct Suites {
    /// The scratch directory the executable was built in; the runs get directories below it.
    pub(crate) scratch: PathBuf,
    pub(crate) ruby: PathBuf,
    /// How long one test process may run.
    pub(crate) timeout: Duration,
    /// Whether a file of `test/ruby` is loaded and run with `GC.stress = true`.
    pub(crate) stress: bool,
    /// How many test processes run at a time.
    pub(crate) workers: usize,
    /// Environment variables added for every test process.
    pub(crate) env: Vec<(&'static str, String)>,
}

impl Suites {
    /// Builds the executable in a fresh scratch directory of this name.
    pub(crate) fn build(name: &str, timeout: Duration, stress: bool) -> Self {
        let scratch = scratch_dir(name);
        let ruby = build_ruby_executable(&scratch);
        Self::with_executable(scratch, ruby, timeout, stress)
    }

    /// The suites of an executable that was built in `scratch`.
    pub(crate) fn with_executable(
        scratch: PathBuf,
        ruby: PathBuf,
        timeout: Duration,
        stress: bool,
    ) -> Self {
        let workers = thread::available_parallelism().map_or(4, |count| count.get().min(10));
        Self {
            scratch,
            ruby,
            timeout,
            stress,
            workers,
            env: Vec::new(),
        }
    }

    /// Runs the executable in a working directory of its own, the way `make test-all` sets it
    /// up: `./ruby` and `./miniruby` in the current directory, `RUBY` pointing at the former, a
    /// scratch `HOME` and `TMPDIR`, a UTF-8 locale, and a load path of the test libraries of
    /// this crate and the vendored `lib`. The search path is that directory, `/usr/bin` and
    /// `/bin`, not the caller's.
    ///
    /// SIGINT and SIGQUIT get their default dispositions whatever this process inherited: the
    /// suites start interpreters that are to be ended by them, and an ignored disposition,
    /// which a shell gives both signals in a command it starts in the background, would be
    /// inherited by all of them.
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

        let search = [dir.as_path(), Path::new("/usr/bin"), Path::new("/bin")];
        let log = fs::File::create(dir.join("output.log")).expect("log file");
        let mut command = Command::new(&ruby);
        // SAFETY: the closure runs in the child between `fork` and `exec` and only calls
        // `signal` and `setpgid`, which are async-signal-safe.
        unsafe {
            command.pre_exec(|| {
                for signal in [libc::SIGINT, libc::SIGQUIT] {
                    if libc::signal(signal, libc::SIG_DFL) == libc::SIG_ERR {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                // A group of its own, so that a run that hits the time limit can be ended
                // with the interpreters it started.
                if libc::setpgid(0, 0) != 0 {
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
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env(
                "PATH",
                std::env::join_paths(search).expect("a joinable PATH"),
            )
            .env(
                "RUBYLIB",
                format!(
                    "{}:{}",
                    testlib_dir().display(),
                    source.join("lib").display()
                ),
            )
            .env_remove("RUBYOPT")
            .env_remove("TZ")
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
                // SAFETY: the child made itself the leader of a process group of its own,
                // whose number is its process id; the signal reaches that group only.
                unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
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

    /// Runs one file of `test/ruby`, given as `ruby/test_x.rb`, through `test/runner.rb`,
    /// leaving out the test methods that `skip` names as `ruby/test_x.rb#test_y`. A file
    /// that reports no test has not passed.
    pub(crate) fn run_unit_file(&self, file: &str, skip: &[String]) -> Result<Totals, String> {
        let source = source_dir();
        let testlib = testlib_dir();
        let methods: Vec<&str> = skip
            .iter()
            .filter_map(|entry| entry.strip_prefix(file)?.strip_prefix('#'))
            .collect();
        let name = file.trim_end_matches(".rb").replace('/', "-");
        let mut args = vec![format!("-r{}", testlib.join("test_prelude.rb").display())];
        if self.stress {
            args.push(format!("-r{}", testlib.join("gc_stress.rb").display()));
        }
        args.push(source.join("test/runner.rb").display().to_string());
        // Every method but those named. Without one the pattern matches every name.
        args.push(format!("--name=/\\A(?!(?:{})\\z)/", methods.join("|")));
        args.push(file.to_owned());
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let finished = self.run_ruby(&name, &args);
        match parse_totals(&finished.output) {
            Some(totals)
                if finished.success
                    && totals.tests > 0
                    && totals.failures == 0
                    && totals.errors == 0 =>
            {
                Ok(totals)
            }
            _ => Err(format!("--- {file}\n{}", finished.output)),
        }
    }

    /// Runs the files of `test/ruby` given, each as its own process, several at a time, and
    /// sums the totals. Panics with the output of every file that did not pass.
    pub(crate) fn run_unit_files(&self, files: &[String], skip: &[String]) -> Totals {
        assert!(!files.is_empty(), "no test files to run");
        let queue = Mutex::new(VecDeque::from(files.to_vec()));
        let results: Mutex<Vec<Result<Totals, String>>> = Mutex::new(Vec::new());
        thread::scope(|scope| {
            for _ in 0..self.workers {
                scope.spawn(|| {
                    loop {
                        let Some(file) = queue.lock().unwrap().pop_front() else {
                            break;
                        };
                        let result = self.run_unit_file(&file, skip);
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
                    sum.skips += totals.skips;
                }
                Err(problem) => problems.push(problem),
            }
        }
        assert!(
            problems.is_empty(),
            "{} file(s) of test/ruby did not pass:\n{}",
            problems.len(),
            problems.join("\n")
        );
        sum
    }

    /// Runs one file of `bootstraptest` with the release's runner, the executable being both
    /// the runner and the interpreter under test. Returns the number of assertions it passed.
    pub(crate) fn run_bootstrap_file(&self, file: &str) -> Result<u64, String> {
        let source = source_dir().join("bootstraptest");
        let name = format!("bootstraptest-{}", file.trim_end_matches(".rb"));
        let finished = self.run_ruby(
            &name,
            &[
                source.join("runner.rb").to_str().expect("UTF-8 path"),
                &format!("--ruby={}", self.ruby.display()),
                "-q",
                source.join(file).to_str().expect("UTF-8 path"),
            ],
        );
        let passed = finished.output.lines().find_map(|line| {
            line.strip_prefix("PASS all ")?
                .strip_suffix(" tests")?
                .parse::<u64>()
                .ok()
        });
        match passed {
            Some(count) if finished.success && count > 0 => Ok(count),
            _ => Err(format!("--- bootstraptest/{file}\n{}", finished.output)),
        }
    }
}
