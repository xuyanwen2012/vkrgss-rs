//! Builds the vendored Ruby 1.9.2-p0 interpreter, the encodings and transcoders the original
//! runtime has, and its `Zlib` extension as static libraries.
//!
//! The file list and the flags are fixed: no `configure`, no `make`, no installed Ruby, and
//! nothing is generated. The release ships every generated source the core needs; the one
//! file `configure` would write, `ruby/config.h`, is checked in under `config/`.
//!
//! `-std=gnu89` and `-fwrapv` are not optional. Without the first the sources do not compile
//! under today's default C dialect; without the second the build succeeds and overflow checks
//! are optimised away. Ruby's own test suites, run by `tests/upstream.rs`, are what detects a
//! wrong build.
//!
//! The mathematical functions of `Math` and the power of floats come from the vendored
//! CORE-MATH, which is correctly rounded: the files `third_party/core-math/FILES` lists,
//! built into a library of their own, and a recorded patch that makes the interpreter call
//! them. Neither is compiled with contraction of floating-point operations.
//!
//! The crate is for the targets of `targets.rs`. For any other the script compiles nothing
//! and says so, and sets the `cfg` `vkrgss_interpreter_absent`, under which the crate is
//! empty.

mod targets;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use targets::{TARGETS, is_built_for};

/// The C files of `libruby-static.a` in the release's own build, without their extension,
/// with these differences: `miniprelude` stands for `prelude`, which the release generates
/// with an interpreter (`ruby.c` refers to `Init_prelude`; this crate never calls it), and
/// `dmydln` stands for `dln`, because nothing is loaded dynamically.
const CORE_SOURCES: &[&str] = &[
    "array",
    "bignum",
    "class",
    "compar",
    "complex",
    "dir",
    "dln_find",
    "enum",
    "enumerator",
    "error",
    "eval",
    "load",
    "proc",
    "file",
    "gc",
    "hash",
    "inits",
    "io",
    "marshal",
    "math",
    "node",
    "numeric",
    "object",
    "pack",
    "parse",
    "process",
    "random",
    "range",
    "rational",
    "re",
    "regcomp",
    "regenc",
    "regerror",
    "regexec",
    "regparse",
    "regsyntax",
    "ruby",
    "safe",
    "signal",
    "sprintf",
    "st",
    "strftime",
    "string",
    "struct",
    "time",
    "transcode",
    "util",
    "variable",
    "compile",
    "debug",
    "iseq",
    "vm",
    "vm_dump",
    "thread",
    "cont",
    "enc/ascii",
    "enc/us_ascii",
    "enc/unicode",
    "enc/utf_8",
    "enc/trans/newline",
    "encoding",
    "version",
    "miniprelude",
    "dmyext",
    "dmydln",
];

/// The encodings linked beside the four of the core: those the original runtime was measured
/// to have. They are compiled with `ONIG_ENC_REGISTER`, without which they define no `Init_`
/// function.
const ENCODING_SOURCES: &[&str] = &["enc/utf_16le", "enc/shift_jis", "enc/iso_8859_1"];

/// The transcoder libraries linked beside `newline`.
const TRANSCODER_SOURCES: &[&str] = &[
    "enc/trans/japanese_sjis",
    "enc/trans/single_byte",
    "enc/trans/utf_16_32",
];

/// The pinned flags. `-fvisibility=hidden` keeps the interpreter's names out of the dynamic
/// symbol table of the final executable. `-ffp-contract=off` keeps the compiler from fusing a
/// multiplication and an addition, which it would do on one architecture and not on another,
/// so that a float is computed the same everywhere. These two are the deliberate differences
/// from the release's build.
const FLAGS: &[&str] = &[
    "-std=gnu89",
    "-fwrapv",
    "-fno-strict-aliasing",
    "-fvisibility=hidden",
    "-ffp-contract=off",
];

/// The flags of CORE-MATH, which is C11, besides the hardening flags: no contraction, no
/// fast-math, which would break its error-free transformations, and hidden names. As the
/// build script of `vkrgss-ruby-sys` has them.
const CORE_MATH_FLAGS: &[&str] = &[
    "-std=gnu11",
    "-ffp-contract=off",
    "-fno-fast-math",
    "-fvisibility=hidden",
];

/// clang turns these three diagnostics of the old sources into errors by default.
const CLANG_FLAGS: &[&str] = &[
    "-Wno-error=int-conversion",
    "-Wno-error=incompatible-pointer-types",
    "-Wno-error=incompatible-function-pointer-types",
];

/// The hardening flags, asked for whatever the compiler defines by default, for gcc and clang
/// alike: a compiler that rejects one fails the build. `_FORTIFY_SOURCE` is undefined first,
/// because a compiler that defines it by default warns about a second definition.
const FORTIFY_FLAGS: &[&str] = &["-U_FORTIFY_SOURCE", "-D_FORTIFY_SOURCE=3"];

/// The stack hardening of every architecture: canaries in frames with arrays or addresses of
/// locals, and probes of every page a large frame or `alloca` reaches.
const HARDENING_FLAGS: &[&str] = &["-fstack-protector-strong", "-fstack-clash-protection"];

/// The hardening of x86_64 alone: the instructions of CET, which also mark the indirect
/// branch targets.
const X86_64_HARDENING_FLAGS: &[&str] = &["-fcf-protection=full"];

/// The configuration shared by every C file, `flags` first, then the hardening flags.
fn hardened_build(flags: &[&str]) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .opt_level(2)
        .pic(true)
        .warnings(false)
        .extra_warnings(false)
        .force_frame_pointer(true);
    for flag in flags.iter().chain(FORTIFY_FLAGS).chain(HARDENING_FLAGS) {
        build.flag(flag);
    }
    if env::var("CARGO_CFG_TARGET_ARCH").expect("cargo sets CARGO_CFG_TARGET_ARCH") == "x86_64" {
        for flag in X86_64_HARDENING_FLAGS {
            build.flag(flag);
        }
    }
    build
}

/// The configuration shared by every C file of the interpreter, without include paths.
fn base_build() -> cc::Build {
    let mut build = hardened_build(FLAGS);
    build.define("RUBY_EXPORT", None).define("_GNU_SOURCE", "1");
    if build.get_compiler().is_like_clang() {
        for flag in CLANG_FLAGS {
            build.flag(flag);
        }
    }
    build
}

/// The C files of `third_party/core-math/FILES`, which both interpreters build, as the build
/// script of `vkrgss-ruby-sys` reads them: one path per line, relative to that directory;
/// empty lines and lines that start with `#` are skipped.
fn core_math_files(core_math_dir: &Path) -> Vec<PathBuf> {
    let list = core_math_dir.join("FILES");
    let text =
        fs::read_to_string(&list).unwrap_or_else(|error| panic!("{}: {error}", list.display()));
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| core_math_dir.join(line))
        .collect()
}

/// Links libcrypt, which `String#crypt` needs: dynamically, or statically when
/// `VKRGSS_STATIC_CRYPT` is `1`, as `cargo xtask dist` sets it, because the shared library has
/// a different name on Ubuntu (`libcrypt.so.1`) and on Fedora (`libcrypt.so.2`). As the
/// build script of `vkrgss-ruby-sys` does.
fn link_crypt(base: &cc::Build) {
    println!("cargo:rerun-if-env-changed=VKRGSS_STATIC_CRYPT");
    let value = env::var("VKRGSS_STATIC_CRYPT").unwrap_or_default();
    match value.as_str() {
        "" | "0" => println!("cargo:rustc-link-lib=crypt"),
        "1" => {
            // rustc looks for a static library only in its own search paths, so it is given
            // the directory in which the C compiler finds the library.
            let compiler = base.get_compiler();
            let output = compiler
                .to_command()
                .arg("-print-file-name=libcrypt.a")
                .output()
                .unwrap_or_else(|error| panic!("{}: {error}", compiler.path().display()));
            let printed = String::from_utf8_lossy(&output.stdout);
            let directory = Path::new(printed.trim())
                .parent()
                .filter(|directory| directory.is_dir())
                .unwrap_or_else(|| {
                    panic!(
                        "VKRGSS_STATIC_CRYPT=1 needs the static library `libcrypt.a`, which the \
                         C compiler does not find. It is in the package `libcrypt-dev` on \
                         Debian and Ubuntu and `libxcrypt-static` on Fedora."
                    )
                });
            println!("cargo:rustc-link-search=native={}", utf8(directory));
            println!("cargo:rustc-link-lib=static=crypt");
        }
        other => panic!("VKRGSS_STATIC_CRYPT is `{other}`; it must be unset, empty, `0` or `1`"),
    }
}

/// A path as a string for `cargo:` lines, which cannot carry anything but UTF-8.
fn utf8(path: &Path) -> &str {
    path.to_str()
        .unwrap_or_else(|| panic!("path is not UTF-8: {}", path.display()))
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=targets.rs");
    let target = env::var("TARGET").expect("cargo sets TARGET");
    if !is_built_for(&target) {
        println!(
            "cargo:warning=vkrgss-ruby192-sys is empty for target `{target}`: the embedded Ruby \
             1.9.2 is built for {} only",
            TARGETS.join(", ")
        );
        println!("cargo:rustc-cfg=vkrgss_interpreter_absent");
        return;
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let config_dir = manifest_dir.join("config").join(&target);
    let ruby_dir = manifest_dir
        .join("../../third_party/ruby-1.9.2")
        .canonicalize()
        .expect("third_party/ruby-1.9.2 exists");
    let core_math_dir = manifest_dir
        .join("../../third_party/core-math")
        .canonicalize()
        .expect("third_party/core-math exists");
    let zlib_include = PathBuf::from(
        env::var_os("DEP_Z_INCLUDE").expect("libz-sys exports the include path of its zlib"),
    );
    let zlib_root =
        PathBuf::from(env::var_os("DEP_Z_ROOT").expect("libz-sys exports the root of its zlib"));

    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed=encodings.c");
    println!("cargo:rerun-if-changed={}", utf8(&ruby_dir));
    println!("cargo:rerun-if-changed={}", utf8(&core_math_dir));

    // The include order matters: `ruby/config.h` first, then the vendored tree.
    // `vkrgss_math.h`, which the patched `math.c`, `numeric.c` and `bignum.c` include, is in
    // the directory of CORE-MATH.
    let mut base = base_build();
    base.include(&config_dir)
        .include(ruby_dir.join("include"))
        .include(&ruby_dir)
        .include(core_math_dir.join("vkrgss"));

    // The interpreter with its regular expression engine, the four encodings of the core and
    // the transcoders, and the table of the encodings that are registered at first use.
    let mut core = base.clone();
    for name in CORE_SOURCES.iter().chain(TRANSCODER_SOURCES) {
        core.file(ruby_dir.join(format!("{name}.c")));
    }
    core.file(manifest_dir.join("encodings.c"));
    core.compile("vkrgss-ruby192-static");

    let mut encodings = base.clone();
    encodings.define("ONIG_ENC_REGISTER", "rb_enc_register");
    for name in ENCODING_SOURCES {
        encodings.file(ruby_dir.join(format!("{name}.c")));
    }
    encodings.compile("vkrgss-ruby192-enc");

    // The `Zlib` extension, against the zlib that `libz-sys` built from source.
    let mut zlib = base.clone();
    zlib.include(&zlib_include)
        .define("HAVE_ZLIB_H", None)
        .define("OS_CODE", "OS_UNIX")
        .file(ruby_dir.join("ext/zlib/zlib.c"));
    zlib.compile("vkrgss-ruby192-zlib");

    // The correctly rounded functions, which the interpreter refers to.
    let mut core_math = hardened_build(CORE_MATH_FLAGS);
    core_math.files(core_math_files(&core_math_dir));
    core_math.compile("vkrgss-ruby192-core-math");

    for library in ["pthread", "rt"] {
        println!("cargo:rustc-link-lib={library}");
    }
    link_crypt(&base);
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
    // And what the opt-in sanitized run and the fuzz targets need to compile the same files
    // themselves, with a compiler that has the sanitizer runtimes.
    let core_sources: Vec<&str> = CORE_SOURCES
        .iter()
        .chain(TRANSCODER_SOURCES)
        .copied()
        .collect();
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_CORE_SOURCES={}",
        core_sources.join(" ")
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_ENCODING_SOURCES={}",
        ENCODING_SOURCES.join(" ")
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_CLANG_CFLAGS={}",
        CLANG_FLAGS.join(" ")
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_FORTIFY_CFLAGS={}",
        FORTIFY_FLAGS.join(" ")
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_ZLIB_INCLUDE_DIR={}",
        utf8(&zlib_include)
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_CC={}",
        utf8(compiler.path())
    );
    println!("cargo:rustc-env=VKRGSS_RUBY192_CFLAGS={}", args.join(" "));
    let core_math_compiler = core_math.get_compiler();
    let core_math_args: Vec<&str> = core_math_compiler
        .args()
        .iter()
        .map(|arg| arg.to_str().expect("compiler arguments are UTF-8"))
        .collect();
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_CORE_MATH_CFLAGS={}",
        core_math_args.join(" ")
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_SOURCE_DIR={}",
        utf8(&ruby_dir)
    );
    println!("cargo:rustc-env=VKRGSS_RUBY192_LIB_DIR={}", utf8(&out_dir));
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_ZLIB_LIB_DIR={}",
        utf8(&zlib_root.join("lib"))
    );
}
