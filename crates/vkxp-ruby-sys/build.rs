//! Builds the vendored Ruby 1.8.7 interpreter and its `Zlib` extension as static libraries.
//!
//! The file list and the flags are fixed: no `configure`, no `make`, no installed Ruby. The one
//! file `configure` would generate, `config.h`, is checked in per target under `config/`.
//!
//! `-std=gnu89` and `-fwrapv` are not optional. Without the first the sources do not compile
//! under today's default C dialect; without the second the build succeeds and the overflow
//! checks that promote to `Bignum` are optimised away (`2**64` evaluates to 0). Ruby's own test
//! suites, run by `tests/upstream.rs`, are what detects a wrong build.

use std::env;
use std::path::{Path, PathBuf};

/// The targets that have a checked-in `config.h`.
const SUPPORTED_TARGETS: &[&str] = &["x86_64-unknown-linux-gnu"];

/// The C files of `libruby-static.a` in the release's own build, without their extension.
const CORE_SOURCES: &[&str] = &[
    "array",
    "bignum",
    "class",
    "compar",
    "dir",
    "dln",
    "enum",
    "enumerator",
    "error",
    "eval",
    "file",
    "gc",
    "hash",
    "inits",
    "io",
    "marshal",
    "math",
    "numeric",
    "object",
    "pack",
    "parse",
    "process",
    "prec",
    "random",
    "range",
    "re",
    "regex",
    "ruby",
    "signal",
    "sprintf",
    "st",
    "string",
    "struct",
    "time",
    "util",
    "variable",
    "version",
    "dmyext",
];

/// The pinned flags. `-fvisibility=hidden` is the one deliberate difference from the release's
/// build: it keeps names such as `re_set_syntax`, which glibc also defines, out of the dynamic
/// symbol table of the final executable.
const FLAGS: &[&str] = &[
    "-std=gnu89",
    "-fwrapv",
    "-fno-strict-aliasing",
    "-fvisibility=hidden",
];

/// clang turns these three diagnostics of the K&R-style sources into errors by default.
const CLANG_FLAGS: &[&str] = &[
    "-Wno-error=int-conversion",
    "-Wno-error=incompatible-pointer-types",
    "-Wno-error=incompatible-function-pointer-types",
];

/// The configuration shared by every C file of the interpreter, without include paths.
fn base_build() -> cc::Build {
    let mut build = cc::Build::new();
    build
        .opt_level(2)
        .pic(true)
        .warnings(false)
        .extra_warnings(false)
        .force_frame_pointer(true)
        .define("RUBY_EXPORT", None)
        .define("_GNU_SOURCE", "1");
    for flag in FLAGS {
        build.flag(flag);
    }
    if build.get_compiler().is_like_clang() {
        for flag in CLANG_FLAGS {
            build.flag(flag);
        }
    }
    build
}

/// A path as a string for `cargo:` lines, which cannot carry anything but UTF-8.
fn utf8(path: &Path) -> &str {
    path.to_str()
        .unwrap_or_else(|| panic!("path is not UTF-8: {}", path.display()))
}

fn main() {
    let target = env::var("TARGET").expect("cargo sets TARGET");
    if !SUPPORTED_TARGETS.contains(&target.as_str()) {
        panic!(
            "vkxp-ruby-sys cannot be built for target `{target}`: it has no checked-in Ruby \
             configuration header. Supported: {}. A new target needs its own verified \
             `config/<target>/config.h`.",
            SUPPORTED_TARGETS.join(", ")
        );
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let config_dir = manifest_dir.join("config").join(&target);
    let ruby_dir = manifest_dir
        .join("../../third_party/ruby-1.8.7")
        .canonicalize()
        .expect("third_party/ruby-1.8.7 exists");
    let zlib_include = PathBuf::from(
        env::var_os("DEP_Z_INCLUDE").expect("libz-sys exports the include path of its zlib"),
    );
    let zlib_root =
        PathBuf::from(env::var_os("DEP_Z_ROOT").expect("libz-sys exports the root of its zlib"));

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed={}", utf8(&ruby_dir));

    let base = base_build();

    // The interpreter. The include order matters: `config.h` first, then the vendored tree.
    let mut core = base.clone();
    core.include(&config_dir).include(&ruby_dir);
    for name in CORE_SOURCES {
        core.file(ruby_dir.join(format!("{name}.c")));
    }
    core.compile("vkxp-ruby-static");

    // The `Zlib` extension, against the zlib that `libz-sys` built from source.
    let mut zlib = base.clone();
    zlib.include(&config_dir)
        .include(&ruby_dir)
        .include(&zlib_include)
        .define("HAVE_ZLIB_H", None)
        .define("OS_CODE", "OS_UNIX")
        .file(ruby_dir.join("ext/zlib/zlib.c"));
    zlib.compile("vkxp-ruby-zlib");

    println!("cargo:rustc-link-lib=crypt");
    println!("cargo:rustc-link-lib=m");

    // What the tests need to build Ruby's own test executable exactly as the library was
    // built: the compiler, its arguments, and where the inputs and outputs are.
    let compiler = base.get_compiler();
    let args: Vec<&str> = compiler
        .args()
        .iter()
        .map(|arg| arg.to_str().expect("compiler arguments are UTF-8"))
        .collect();
    assert!(
        args.iter().all(|arg| !arg.contains(char::is_whitespace)),
        "a compiler argument contains white space and cannot be passed on: {args:?}"
    );
    println!("cargo:rustc-env=VKXP_RUBY_CC={}", utf8(compiler.path()));
    println!("cargo:rustc-env=VKXP_RUBY_CFLAGS={}", args.join(" "));
    println!("cargo:rustc-env=VKXP_RUBY_CONFIG_DIR={}", utf8(&config_dir));
    println!("cargo:rustc-env=VKXP_RUBY_SOURCE_DIR={}", utf8(&ruby_dir));
    println!("cargo:rustc-env=VKXP_RUBY_LIB_DIR={}", utf8(&out_dir));
    println!(
        "cargo:rustc-env=VKXP_RUBY_ZLIB_LIB_DIR={}",
        utf8(&zlib_root.join("lib"))
    );
}
