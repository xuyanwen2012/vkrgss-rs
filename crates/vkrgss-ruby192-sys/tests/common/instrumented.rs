//! The instrumented builds of the sanitized run and of the fuzz targets: the files of the
//! build script compiled again, with clang and more flags, in a scratch directory of their
//! own.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::thread;

use super::{TEST_EXTENSIONS, run, source_dir, write_init_ext_with};

/// The compiler of the instrumented builds. A distribution may package the sanitizer
/// runtimes of clang only, as Fedora does with `compiler-rt`, so this is not necessarily the
/// compiler the build script used.
pub(crate) const INSTRUMENTED_COMPILER: &str = "clang";

/// One C file to compile: the command and the object file it writes.
pub(crate) struct Job {
    command: Command,
    object: PathBuf,
}

/// The instrumented compiler with the flags, defines and include paths of the build script
/// but those of `_FORTIFY_SOURCE`, which conflicts with the interceptors of
/// AddressSanitizer, with the flags clang needs for the old sources, and with `flags`.
pub(crate) fn instrumented_compiler(scratch: &Path, flags: &[&str]) -> Command {
    let fortify: Vec<&str> = env!("VKRGSS_RUBY192_FORTIFY_CFLAGS").split(' ').collect();
    let mut command = Command::new(INSTRUMENTED_COMPILER);
    command
        .args(
            env!("VKRGSS_RUBY192_CFLAGS")
                .split(' ')
                .chain(env!("VKRGSS_RUBY192_CLANG_CFLAGS").split(' '))
                .filter(|arg| !arg.is_empty() && !fortify.contains(arg)),
        )
        .args(flags)
        .env("TMPDIR", scratch);
    command
}

/// The job that compiles `source` into the directory `objects` with the flags `flags`.
pub(crate) fn job(scratch: &Path, objects: &Path, source: &Path, flags: &[&str]) -> Job {
    let stem = source.file_stem().expect("a file name");
    let object = objects.join(stem).with_extension("o");
    let mut command = instrumented_compiler(scratch, flags);
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

fn directory(scratch: &Path, name: &str) -> PathBuf {
    let dir = scratch.join(name);
    fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    dir
}

fn refs(flags: &[String]) -> Vec<&str> {
    flags.iter().map(String::as_str).collect()
}

/// Puts object files into a static library in `scratch`.
fn archive(scratch: &Path, name: &str, objects: &[PathBuf]) -> PathBuf {
    let library = scratch.join(name);
    run(Command::new("ar").arg("rcs").arg(&library).args(objects));
    library
}

/// The interpreter with its encodings and `Zlib`, compiled with instrumentation.
pub(crate) struct InstrumentedLibraries {
    pub(crate) archives: Vec<PathBuf>,
}

impl InstrumentedLibraries {
    /// What follows the objects of an executable on the linker's command line, as in
    /// `super::link_arguments`. CORE-MATH is linked as the build script built it, without
    /// instrumentation.
    pub(crate) fn link_arguments(&self) -> Vec<PathBuf> {
        let zlib_libs = Path::new(env!("VKRGSS_RUBY192_ZLIB_LIB_DIR"));
        let mut arguments = vec![PathBuf::from("-Wl,--start-group")];
        arguments.extend(self.archives.iter().cloned());
        arguments.extend([
            PathBuf::from("-Wl,--end-group"),
            Path::new(env!("VKRGSS_RUBY192_LIB_DIR")).join("libvkrgss-ruby192-core-math.a"),
            zlib_libs.join("libz.a"),
            PathBuf::from("-lpthread"),
            PathBuf::from("-lrt"),
            PathBuf::from("-lcrypt"),
            PathBuf::from("-lm"),
        ]);
        arguments
    }
}

/// Compiles the files of the build script into three static libraries in `scratch`, as the
/// build script groups them, every file with `flags`; the files of the regular expression
/// engine, which is part of the core, get `engine_flags` after them.
pub(crate) fn build_instrumented_libraries(
    scratch: &Path,
    flags: &[&str],
    engine_flags: &[&str],
) -> InstrumentedLibraries {
    let source = source_dir();
    let (core_dir, enc_dir, zlib_dir) = (
        directory(scratch, "core"),
        directory(scratch, "enc"),
        directory(scratch, "zlib"),
    );
    let with = |more: &[&str]| -> Vec<String> {
        flags
            .iter()
            .chain(more)
            .map(|flag| (*flag).to_owned())
            .collect()
    };
    let core_flags = with(&[]);
    let engine_flags = with(engine_flags);
    let encoding_flags = with(&["-DONIG_ENC_REGISTER=rb_enc_register"]);
    let zlib_include = format!("-I{}", env!("VKRGSS_RUBY192_ZLIB_INCLUDE_DIR"));
    let zlib_flags = with(&[&zlib_include, "-DHAVE_ZLIB_H", "-DOS_CODE=OS_UNIX"]);

    let mut core_jobs: Vec<Job> = env!("VKRGSS_RUBY192_CORE_SOURCES")
        .split(' ')
        .map(|name| {
            let flags = if name.starts_with("reg") {
                refs(&engine_flags)
            } else {
                refs(&core_flags)
            };
            let file = source.join(format!("{name}.c"));
            job(scratch, &core_dir, &file, &flags)
        })
        .collect();
    core_jobs.push(job(
        scratch,
        &core_dir,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("encodings.c"),
        &refs(&core_flags),
    ));
    let encoding_jobs: Vec<Job> = env!("VKRGSS_RUBY192_ENCODING_SOURCES")
        .split(' ')
        .map(|name| source.join(format!("{name}.c")))
        .map(|file| job(scratch, &enc_dir, &file, &refs(&encoding_flags)))
        .collect();
    let zlib_job = job(
        scratch,
        &zlib_dir,
        &source.join("ext/zlib/zlib.c"),
        &refs(&zlib_flags),
    );

    let (core_count, encoding_count) = (core_jobs.len(), encoding_jobs.len());
    let mut objects = compile_jobs(
        core_jobs
            .into_iter()
            .chain(encoding_jobs)
            .chain([zlib_job])
            .collect(),
    );
    let zlib_objects = objects.split_off(core_count + encoding_count);
    let encoding_objects = objects.split_off(core_count);
    InstrumentedLibraries {
        archives: vec![
            archive(scratch, "libruby192-zlib-instrumented.a", &zlib_objects),
            archive(scratch, "libruby192-instrumented.a", &objects),
            archive(scratch, "libruby192-enc-instrumented.a", &encoding_objects),
        ],
    }
}

/// Builds Ruby's own command-line executable from instrumented libraries, as
/// `super::build_ruby_executable` builds it from the archives of the build script: `main.c`,
/// the extensions the tests load and an `Init_ext` with `definitions` and `statements` added
/// (see `super::write_init_ext_with`), all compiled with `flags`.
pub(crate) fn build_instrumented_executable(
    scratch: &Path,
    flags: &[&str],
    engine_flags: &[&str],
    definitions: &str,
    statements: &str,
) -> PathBuf {
    let source = source_dir();
    let libraries = build_instrumented_libraries(scratch, flags, engine_flags);
    let top_dir = directory(scratch, "top");
    let init_ext = write_init_ext_with(scratch, definitions, statements);

    let mut jobs = vec![
        job(scratch, &top_dir, &source.join("main.c"), flags),
        job(scratch, &top_dir, &init_ext, flags),
    ];
    for (name, defines) in TEST_EXTENSIONS {
        let flags: Vec<&str> = flags.iter().chain(defines.iter()).copied().collect();
        let file = source.join(format!("ext/{name}/{name}.c"));
        jobs.push(job(scratch, &top_dir, &file, &flags));
    }
    let objects = compile_jobs(jobs);

    // As in the uninstrumented executable: the objects first, so that their `Init_ext` wins.
    let executable = scratch.join("ruby");
    run(instrumented_compiler(scratch, flags)
        .arg("-rdynamic")
        .args(&objects)
        .args(libraries.link_arguments())
        .arg("-o")
        .arg(&executable));
    executable
}
