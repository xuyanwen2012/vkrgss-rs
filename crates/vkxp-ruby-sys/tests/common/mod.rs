//! Support shared by the integration tests: the C compiler exactly as the build script used
//! it, and scratch directories below cargo's target directory.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The vendored Ruby tree.
pub fn source_dir() -> PathBuf {
    PathBuf::from(env!("VKXP_RUBY_SOURCE_DIR"))
}

/// A fresh, empty directory for one test, below cargo's directory for test files.
pub fn scratch_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    }
    fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    dir
}

/// The compiler with the flags and defines the interpreter was built with, the include paths
/// of `config.h` and the vendored tree, and a temporary directory that is surely writable.
pub fn compiler(scratch: &Path) -> Command {
    let mut command = Command::new(env!("VKXP_RUBY_CC"));
    command
        .args(
            env!("VKXP_RUBY_CFLAGS")
                .split(' ')
                .filter(|arg| !arg.is_empty()),
        )
        .arg("-I")
        .arg(env!("VKXP_RUBY_CONFIG_DIR"))
        .arg("-I")
        .arg(source_dir())
        .env("TMPDIR", scratch);
    command
}

/// Runs a command to completion and returns its standard output; panics with both output
/// streams if it fails.
pub fn run(command: &mut Command) -> String {
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
pub fn compile_object(scratch: &Path, source: &Path) -> PathBuf {
    let stem = source.file_stem().expect("a file name");
    let object = scratch.join(stem).with_extension("o");
    run(compiler(scratch)
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(&object));
    object
}

/// Builds Ruby's own command-line executable from the archives the build script produced:
/// `main.c`, the `fcntl` and `stringio` extensions that Ruby's tests load, and an `Init_ext`
/// that registers them and `Zlib`. `-rdynamic` exports every symbol of default visibility, as
/// a real `ruby` does, which is also what makes the executable a probe for leaked symbols.
pub fn build_ruby_executable(scratch: &Path) -> PathBuf {
    let source = source_dir();
    let init_ext = scratch.join("extinit.c");
    fs::write(
        &init_ext,
        "#include \"ruby.h\"\n\
         void ruby_init_ext _((const char *name, void (*init)(void)));\n\
         void Init_zlib _((void));\n\
         void Init_fcntl _((void));\n\
         void Init_stringio _((void));\n\
         void Init_ext _((void))\n\
         {\n\
         \x20   ruby_init_ext(\"zlib.so\", Init_zlib);\n\
         \x20   ruby_init_ext(\"fcntl.so\", Init_fcntl);\n\
         \x20   ruby_init_ext(\"stringio.so\", Init_stringio);\n\
         }\n",
    )
    .expect("extinit.c is writable");

    let objects = [
        compile_object(scratch, &source.join("main.c")),
        compile_object(scratch, &init_ext),
        compile_object(scratch, &source.join("ext/fcntl/fcntl.c")),
        compile_object(scratch, &source.join("ext/stringio/stringio.c")),
    ];

    // The objects come first so that their `Init_ext` wins over the empty one of `dmyext.o`.
    let executable = scratch.join("ruby");
    let ruby_libs = Path::new(env!("VKXP_RUBY_LIB_DIR"));
    let zlib_libs = Path::new(env!("VKXP_RUBY_ZLIB_LIB_DIR"));
    run(compiler(scratch)
        .arg("-rdynamic")
        .args(&objects)
        .arg(ruby_libs.join("libvkxp-ruby-zlib.a"))
        .arg(ruby_libs.join("libvkxp-ruby-static.a"))
        .arg(zlib_libs.join("libz.a"))
        .args(["-lcrypt", "-lm"])
        .arg("-o")
        .arg(&executable));
    executable
}
