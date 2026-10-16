//! Support shared by the integration tests: the C compiler exactly as the build script used
//! it, scratch directories below cargo's target directory, and Ruby's own test executable.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The vendored Ruby tree.
pub(crate) fn source_dir() -> PathBuf {
    PathBuf::from(env!("VKRGSS_RUBY192_SOURCE_DIR"))
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

/// The compiler with the flags, defines and include paths the interpreter was built with,
/// and a temporary directory that is surely writable.
pub(crate) fn compiler(scratch: &Path) -> Command {
    let mut command = Command::new(env!("VKRGSS_RUBY192_CC"));
    command
        .args(
            env!("VKRGSS_RUBY192_CFLAGS")
                .split(' ')
                .filter(|arg| !arg.is_empty()),
        )
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

/// Compiles one C file to an object file in `scratch`, with more compiler arguments, and
/// returns the object's path.
pub(crate) fn compile_object_with(scratch: &Path, source: &Path, args: &[&str]) -> PathBuf {
    let stem = source.file_stem().expect("a file name");
    let object = scratch.join(stem).with_extension("o");
    run(compiler(scratch)
        .args(args)
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(&object));
    object
}

/// Compiles one C file to an object file in `scratch` and returns the object's path.
pub(crate) fn compile_object(scratch: &Path, source: &Path) -> PathBuf {
    compile_object_with(scratch, source, &[])
}

/// What follows the objects of an executable on the linker's command line: the archives of
/// the build script, the zlib of `libz-sys` and the libraries of the system. The archives
/// refer to each other, hence the group.
pub(crate) fn link_arguments() -> Vec<PathBuf> {
    let ruby_libs = Path::new(env!("VKRGSS_RUBY192_LIB_DIR"));
    let zlib_libs = Path::new(env!("VKRGSS_RUBY192_ZLIB_LIB_DIR"));
    vec![
        PathBuf::from("-Wl,--start-group"),
        ruby_libs.join("libvkrgss-ruby192-zlib.a"),
        ruby_libs.join("libvkrgss-ruby192-static.a"),
        ruby_libs.join("libvkrgss-ruby192-enc.a"),
        PathBuf::from("-Wl,--end-group"),
        ruby_libs.join("libvkrgss-ruby192-core-math.a"),
        zlib_libs.join("libz.a"),
        PathBuf::from("-lpthread"),
        PathBuf::from("-lrt"),
        PathBuf::from("-lcrypt"),
        PathBuf::from("-lm"),
    ]
}

/// The extensions that only Ruby's test executable links, with the defines of their own
/// `extconf.rb` on this target.
pub(crate) const TEST_EXTENSIONS: &[(&str, &[&str])] = &[
    ("stringio", &[]),
    ("fcntl", &[]),
    (
        "etc",
        &[
            "-DHAVE_GETLOGIN",
            "-DHAVE_GETPWENT",
            "-DHAVE_GETGRENT",
            "-DHAVE_ST_PW_GECOS",
            "-DHAVE_ST_PW_PASSWD",
            "-DSYSCONFDIR=\"/etc\"",
        ],
    ),
    ("continuation", &[]),
    ("fiber", &[]),
];

/// Writes the `Init_ext` of Ruby's test executable into `scratch` and returns its path: it
/// registers the encodings and transcoders that the crate registers at start-up, `Zlib`, and
/// the extensions that Ruby's tests load.
///
/// It also gives the three encodings of the core their constants (`Encoding::UTF_8` and so
/// on), which the release declares in `Init_encdb` and the suites use throughout. The
/// library declares no name of the release, so these constants are undefined in it.
fn write_init_ext(scratch: &Path) -> PathBuf {
    write_init_ext_with(scratch, "", "")
}

/// As [`write_init_ext`], with more C definitions in front of `Init_ext` and more statements
/// at its end.
pub(crate) fn write_init_ext_with(scratch: &Path, definitions: &str, more: &str) -> PathBuf {
    let mut declarations = String::new();
    let mut statements = String::new();
    for (name, _) in TEST_EXTENSIONS {
        declarations.push_str(&format!("void Init_{name}(void);\n"));
        statements.push_str(&format!("    ruby_init_ext(\"{name}.so\", Init_{name});\n"));
    }
    let init_ext = scratch.join("extinit.c");
    fs::write(
        &init_ext,
        format!(
            "#include \"ruby.h\"\n\
             void ruby_init_ext(const char *name, void (*init)(void));\n\
             void Init_utf_16le(void);\n\
             void Init_japanese_sjis(void);\n\
             void Init_single_byte(void);\n\
             void Init_utf_16_32(void);\n\
             void Init_zlib(void);\n\
             void rb_encdb_declare(const char *name);\n\
             {declarations}\
             __attribute__((visibility(\"default\"))) int vkrgss_export_probe = 1;\n\
             {definitions}\
             void Init_ext(void)\n\
             {{\n\
             \x20   rb_encdb_declare(\"ASCII-8BIT\");\n\
             \x20   rb_encdb_declare(\"UTF-8\");\n\
             \x20   rb_encdb_declare(\"US-ASCII\");\n\
             \x20   Init_utf_16le();\n\
             \x20   Init_japanese_sjis();\n\
             \x20   Init_single_byte();\n\
             \x20   Init_utf_16_32();\n\
             \x20   ruby_init_ext(\"zlib.so\", Init_zlib);\n\
             {statements}\
             {more}\
             }}\n"
        ),
    )
    .expect("extinit.c is writable");
    init_ext
}

/// Builds Ruby's own command-line executable from the archives the build script produced:
/// `main.c`, the extensions that Ruby's tests load, and an `Init_ext` that registers them,
/// `Zlib` and the encodings the crate registers at start-up. It runs `ruby_options`, and with
/// it the prelude that the library never runs. `-rdynamic` exports every symbol of default
/// visibility, as a real `ruby` does, which is also what makes the executable a probe for
/// leaked symbols: `vkrgss_export_probe` is the one symbol given default visibility on purpose,
/// to show that.
pub(crate) fn build_ruby_executable(scratch: &Path) -> PathBuf {
    let source = source_dir();
    let init_ext = write_init_ext(scratch);

    let mut objects = vec![
        compile_object(scratch, &source.join("main.c")),
        compile_object(scratch, &init_ext),
    ];
    for (name, defines) in TEST_EXTENSIONS {
        objects.push(compile_object_with(
            scratch,
            &source.join(format!("ext/{name}/{name}.c")),
            defines,
        ));
    }

    // The objects come first so that their `Init_ext` wins over the empty one of `dmyext.o`.
    let executable = scratch.join("ruby");
    run(compiler(scratch)
        .arg("-rdynamic")
        .args(&objects)
        .args(link_arguments())
        .arg("-o")
        .arg(&executable));
    executable
}

/// Evaluates Ruby source through the crate's helpers and returns `inspect` of the result, or
/// the exception as `Class: message`.
pub(crate) fn eval_inspect(
    ruby: &vkrgss_ruby192_sys::Ruby,
    source: &str,
) -> Result<String, String> {
    let value = ruby
        .eval_protected(source, "test", 1)
        .map_err(|error| format!("{}: {}", error.class_name, error.message))?;
    // SAFETY: `value` was just returned by the interpreter and is held in a local.
    let text = unsafe { ruby.inspect(value) };
    text.map_err(|_| "inspect raised".to_owned())
}

/// A script that allocates 100 objects and returns how many collections ran meanwhile.
pub(crate) const COLLECTIONS_FOR_HUNDRED_OBJECTS: &str =
    "before = GC.count; made = 0; 100.times { Object.new; made += 1 }; [made, GC.count - before]";

/// The two integers of an inspected pair, such as `[100, 3]`.
pub(crate) fn parse_pair(text: &str) -> (u64, u64) {
    let (first, second) = text
        .trim_matches(|character| character == '[' || character == ']')
        .split_once(", ")
        .unwrap_or_else(|| panic!("not a pair: {text}"));
    (
        first.parse().expect("an integer"),
        second.parse().expect("an integer"),
    )
}

/// The four ways a script recurses without end: a method that calls itself, a block that
/// yields to itself, a lambda that calls itself, and a string that evaluates itself.
pub(crate) const UNBOUNDED_RECURSIONS: [&str; 4] = [
    "def vkrgss_rec_method; vkrgss_rec_method; end; vkrgss_rec_method",
    "def vkrgss_rec_block; [1].each { vkrgss_rec_block }; end; vkrgss_rec_block",
    "$vkrgss_rec_lambda = lambda { $vkrgss_rec_lambda.call }; $vkrgss_rec_lambda.call",
    "$vkrgss_rec_source = 'eval($vkrgss_rec_source)'; eval($vkrgss_rec_source)",
];

/// Runs each unbounded recursion several times, in the main program and inside a Fiber, and
/// checks that it raises `SystemStackError` and that the interpreter still computes
/// afterwards.
pub(crate) fn assert_recursion_is_bounded(interpreter: &vkrgss_ruby192_sys::Interpreter) {
    for source in UNBOUNDED_RECURSIONS {
        for (place, source) in [
            ("the main program", source.to_owned()),
            ("a Fiber", format!("Fiber.new {{ {source} }}.resume")),
        ] {
            for round in 0..3 {
                let script = source.clone();
                let outcome = interpreter.run(move |ruby| eval_inspect(ruby, &script));
                let error = outcome.expect_err("unbounded recursion cannot return");
                assert!(
                    error.starts_with("SystemStackError: "),
                    "round {round} of {source:?} in {place} raised {error}"
                );
                let after = interpreter.run(|ruby| eval_inspect(ruby, "1 + 1"));
                assert_eq!(
                    after,
                    Ok("2".to_owned()),
                    "after round {round} of {source:?} in {place}"
                );
            }
        }
    }
}

/// The current disposition of a signal, as (handler, flags, mask).
pub(crate) fn disposition(signal: libc::c_int) -> (usize, libc::c_int, Vec<libc::c_int>) {
    // SAFETY: a null new action only queries, and a zeroed `sigaction` is a valid buffer for
    // the old one. `sigismember` reads the mask that was just filled in.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        assert_eq!(libc::sigaction(signal, std::ptr::null(), &mut action), 0);
        let mask = (1..=64)
            .filter(|&other| libc::sigismember(&action.sa_mask, other) == 1)
            .collect();
        (action.sa_sigaction, action.sa_flags, mask)
    }
}

/// The current disposition of each signal Ruby takes, as (signal, handler, flags, mask).
pub(crate) fn dispositions() -> Vec<(libc::c_int, usize, libc::c_int, Vec<libc::c_int>)> {
    vkrgss_ruby192_sys::SIGNALS_TAKEN_BY_RUBY
        .iter()
        .map(|&signal| {
            let (handler, flags, mask) = disposition(signal);
            (signal, handler, flags, mask)
        })
        .collect()
}

/// The signals from 1 to 64 that the calling thread blocks.
pub(crate) fn blocked_signals() -> Vec<libc::c_int> {
    // SAFETY: a null new set only queries, and a zeroed set is a valid buffer for the old
    // one. `sigismember` reads the set that was just filled in.
    unsafe {
        let mut mask: libc::sigset_t = std::mem::zeroed();
        assert_eq!(
            libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut mask),
            0
        );
        (1..=64)
            .filter(|&signal| libc::sigismember(&mask, signal) == 1)
            .collect()
    }
}

/// How many threads the process has, as the kernel counts them.
pub(crate) fn thread_count() -> usize {
    fs::read_dir("/proc/self/task")
        .expect("the threads of the process are listed")
        .count()
}

pub(crate) mod suites;

pub(crate) mod frames;

pub(crate) mod copy;

pub(crate) mod instrumented;
