//! Support shared by the integration tests: the C compiler exactly as the build script used
//! it, scratch directories below cargo's target directory, and Ruby's own test executable.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The vendored Ruby tree.
pub(crate) fn source_dir() -> PathBuf {
    PathBuf::from(env!("VKXP_RUBY192_SOURCE_DIR"))
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
    let mut command = Command::new(env!("VKXP_RUBY192_CC"));
    command
        .args(
            env!("VKXP_RUBY192_CFLAGS")
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
    let ruby_libs = Path::new(env!("VKXP_RUBY192_LIB_DIR"));
    let zlib_libs = Path::new(env!("VKXP_RUBY192_ZLIB_LIB_DIR"));
    vec![
        PathBuf::from("-Wl,--start-group"),
        ruby_libs.join("libvkxp-ruby192-zlib.a"),
        ruby_libs.join("libvkxp-ruby192-static.a"),
        ruby_libs.join("libvkxp-ruby192-enc.a"),
        PathBuf::from("-Wl,--end-group"),
        zlib_libs.join("libz.a"),
        PathBuf::from("-lpthread"),
        PathBuf::from("-lrt"),
        PathBuf::from("-lcrypt"),
        PathBuf::from("-lm"),
    ]
}

/// The extensions that only Ruby's test executable links, with the defines of their own
/// `extconf.rb` on this target.
const TEST_EXTENSIONS: &[(&str, &[&str])] = &[
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
             __attribute__((visibility(\"default\"))) int vkxp_export_probe = 1;\n\
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
/// leaked symbols: `vkxp_export_probe` is the one symbol given default visibility on purpose,
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
