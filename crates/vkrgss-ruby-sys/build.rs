//! Builds the vendored Ruby 1.8.7 interpreter, its `Zlib` extension and the vendored Oniguruma
//! regular expression engine as static libraries.
//!
//! The file list and the flags are fixed: no `configure`, no `make`, no installed Ruby. The one
//! file `configure` would generate, `config.h`, is checked in per target under `config/`, for
//! the interpreter and, in `oniguruma/` below it, for the engine.
//!
//! `-std=gnu89` and `-fwrapv` are not optional. Without the first the sources do not compile
//! under today's default C dialect; without the second the build succeeds and the overflow
//! checks that promote to `Bignum` are optimised away (`2**64` evaluates to 0). Ruby's own test
//! suites, run by `tests/upstream.rs`, are what detects a wrong build.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The targets the build accepts, each with the target whose checked-in `config.h` it uses.
///
/// Only x86_64 is supported. aarch64 is accepted so that the CI job `gate-aarch64` shows
/// what works there: it has no header of its own and is built with that of x86_64, which
/// describes the same C library and the same sizes but was not verified on aarch64, and
/// which names the platform `x86_64-linux`.
const TARGETS: &[(&str, &str)] = &[
    ("x86_64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"),
    ("aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"),
];

/// The C files of `libruby-static.a` in the release's own build, without their extension and
/// without `regex`, the interpreter's own regular expression engine: Oniguruma replaces it.
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

/// The C files of Oniguruma that are built: the engine, its GNU-compatible interface, and the
/// encodings that `$KCODE` can select (`unicode` is what the UTF-8 one needs).
const ONIGURUMA_SOURCES: &[&str] = &[
    "regcomp",
    "regenc",
    "regerror",
    "regexec",
    "regext",
    "reggnu",
    "regparse",
    "regsyntax",
    "regtrav",
    "regversion",
    "st",
    "enc/ascii",
    "enc/euc_jp",
    "enc/sjis",
    "enc/unicode",
    "enc/utf8",
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

/// The measured start-up tables, in the order of the numbers that
/// `ruby_vkxp_select_symbols` takes: the suffix of the C array and the data file.
const SYMBOL_TABLES: &[(&str, &str)] = &[
    ("rgss1", "data/symbols.startup.txt"),
    ("rgss2", "data/symbols.startup.rgss2.txt"),
];

/// Reads one measured table: an `id name` pair per line after a `#` header line. Returns
/// the pairs sorted by name in the order of `strcmp`, for a binary search.
fn read_symbol_table(data: &Path) -> Vec<(String, u64)> {
    let text =
        fs::read_to_string(data).unwrap_or_else(|error| panic!("{}: {error}", data.display()));
    let mut symbols: Vec<(String, u64)> = text
        .lines()
        .filter(|line| !line.starts_with("# "))
        .map(|line| {
            let (id, name) = line
                .split_once(' ')
                .unwrap_or_else(|| panic!("malformed symbol table line: {line:?}"));
            let id: u64 = id
                .parse()
                .unwrap_or_else(|_| panic!("malformed symbol number: {line:?}"));
            assert!(
                !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_graphic()),
                "symbol name is not printable ASCII: {line:?}"
            );
            (name.to_owned(), id)
        })
        .collect();
    symbols.sort_unstable();
    for pair in symbols.windows(2) {
        assert!(
            pair[0].0 != pair[1].0,
            "symbol {} is listed twice in {}",
            pair[0].0,
            data.display()
        );
    }
    symbols
}

/// Writes `vkxp_symbol_table.h`, which the patched `parse.c` includes, from the measured
/// tables of [`SYMBOL_TABLES`]: every table as a C array, and `vkxp_symbol_tables`, which
/// lists them with their lengths and their largest counters.
fn generate_symbol_table(manifest_dir: &Path, header: &Path) {
    let mut out = String::from(
        "/* Generated by build.rs from the files data/symbols.startup*.txt. */\n\
         struct vkrgss_symbol { const char *name; ID id; };\n\
         struct vkxp_symbol_table { const struct vkrgss_symbol *symbols; int length; ID last; };\n",
    );
    let mut index = String::new();
    for (position, (suffix, data)) in SYMBOL_TABLES.iter().enumerate() {
        let symbols = read_symbol_table(&manifest_dir.join(data));
        let last = symbols.iter().map(|&(_, id)| id >> 3).max().unwrap_or(0);
        if position == 0 {
            writeln!(out, "#define VKXP_LAST_MEASURED_DEFAULT {last}").unwrap();
        }
        writeln!(
            out,
            "static const struct vkrgss_symbol vkrgss_symbol_table_{suffix}[] = {{"
        )
        .unwrap();
        for (name, id) in &symbols {
            let name = name.replace('\\', "\\\\").replace('"', "\\\"");
            writeln!(out, "    {{\"{name}\", {id}}},").unwrap();
        }
        out.push_str("};\n");
        writeln!(
            index,
            "    {{vkrgss_symbol_table_{suffix}, {}, {last}}},",
            symbols.len()
        )
        .unwrap();
    }
    out.push_str("static const struct vkxp_symbol_table vkxp_symbol_tables[] = {\n");
    out.push_str(&index);
    out.push_str("};\n");
    fs::write(header, out).unwrap_or_else(|error| panic!("{}: {error}", header.display()));
}

/// Links libcrypt, which `String#crypt` needs: dynamically, or statically when
/// `VKRGSS_STATIC_CRYPT` is `1`, as `cargo xtask dist` sets it, because the shared library has
/// a different name on Ubuntu (`libcrypt.so.1`) and on Fedora (`libcrypt.so.2`).
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

fn main() {
    let target = env::var("TARGET").expect("cargo sets TARGET");
    let Some(&(_, config_target)) = TARGETS.iter().find(|(name, _)| *name == target) else {
        let (supported, borrowing): (Vec<_>, Vec<_>) =
            TARGETS.iter().partition(|(name, config)| name == config);
        let names = |targets: Vec<&(&'static str, &str)>| -> String {
            let names: Vec<&str> = targets.iter().map(|&&(name, _)| name).collect();
            names.join(", ")
        };
        panic!(
            "vkrgss-ruby-sys cannot be built for target `{target}`: it has no checked-in Ruby \
             configuration header. Supported: {}. Built with the header of a supported \
             target, unsupported: {}. A new target needs its own verified \
             `config/<target>/config.h`.",
            names(supported),
            names(borrowing)
        );
    };

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let config_dir = manifest_dir.join("config").join(config_target);
    let ruby_dir = manifest_dir
        .join("../../third_party/ruby-1.8.7")
        .canonicalize()
        .expect("third_party/ruby-1.8.7 exists");
    let onig_dir = manifest_dir
        .join("../../third_party/oniguruma-5.9.6")
        .canonicalize()
        .expect("third_party/oniguruma-5.9.6 exists");
    let zlib_include = PathBuf::from(
        env::var_os("DEP_Z_INCLUDE").expect("libz-sys exports the include path of its zlib"),
    );
    let zlib_root =
        PathBuf::from(env::var_os("DEP_Z_ROOT").expect("libz-sys exports the root of its zlib"));

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=compat");
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed=data");
    println!("cargo:rerun-if-changed={}", utf8(&ruby_dir));
    println!("cargo:rerun-if-changed={}", utf8(&onig_dir));

    let base = base_build();

    let generated_dir = out_dir.join("generated");
    fs::create_dir_all(&generated_dir).expect("OUT_DIR is writable");
    generate_symbol_table(&manifest_dir, &generated_dir.join("vkxp_symbol_table.h"));

    // The interpreter. The include order matters: `config.h` first, then the vendored tree.
    // `compat` holds `vkxp_regex.h`, which the patched `re.h` includes and which in turn
    // includes Oniguruma's `oniggnu.h`.
    let compat_dir = manifest_dir.join("compat");
    let mut core = base.clone();
    core.include(&config_dir)
        .include(&ruby_dir)
        .include(&generated_dir)
        .include(&compat_dir)
        .include(&onig_dir);
    for name in CORE_SOURCES {
        core.file(ruby_dir.join(format!("{name}.c")));
    }
    core.compile("vkrgss-ruby-static");

    // The `Zlib` extension, against the zlib that `libz-sys` built from source.
    let mut zlib = base.clone();
    zlib.include(&config_dir)
        .include(&ruby_dir)
        .include(&zlib_include)
        .define("HAVE_ZLIB_H", None)
        .define("OS_CODE", "OS_UNIX")
        .file(ruby_dir.join("ext/zlib/zlib.c"));
    zlib.compile("vkrgss-ruby-zlib");

    // The engine, after the interpreter that refers to it. Its own `config.h` comes first.
    let mut onig = base.clone();
    onig.include(config_dir.join("oniguruma"))
        .include(&onig_dir);
    for name in ONIGURUMA_SOURCES {
        onig.file(onig_dir.join(format!("{name}.c")));
    }
    onig.compile("vkrgss-ruby-onig");

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
    // And what the opt-in sanitized run needs to compile the same files itself, with a
    // compiler that has the sanitizer runtimes.
    println!(
        "cargo:rustc-env=VKRGSS_RUBY_CORE_SOURCES={}",
        CORE_SOURCES.join(" ")
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY_ONIG_SOURCES={}",
        ONIGURUMA_SOURCES.join(" ")
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY_CLANG_CFLAGS={}",
        CLANG_FLAGS.join(" ")
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY_GENERATED_DIR={}",
        utf8(&generated_dir)
    );
    println!(
        "cargo:rustc-env=VKRGSS_RUBY_ZLIB_INCLUDE_DIR={}",
        utf8(&zlib_include)
    );
    println!("cargo:rustc-env=VKRGSS_RUBY_CC={}", utf8(compiler.path()));
    println!("cargo:rustc-env=VKRGSS_RUBY_CFLAGS={}", args.join(" "));
    println!(
        "cargo:rustc-env=VKRGSS_RUBY_CONFIG_DIR={}",
        utf8(&config_dir)
    );
    println!("cargo:rustc-env=VKRGSS_RUBY_SOURCE_DIR={}", utf8(&ruby_dir));
    println!(
        "cargo:rustc-env=VKRGSS_RUBY_COMPAT_DIR={}",
        utf8(&compat_dir)
    );
    println!("cargo:rustc-env=VKRGSS_RUBY_ONIG_DIR={}", utf8(&onig_dir));
    println!("cargo:rustc-env=VKRGSS_RUBY_LIB_DIR={}", utf8(&out_dir));
    println!(
        "cargo:rustc-env=VKRGSS_RUBY_ZLIB_LIB_DIR={}",
        utf8(&zlib_root.join("lib"))
    );
}
