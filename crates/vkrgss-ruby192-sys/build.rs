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
//! The crate is for one target. For any other the script compiles nothing and says so, and
//! the crate is empty.

use std::env;
use std::path::{Path, PathBuf};

/// The one target the interpreter is built for, and the name of its directory under `config/`.
const TARGET: &str = "x86_64-unknown-linux-gnu";

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

/// The pinned flags. `-fvisibility=hidden` is the one deliberate difference from the release's
/// build: it keeps the interpreter's names out of the dynamic symbol table of the final
/// executable.
const FLAGS: &[&str] = &[
    "-std=gnu89",
    "-fwrapv",
    "-fno-strict-aliasing",
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

/// The configuration shared by every C file, without include paths.
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
    for flag in FLAGS.iter().chain(FORTIFY_FLAGS).chain(HARDENING_FLAGS) {
        build.flag(flag);
    }
    if env::var("CARGO_CFG_TARGET_ARCH").expect("cargo sets CARGO_CFG_TARGET_ARCH") == "x86_64" {
        for flag in X86_64_HARDENING_FLAGS {
            build.flag(flag);
        }
    }
    if build.get_compiler().is_like_clang() {
        for flag in CLANG_FLAGS {
            build.flag(flag);
        }
    }
    build
}

/// A path as a string for `cargo:` lines, which cannot carry anything but UTF-8.
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

fn utf8(path: &Path) -> &str {
    path.to_str()
        .unwrap_or_else(|| panic!("path is not UTF-8: {}", path.display()))
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = env::var("TARGET").expect("cargo sets TARGET");
    if target != TARGET {
        println!(
            "cargo:warning=vkrgss-ruby192-sys is empty for target `{target}`: the embedded Ruby \
             1.9.2 is built for `{TARGET}` only"
        );
        return;
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let config_dir = manifest_dir.join("config").join(TARGET);
    let ruby_dir = manifest_dir
        .join("../../third_party/ruby-1.9.2")
        .canonicalize()
        .expect("third_party/ruby-1.9.2 exists");
    let zlib_include = PathBuf::from(
        env::var_os("DEP_Z_INCLUDE").expect("libz-sys exports the include path of its zlib"),
    );
    let zlib_root =
        PathBuf::from(env::var_os("DEP_Z_ROOT").expect("libz-sys exports the root of its zlib"));

    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed=encodings.c");
    println!("cargo:rerun-if-changed={}", utf8(&ruby_dir));

    // The include order matters: `ruby/config.h` first, then the vendored tree.
    let mut base = base_build();
    base.include(&config_dir)
        .include(ruby_dir.join("include"))
        .include(&ruby_dir);

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
    println!(
        "cargo:rustc-env=VKRGSS_RUBY192_CC={}",
        utf8(compiler.path())
    );
    println!("cargo:rustc-env=VKRGSS_RUBY192_CFLAGS={}", args.join(" "));
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
