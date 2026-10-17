//! The tests of the interpreter crates on a phone over adb: one command, run by hand and not
//! by the gate.
//!
//! `cargo xtask android-test <CRATE> [--device <SERIAL>]` cross-builds the test binaries of
//! `vkrgss-ruby-sys` or `vkrgss-ruby192-sys` for `aarch64-linux-android` at API level 34 with
//! the NDK that `ANDROID_NDK_HOME` names, and links Ruby's own test executable on the host as
//! `tests/common/mod.rs` of the crate does where a C compiler runs beside the tests. It pushes
//! both, the directory of the crate and the vendored trees its tests read into
//! [`DEVICE_DIR`], runs every binary there with one test thread under `umask 022`, writes
//! the output of each under `target/android-test/` and removes the directory from the phone.
//! The tests take the directories of the host from environment variables of the same names,
//! and the executable from `<PREFIX>_EXECUTABLE`. See `docs/building.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// The target the binaries are built for.
pub(crate) const TARGET: &str = "aarch64-linux-android";

/// The API level of the compiler, the lowest the interpreters are built for.
pub(crate) const API_LEVEL: u32 = 34;

/// The directory on the phone, the only one the command writes; removed after a run.
const DEVICE_DIR: &str = "/data/local/tmp/vkrgss-test";

/// How long one test binary may run on the phone, in seconds.
const BINARY_TIMEOUT: u64 = 3600;

/// One crate whose tests run on a phone.
#[derive(Debug, PartialEq, Eq)]
struct Crate {
    package: &'static str,
    /// The prefix of the variables its build script gives its tests.
    prefix: &'static str,
    /// The trees of `third_party/` its tests read; the first is the vendored Ruby.
    trees: &'static [&'static str],
}

/// The crates. The manifest test of `vkrgss-ruby-sys` checks every vendored tree.
const CRATES: [Crate; 2] = [
    Crate {
        package: "vkrgss-ruby-sys",
        prefix: "VKRGSS_RUBY",
        trees: &["ruby-1.8.7", "oniguruma-5.9.6", "core-math", "ruby-1.9.2"],
    },
    Crate {
        package: "vkrgss-ruby192-sys",
        prefix: "VKRGSS_RUBY192",
        trees: &["ruby-1.9.2"],
    },
];

pub(crate) const USAGE: &str =
    "usage: cargo xtask android-test <vkrgss-ruby-sys|vkrgss-ruby192-sys> [--device <SERIAL>]";

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    krate: &'static Crate,
    /// The serial of the phone; without one adb takes `ANDROID_SERIAL` or the only phone.
    device: Option<String>,
}

/// Parses the arguments after `android-test`.
pub(crate) fn parse(arguments: &[&str]) -> Result<Options> {
    let names = || {
        CRATES
            .iter()
            .map(|krate| format!("`{}`", krate.package))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let (name, rest) = match arguments {
        [name, rest @ ..] => (*name, rest),
        [] => bail!("android-test: no crate given; the crates are {}", names()),
    };
    let Some(krate) = CRATES.iter().find(|krate| krate.package == name) else {
        bail!(
            "android-test: no crate `{name}`; the crates are {}",
            names()
        );
    };
    let device = match rest {
        [] => None,
        ["--device", serial] if !serial.is_empty() => Some((*serial).to_owned()),
        _ => bail!("android-test: unexpected arguments `{}`", rest.join(" ")),
    };
    Ok(Options { krate, device })
}

/// The environment of the tests on the phone: the directories of the host, the executable,
/// and a home and temporary directory inside [`DEVICE_DIR`].
fn device_environment(krate: &Crate) -> Vec<(String, String)> {
    let tmp = format!("{DEVICE_DIR}/tmp");
    vec![
        (
            "CARGO_MANIFEST_DIR".to_owned(),
            format!("{DEVICE_DIR}/repo/crates/{}", krate.package),
        ),
        ("CARGO_TARGET_TMPDIR".to_owned(), tmp.clone()),
        (
            format!("{}_SOURCE_DIR", krate.prefix),
            format!("{DEVICE_DIR}/repo/third_party/{}", krate.trees[0]),
        ),
        (
            format!("{}_EXECUTABLE", krate.prefix),
            format!("{DEVICE_DIR}/ruby"),
        ),
        ("HOME".to_owned(), tmp.clone()),
        ("TMPDIR".to_owned(), tmp),
    ]
}

/// The shell command that runs one test binary on the phone.
fn device_command(krate: &Crate, binary: &str) -> String {
    let variables: Vec<String> = device_environment(krate)
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    format!(
        "cd {DEVICE_DIR}/tmp && umask 022 && env {} timeout {BINARY_TIMEOUT} \
         {DEVICE_DIR}/bin/{binary} --test-threads=1",
        variables.join(" ")
    )
}

/// The compiler of the NDK for [`API_LEVEL`] and its archiver.
pub(crate) fn ndk_tools() -> Result<(PathBuf, PathBuf)> {
    let ndk = std::env::var_os("ANDROID_NDK_HOME")
        .or_else(|| std::env::var_os("ANDROID_NDK_ROOT"))
        .context("neither ANDROID_NDK_HOME nor ANDROID_NDK_ROOT names the Android NDK")?;
    let bin = Path::new(&ndk).join("toolchains/llvm/prebuilt/linux-x86_64/bin");
    let clang = bin.join(format!("{TARGET}{API_LEVEL}-clang"));
    let ar = bin.join("llvm-ar");
    for tool in [&clang, &ar] {
        if !tool.is_file() {
            bail!("the NDK has no {}", tool.display());
        }
    }
    Ok((clang, ar))
}

/// One test binary that cargo built: the name of its target and its path.
#[derive(Debug, PartialEq, Eq)]
struct Binary {
    name: String,
    path: PathBuf,
}

/// The test binaries of the cargo messages `messages` (one JSON object per line) and the
/// variables that the build script of the crate gave them, which the one with
/// `<PREFIX>_CC` among them is.
fn read_messages(messages: &str, prefix: &str) -> Result<(Vec<Binary>, BTreeMap<String, String>)> {
    let compiler = format!("{prefix}_CC");
    let mut binaries = Vec::new();
    let mut variables = None;
    for line in messages.lines().filter(|line| line.starts_with('{')) {
        let message: Value = serde_json::from_str(line).context("android-test: cargo message")?;
        match message["reason"].as_str() {
            Some("compiler-artifact") if message["profile"]["test"] == true => {
                if let Some(path) = message["executable"].as_str() {
                    binaries.push(Binary {
                        name: message["target"]["name"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned(),
                        path: PathBuf::from(path),
                    });
                }
            }
            Some("build-script-executed") => {
                let pairs: BTreeMap<String, String> = message["env"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|pair| {
                        Some((pair[0].as_str()?.to_owned(), pair[1].as_str()?.to_owned()))
                    })
                    .collect();
                if pairs.contains_key(&compiler) {
                    variables = Some(pairs);
                }
            }
            _ => {}
        }
    }
    let Some(variables) = variables else {
        bail!("android-test: the build script gave no {compiler}");
    };
    if binaries.is_empty() {
        bail!("android-test: cargo built no test binary");
    }
    Ok((binaries, variables))
}

/// Builds the test binaries of the crate for [`TARGET`].
fn build(
    krate: &Crate,
    root: &Path,
    clang: &Path,
    ar: &Path,
) -> Result<(Vec<Binary>, BTreeMap<String, String>)> {
    let output = Command::new(crate::cargo())
        .args([
            "test",
            "-p",
            krate.package,
            "--locked",
            "--target",
            TARGET,
            "--no-run",
            "--message-format=json-render-diagnostics",
        ])
        .env("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", clang)
        .env("CC_aarch64_linux_android", clang)
        .env("AR_aarch64_linux_android", ar)
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .context("android-test: cargo could not be started")?;
    if !output.status.success() {
        bail!("android-test: the build of {} failed", krate.package);
    }
    read_messages(&String::from_utf8_lossy(&output.stdout), krate.prefix)
}

/// Runs a command of the host and fails with its output if it fails.
pub(crate) fn run_host(command: &mut Command) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("could not start {command:?}"))?;
    if !output.status.success() {
        bail!(
            "{command:?} ended with {}\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// A variable of the build script.
fn variable<'a>(variables: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str> {
    variables
        .get(name)
        .map(String::as_str)
        .with_context(|| format!("android-test: the build script gave no {name}"))
}

/// The `Init_ext` of Ruby 1.8.7's test executable, as `write_init_ext` of
/// `crates/vkrgss-ruby-sys/tests/common/mod.rs` writes it.
const INIT_EXT_187: &str = "#include \"ruby.h\"\n\
    void ruby_init_ext _((const char *name, void (*init)(void)));\n\
    void Init_zlib _((void));\n\
    void Init_fcntl _((void));\n\
    void Init_stringio _((void));\n\
    __attribute__((visibility(\"default\"))) int vkrgss_export_probe = 1;\n\
    void Init_ext _((void))\n\
    {\n\
    \x20   ruby_init_ext(\"zlib.so\", Init_zlib);\n\
    \x20   ruby_init_ext(\"fcntl.so\", Init_fcntl);\n\
    \x20   ruby_init_ext(\"stringio.so\", Init_stringio);\n\
    }\n";

/// The extensions of Ruby 1.9.2's test executable with the defines of their `extconf.rb`, as
/// `TEST_EXTENSIONS` of `crates/vkrgss-ruby192-sys/tests/common/mod.rs` lists them.
const EXTENSIONS_192: &[(&str, &[&str])] = &[
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

/// The `Init_ext` of Ruby 1.9.2's test executable, as `write_init_ext` of
/// `crates/vkrgss-ruby192-sys/tests/common/mod.rs` writes it.
fn init_ext_192() -> String {
    let mut declarations = String::new();
    let mut statements = String::new();
    for (name, _) in EXTENSIONS_192 {
        declarations.push_str(&format!("void Init_{name}(void);\n"));
        statements.push_str(&format!("    ruby_init_ext(\"{name}.so\", Init_{name});\n"));
    }
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
    )
}

/// Links Ruby's own test executable for the phone in `scratch` from the archives of the
/// build script, as `build_ruby_executable` of the crate's `tests/common/mod.rs` does, with
/// the zlib and the C library of the NDK.
fn link_executable(
    krate: &Crate,
    variables: &BTreeMap<String, String>,
    scratch: &Path,
) -> Result<PathBuf> {
    let prefix = krate.prefix;
    let compiler = variable(variables, &format!("{prefix}_CC"))?;
    let flags: Vec<&str> = variable(variables, &format!("{prefix}_CFLAGS"))?
        .split(' ')
        .filter(|flag| !flag.is_empty())
        .collect();
    let source = PathBuf::from(variable(variables, &format!("{prefix}_SOURCE_DIR"))?);
    let libraries = PathBuf::from(variable(variables, &format!("{prefix}_LIB_DIR"))?);
    let mut includes = Vec::new();
    if prefix == "VKRGSS_RUBY" {
        for name in ["CONFIG_DIR", "SOURCE_DIR", "COMPAT_DIR", "ONIG_DIR"] {
            includes.push("-I".to_owned());
            includes.push(variable(variables, &format!("{prefix}_{name}"))?.to_owned());
        }
    }
    let compile = |file: &Path, defines: &[&str]| -> Result<PathBuf> {
        let stem = file.file_stem().context("android-test: a file name")?;
        let object = scratch.join(stem).with_extension("o");
        run_host(
            Command::new(compiler)
                .args(&flags)
                .args(&includes)
                .args(defines)
                .arg("-c")
                .arg(file)
                .arg("-o")
                .arg(&object),
        )?;
        Ok(object)
    };

    let init_ext = scratch.join("extinit.c");
    let mut objects = vec![compile(&source.join("main.c"), &[])?];
    let mut archives: Vec<String> = Vec::new();
    let library = |name: &str| libraries.join(name).display().to_string();
    if prefix == "VKRGSS_RUBY" {
        fs::write(&init_ext, INIT_EXT_187).context("android-test: extinit.c")?;
        objects.push(compile(&init_ext, &[])?);
        for extension in ["fcntl", "stringio"] {
            objects.push(compile(
                &source.join(format!("ext/{extension}/{extension}.c")),
                &[],
            )?);
        }
        for name in ["zlib", "static", "onig", "core-math"] {
            archives.push(library(&format!("libvkrgss-ruby-{name}.a")));
        }
    } else {
        fs::write(&init_ext, init_ext_192()).context("android-test: extinit.c")?;
        objects.push(compile(&init_ext, &[])?);
        for (extension, defines) in EXTENSIONS_192 {
            objects.push(compile(
                &source.join(format!("ext/{extension}/{extension}.c")),
                defines,
            )?);
        }
        archives.push("-Wl,--start-group".to_owned());
        for name in ["zlib", "static", "enc"] {
            archives.push(library(&format!("libvkrgss-ruby192-{name}.a")));
        }
        archives.push("-Wl,--end-group".to_owned());
        archives.push(library("libvkrgss-ruby192-core-math.a"));
    }
    let executable = scratch.join("ruby");
    run_host(
        Command::new(compiler)
            .args(&flags)
            .arg("-rdynamic")
            .args(&objects)
            .args(&archives)
            .args(["-lz", "-lm", "-o"])
            .arg(&executable),
    )?;
    Ok(executable)
}

/// adb for the chosen phone.
pub(crate) fn adb(device: Option<&str>) -> Command {
    let mut command = Command::new("adb");
    if let Some(serial) = device {
        command.args(["-s", serial]);
    }
    command.stdin(Stdio::null());
    command
}

/// Runs a shell command on the phone and fails with its output if it fails.
fn shell(device: Option<&str>, script: &str) -> Result<()> {
    run_host(adb(device).args(["shell", script]))
}

/// The serial of the phone adb talks to, which also shows that it reaches one.
fn serial(device: Option<&str>) -> Result<String> {
    let output = adb(device)
        .arg("get-serialno")
        .output()
        .context("android-test: adb could not be started")?;
    let serial = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || serial.is_empty() {
        bail!(
            "android-test: adb reaches no single phone (give --device <SERIAL>):\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(serial)
}

/// Pushes what the tests need into [`DEVICE_DIR`], which is made afresh.
fn push(
    krate: &Crate,
    root: &Path,
    device: Option<&str>,
    binaries: &[Binary],
    executable: &Path,
) -> Result<()> {
    shell(
        device,
        &format!(
            "umask 022 && rm -rf {DEVICE_DIR} && mkdir -p {DEVICE_DIR}/repo/crates {DEVICE_DIR}/repo/third_party \
             {DEVICE_DIR}/bin {DEVICE_DIR}/tmp"
        ),
    )?;
    let pushes: Vec<(Vec<PathBuf>, String)> = vec![
        (
            vec![root.join("crates").join(krate.package)],
            format!("{DEVICE_DIR}/repo/crates/"),
        ),
        (
            krate
                .trees
                .iter()
                .map(|tree| root.join("third_party").join(tree))
                .collect(),
            format!("{DEVICE_DIR}/repo/third_party/"),
        ),
        (
            binaries.iter().map(|binary| binary.path.clone()).collect(),
            format!("{DEVICE_DIR}/bin/"),
        ),
        (vec![executable.to_path_buf()], format!("{DEVICE_DIR}/")),
    ];
    for (sources, destination) in pushes {
        run_host(
            adb(device)
                .args(["push", "-q"])
                .args(&sources)
                .arg(destination),
        )?;
    }
    Ok(())
}

/// The last `test result:` line of the output of a test binary.
fn result_line(output: &str) -> &str {
    output
        .lines()
        .rev()
        .find(|line| line.starts_with("test result:"))
        .unwrap_or("(no test result)")
}

/// Runs every test binary on the phone and writes its output to `logs`. Returns the names of
/// those that failed.
fn run_binaries(
    krate: &Crate,
    device: Option<&str>,
    binaries: &[Binary],
    logs: &Path,
) -> Result<Vec<String>> {
    let mut failed = Vec::new();
    for binary in binaries {
        let file = binary
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .context("android-test: a binary without a UTF-8 name")?;
        let output = adb(device)
            .args(["shell", &device_command(krate, file)])
            .output()
            .context("android-test: adb could not be started")?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let log = logs.join(format!("{}.log", binary.name));
        fs::write(&log, &text).with_context(|| format!("android-test: {}", log.display()))?;
        let passed = output.status.success();
        eprintln!(
            "android-test: {} {}: {}",
            if passed { "ok    " } else { "FAILED" },
            binary.name,
            result_line(&text)
        );
        if !passed {
            failed.push(binary.name.clone());
        }
    }
    Ok(failed)
}

/// Builds, pushes and runs the tests of the crate on the phone, and removes what was pushed.
/// Fails when a test binary fails.
pub(crate) fn run(options: &Options, root: &Path) -> Result<()> {
    let krate = options.krate;
    let device = options.device.as_deref();
    let serial = serial(device)?;
    let (clang, ar) = ndk_tools()?;
    eprintln!(
        "android-test: building the tests of {} for {TARGET}",
        krate.package
    );
    let (binaries, variables) = build(krate, root, &clang, &ar)?;

    let scratch = root
        .join("target/android-test")
        .join(&serial)
        .join(krate.package);
    if scratch.exists() {
        fs::remove_dir_all(&scratch)
            .with_context(|| format!("android-test: {}", scratch.display()))?;
    }
    let logs = scratch.join("logs");
    fs::create_dir_all(&logs).with_context(|| format!("android-test: {}", logs.display()))?;
    let executable = link_executable(krate, &variables, &scratch)?;

    eprintln!(
        "android-test: running {} test binaries on {serial} in {DEVICE_DIR}",
        binaries.len()
    );
    let outcome = push(krate, root, device, &binaries, &executable)
        .and_then(|()| run_binaries(krate, device, &binaries, &logs));
    let removed = shell(device, &format!("rm -rf {DEVICE_DIR}"));
    let failed = outcome?;
    removed?;
    if !failed.is_empty() {
        bail!(
            "android-test: {} of {} test binaries failed on {serial}: {}; their output is in {}",
            failed.len(),
            binaries.len(),
            failed.join(", "),
            logs.display()
        );
    }
    eprintln!(
        "android-test: all {} test binaries of {} passed on {serial}",
        binaries.len(),
        krate.package
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crate_and_a_device_are_read() {
        assert_eq!(
            parse(&["vkrgss-ruby-sys"]).unwrap(),
            Options {
                krate: &CRATES[0],
                device: None
            }
        );
        assert_eq!(
            parse(&["vkrgss-ruby192-sys", "--device", "R5CY21Y3VEV"]).unwrap(),
            Options {
                krate: &CRATES[1],
                device: Some("R5CY21Y3VEV".to_owned())
            }
        );
    }

    #[test]
    fn an_unknown_crate_and_stray_arguments_are_refused() {
        for arguments in [&["vkrgss-binding"][..], &[]] {
            let error = parse(arguments).unwrap_err().to_string();
            for krate in &CRATES {
                assert!(error.contains(&format!("`{}`", krate.package)), "{error}");
                assert!(USAGE.contains(krate.package), "{USAGE}");
            }
        }
        assert!(parse(&["vkrgss-ruby-sys", "--device"]).is_err());
        assert!(parse(&["vkrgss-ruby-sys", "--device", ""]).is_err());
        assert!(parse(&["vkrgss-ruby-sys", "vkrgss-ruby192-sys"]).is_err());
    }

    #[test]
    fn the_tests_get_the_directories_on_the_phone_by_the_names_of_the_host() {
        let command = device_command(&CRATES[1], "upstream-0123");
        for part in [
            "umask 022",
            "CARGO_MANIFEST_DIR=/data/local/tmp/vkrgss-test/repo/crates/vkrgss-ruby192-sys",
            "CARGO_TARGET_TMPDIR=/data/local/tmp/vkrgss-test/tmp",
            "VKRGSS_RUBY192_SOURCE_DIR=/data/local/tmp/vkrgss-test/repo/third_party/ruby-1.9.2",
            "VKRGSS_RUBY192_EXECUTABLE=/data/local/tmp/vkrgss-test/ruby",
            "/data/local/tmp/vkrgss-test/bin/upstream-0123 --test-threads=1",
        ] {
            assert!(command.contains(part), "{part} is not in {command}");
        }
    }

    /// The variables the tests read from the environment are those that the build scripts
    /// give them, so that the one name serves on the host and on the phone.
    #[test]
    fn the_variables_are_those_the_tests_read() {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates");
        for krate in &CRATES {
            let common = crates.join(krate.package).join("tests/common/mod.rs");
            let text = fs::read_to_string(&common).unwrap();
            for (name, _) in device_environment(krate) {
                if name != "HOME" && name != "TMPDIR" {
                    assert!(
                        text.contains(&format!("\"{name}\"")),
                        "{name} in {common:?}"
                    );
                }
            }
            for tree in krate.trees {
                assert!(crates.join("../third_party").join(tree).is_dir(), "{tree}");
            }
        }
    }

    #[test]
    fn the_binaries_and_the_variables_of_the_build_script_are_read_from_the_messages() {
        let messages = [
            r#"{"reason":"build-script-executed","env":[["OTHER_CC","x"]]}"#,
            r#"{"reason":"build-script-executed","env":[["VKRGSS_RUBY_CC","clang"],["VKRGSS_RUBY_CFLAGS","-O2"]]}"#,
            r#"{"reason":"compiler-artifact","profile":{"test":true},"target":{"name":"upstream"},"executable":"/t/upstream-1"}"#,
            r#"{"reason":"compiler-artifact","profile":{"test":false},"target":{"name":"cc"},"executable":null}"#,
            r#"{"reason":"build-finished","success":true}"#,
        ]
        .join("\n");
        let (binaries, variables) = read_messages(&messages, "VKRGSS_RUBY").unwrap();
        assert_eq!(
            binaries,
            [Binary {
                name: "upstream".to_owned(),
                path: PathBuf::from("/t/upstream-1")
            }]
        );
        assert_eq!(variables["VKRGSS_RUBY_CFLAGS"], "-O2");
        assert!(read_messages(&messages, "VKRGSS_RUBY192").is_err());
    }

    /// The extensions of Ruby 1.9.2's executable are those its tests link.
    #[test]
    fn the_extensions_of_ruby_1_9_2_are_those_of_its_tests() {
        let common = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../crates/vkrgss-ruby192-sys/tests/common/mod.rs");
        let text = fs::read_to_string(common).unwrap();
        for (name, defines) in EXTENSIONS_192 {
            assert!(text.contains(&format!("\"{name}\"")), "{name}");
            for define in *defines {
                assert!(text.contains(&format!("{define:?}")), "{define}");
            }
        }
        assert!(INIT_EXT_187.contains("vkrgss_export_probe"));
        assert!(init_ext_192().contains("ruby_init_ext(\"fiber.so\", Init_fiber);"));
    }
}
