//! The fuzz targets of the embedded interpreters and of the file readers: one command, run by
//! hand and not by the gate.
//!
//! A target of an interpreter feeds arbitrary bytes to `Marshal.load`, to the parser or to the
//! regular expression engine under libFuzzer and the sanitizers. The targets, their seeds and
//! the test that builds and runs one are in the crate of the interpreter, `vkrgss-ruby-sys`
//! for Ruby 1.8.7 and `vkrgss-ruby192-sys` for Ruby 1.9.2; this command checks its arguments
//! and the compiler and runs that test.
//!
//! A target of a file reader feeds arbitrary bytes to the JPEG decoder, the BMP decoder or the
//! reader of encrypted archives. It is a binary of the package `fuzz/`, a workspace of its
//! own, which this command builds with libFuzzer's coverage instrumentation and runs under
//! `target/tmp/fuzz/<target>/`. See `docs/development.md`.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// One target: its name and what runs it.
#[derive(Debug, PartialEq, Eq)]
struct Target {
    name: &'static str,
    kind: Kind,
}

/// What a target is.
#[derive(Debug, PartialEq, Eq)]
enum Kind {
    /// A target of an interpreter, run by the test of its crate, which knows it by its name.
    Interpreter { package: &'static str },
    /// A target of a file reader: the binary of its name in the package `fuzz/`, which starts
    /// from the seeds in `seeds`, a directory of the repository.
    Reader { seeds: &'static str },
}

const fn interpreter(name: &'static str, package: &'static str) -> Target {
    Target {
        name,
        kind: Kind::Interpreter { package },
    }
}

const fn reader(name: &'static str, seeds: &'static str) -> Target {
    Target {
        name,
        kind: Kind::Reader { seeds },
    }
}

/// The targets. Those of Ruby 1.8.7 are each the name of a C file under
/// `crates/vkrgss-ruby-sys/fuzz/`; those of Ruby 1.9.2 add `192` to the name of a C file
/// under `crates/vkrgss-ruby192-sys/fuzz/`; those of the file readers are each the name of a
/// binary of `fuzz/`.
const TARGETS: [Target; 9] = [
    interpreter("marshal", "vkrgss-ruby-sys"),
    interpreter("parse", "vkrgss-ruby-sys"),
    interpreter("regex", "vkrgss-ruby-sys"),
    interpreter("marshal192", "vkrgss-ruby192-sys"),
    interpreter("parse192", "vkrgss-ruby192-sys"),
    interpreter("regex192", "vkrgss-ruby192-sys"),
    reader("jpeg", "crates/vkrgss-image/testdata/jpeg"),
    reader("bmp", "fuzz/seeds/bmp"),
    reader("rgssad", "fuzz/seeds/rgssad"),
];

/// The manifest of the package of the targets of the file readers.
const READERS_MANIFEST: &str = "fuzz/Cargo.toml";

/// Where the instrumented build of those targets goes, apart from the build of the workspace.
const READERS_TARGET_DIR: &str = "target/fuzz";

/// The flags of rustc that add libFuzzer's coverage instrumentation, as `cargo fuzz` passes
/// them without a sanitizer, which the stable toolchain does not have.
const COVERAGE_FLAGS: [&str; 7] = [
    "-Cpasses=sancov-module",
    "-Cllvm-args=-sanitizer-coverage-level=4",
    "-Cllvm-args=-sanitizer-coverage-inline-8bit-counters",
    "-Cllvm-args=-sanitizer-coverage-pc-table",
    "-Cllvm-args=-sanitizer-coverage-trace-compares",
    "--cfg",
    "fuzzing",
];

/// The largest input of a target of a file reader, in bytes: several times the largest seed.
const READER_MAX_LEN: u32 = 65536;

/// The memory a target of a file reader may take before libFuzzer counts it a finding: above
/// the most the largest image may need, which the target checks itself.
const READER_RSS_LIMIT_MB: u32 = 8192;

/// The seconds a target runs for unless `--seconds` says otherwise.
const DEFAULT_SECONDS: u64 = 60;

/// The variables through which the test of the crate gets the target and the time.
const TARGET_VARIABLE: &str = "VKRGSS_FUZZ_TARGET";
const SECONDS_VARIABLE: &str = "VKRGSS_FUZZ_SECONDS";

/// The compiler of the instrumented builds, as in the tests of both crates.
const COMPILER: &str = "clang";

pub(crate) const USAGE: &str = "usage: cargo xtask fuzz \
     <marshal|parse|regex|marshal192|parse192|regex192|jpeg|bmp|rgssad> [--seconds N]";

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    target: &'static Target,
    seconds: u64,
}

/// Parses the arguments after `fuzz`.
pub(crate) fn parse(arguments: &[&str]) -> Result<Options> {
    let (name, rest) = match arguments {
        [name, rest @ ..] => (*name, rest),
        [] => bail!("fuzz: no target given; the targets are {}", targets()),
    };
    let Some(target) = TARGETS.iter().find(|target| target.name == name) else {
        bail!("fuzz: no target `{name}`; the targets are {}", targets());
    };
    let seconds = match rest {
        [] => DEFAULT_SECONDS,
        ["--seconds", number] => match number.parse() {
            Ok(seconds) if seconds >= 1 => seconds,
            _ => bail!("fuzz: `--seconds {number}` is not a number of 1 or more"),
        },
        _ => bail!("fuzz: unexpected arguments `{}`", rest.join(" ")),
    };
    Ok(Options { target, seconds })
}

fn targets() -> String {
    TARGETS
        .iter()
        .map(|target| format!("`{}`", target.name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The arguments of cargo that run the target of an interpreter in `package`: the ignored
/// test of `tests/fuzz.rs` of its crate, with its output passed through, because the fuzzer
/// reports its progress and its findings there.
fn cargo_arguments(package: &'static str) -> [&'static str; 11] {
    [
        "test",
        "-p",
        package,
        "--locked",
        "--test",
        "fuzz",
        "--",
        "--ignored",
        "--exact",
        "--nocapture",
        "the_chosen_target_runs_without_a_finding",
    ]
}

/// The arguments of cargo that build the target of a file reader for `host`. The target is
/// named so that the flags of [`COVERAGE_FLAGS`] reach the code of the target and not the
/// build scripts, which libFuzzer's runtime is not linked into.
fn reader_build_arguments(name: &'static str, host: &str) -> Vec<String> {
    [
        "build",
        "--manifest-path",
        READERS_MANIFEST,
        "--locked",
        "--release",
        "--target",
        host,
        "--bin",
        name,
    ]
    .map(str::to_owned)
    .to_vec()
}

/// The arguments of libFuzzer for a run of `seconds` of a target of a file reader, under
/// `dir`: the corpus the run grows in `corpus/`, then the seeds, and the inputs of findings
/// in `artifacts/`.
fn reader_run_arguments(seconds: u64, dir: &Path, seeds: &Path) -> Vec<String> {
    vec![
        format!("-max_total_time={seconds}"),
        format!("-max_len={READER_MAX_LEN}"),
        format!("-rss_limit_mb={READER_RSS_LIMIT_MB}"),
        format!("-artifact_prefix={}/", dir.join("artifacts").display()),
        dir.join("corpus").display().to_string(),
        seeds.display().to_string(),
    ]
}

/// The target triple of the host, as rustc names it.
fn host_triple() -> Result<String> {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let output = Command::new(&rustc)
        .arg("-vV")
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("fuzz: `{rustc}` could not be started"))?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .context("fuzz: rustc did not name the host")
}

/// The environment of that test.
fn environment(options: &Options) -> [(&'static str, String); 2] {
    [
        (TARGET_VARIABLE, options.target.name.to_owned()),
        (SECONDS_VARIABLE, options.seconds.to_string()),
    ]
}

/// Links an empty fuzz target with the sanitizers, to say what is missing before the
/// interpreter is compiled.
fn check_compiler(root: &Path) -> Result<()> {
    let needs = format!(
        "fuzz: the targets need `{COMPILER}` with its sanitizer runtimes and libFuzzer \
         (the package `compiler-rt` on Fedora, `libclang-rt-dev` on Debian and Ubuntu)"
    );
    let dir = root.join("target/tmp/fuzz");
    std::fs::create_dir_all(&dir).with_context(|| format!("fuzz: {}", dir.display()))?;
    let source = dir.join("probe.c");
    std::fs::write(
        &source,
        "int LLVMFuzzerTestOneInput(const unsigned char *data, unsigned long size) \
         { (void)data; (void)size; return 0; }\n",
    )
    .with_context(|| format!("fuzz: {}", source.display()))?;
    let output = Command::new(COMPILER)
        .arg("-fsanitize=fuzzer,address,undefined")
        .arg(&source)
        .arg("-o")
        .arg(dir.join("probe"))
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("{needs}; `{COMPILER}` could not be started"))?;
    if !output.status.success() {
        bail!(
            "{needs}; an empty target did not link:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Builds the target and runs it for the given time. Fails when the target reports a
/// finding.
pub(crate) fn run(options: &Options, root: &Path) -> Result<()> {
    match options.target.kind {
        Kind::Interpreter { package } => run_interpreter(options, package, root),
        Kind::Reader { seeds } => run_reader(options, seeds, root),
    }
}

/// Builds the target of a file reader and runs it under `target/tmp/fuzz/<target>/`.
fn run_reader(options: &Options, seeds: &str, root: &Path) -> Result<()> {
    let name = options.target.name;
    let host = host_triple()?;
    let target_dir = root.join(READERS_TARGET_DIR);
    eprintln!("fuzz: building `{name}` with the coverage instrumentation of libFuzzer");
    let status = Command::new(crate::cargo())
        .args(reader_build_arguments(name, &host))
        .env("CARGO_TARGET_DIR", &target_dir)
        .env("CARGO_ENCODED_RUSTFLAGS", COVERAGE_FLAGS.join("\x1f"))
        .env_remove("RUSTFLAGS")
        .current_dir(root)
        .status()
        .context("fuzz: cargo could not be started")?;
    if !status.success() {
        bail!("fuzz: the target `{name}` could not be built ({status})");
    }
    let dir = root.join("target/tmp/fuzz").join(name);
    for part in ["corpus", "artifacts"] {
        let path = dir.join(part);
        std::fs::create_dir_all(&path).with_context(|| format!("fuzz: {}", path.display()))?;
    }
    let arguments = reader_run_arguments(options.seconds, &dir, &root.join(seeds));
    eprintln!(
        "fuzz: running `{name}` for {} s ({})",
        options.seconds,
        arguments.join(" ")
    );
    let binary = target_dir.join(&host).join("release").join(name);
    let status = Command::new(&binary)
        .args(&arguments)
        .current_dir(&dir)
        .status()
        .with_context(|| format!("fuzz: {} could not be started", binary.display()))?;
    if !status.success() {
        bail!(
            "fuzz: the target `{name}` did not end cleanly ({status}); a finding is reported \
             above and its input is under target/tmp/fuzz/{name}/artifacts/"
        );
    }
    eprintln!("fuzz: `{name}` ended without a finding");
    Ok(())
}

/// Builds the target of an interpreter, through the test of `package`, and runs it.
fn run_interpreter(options: &Options, package: &'static str, root: &Path) -> Result<()> {
    check_compiler(root)?;
    let variables: Vec<String> = environment(options)
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    eprintln!(
        "fuzz: running `{}` for {} s ({} cargo {})",
        options.target.name,
        options.seconds,
        variables.join(" "),
        cargo_arguments(package).join(" ")
    );
    let status = Command::new(crate::cargo())
        .args(cargo_arguments(package))
        .envs(environment(options))
        .current_dir(root)
        .status()
        .context("fuzz: cargo could not be started")?;
    if !status.success() {
        bail!(
            "fuzz: the target `{}` did not end cleanly ({status}); a finding is reported \
             above and its input is under target/tmp/fuzz/{}/artifacts/",
            options.target.name,
            options.target.name
        );
    }
    eprintln!("fuzz: `{}` ended without a finding", options.target.name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(name: &str) -> &'static Target {
        TARGETS.iter().find(|target| target.name == name).unwrap()
    }

    #[test]
    fn a_target_runs_for_a_minute_unless_told_otherwise() {
        assert_eq!(
            parse(&["marshal"]).unwrap(),
            Options {
                target: target("marshal"),
                seconds: 60
            }
        );
        assert_eq!(
            parse(&["regex192", "--seconds", "30"]).unwrap(),
            Options {
                target: target("regex192"),
                seconds: 30
            }
        );
    }

    #[test]
    fn an_unknown_target_is_refused_with_the_names_of_the_nine() {
        assert_eq!(TARGETS.len(), 9);
        for arguments in [&["nothing"][..], &[]] {
            let error = parse(arguments).unwrap_err().to_string();
            for target in &TARGETS {
                assert!(error.contains(&format!("`{}`", target.name)), "{error}");
            }
        }
        for target in &TARGETS {
            assert!(USAGE.contains(target.name), "{USAGE}");
        }
    }

    #[test]
    fn a_wrong_time_and_stray_arguments_are_refused() {
        assert!(parse(&["parse", "--seconds", "0"]).is_err());
        assert!(parse(&["parse", "--seconds", "soon"]).is_err());
        assert!(parse(&["parse", "--seconds"]).is_err());
        assert!(parse(&["parse", "regex"]).is_err());
        assert!(parse(&["parse193"]).is_err());
    }

    #[test]
    fn the_run_is_the_ignored_test_of_the_interpreter_crate_with_both_variables() {
        for (name, package) in [
            ("parse", "vkrgss-ruby-sys"),
            ("parse192", "vkrgss-ruby192-sys"),
        ] {
            assert_eq!(target(name).kind, Kind::Interpreter { package });
            let arguments = cargo_arguments(package);
            assert_eq!(
                arguments[..6],
                ["test", "-p", package, "--locked", "--test", "fuzz"]
            );
            assert!(arguments.contains(&"--ignored") && arguments.contains(&"--nocapture"));
        }
        let options = parse(&["parse192", "--seconds", "5"]).unwrap();
        assert_eq!(
            environment(&options),
            [
                ("VKRGSS_FUZZ_TARGET", "parse192".to_owned()),
                ("VKRGSS_FUZZ_SECONDS", "5".to_owned())
            ]
        );
    }

    #[test]
    fn every_target_has_its_file_and_its_seeds() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let manifest = std::fs::read_to_string(root.join(READERS_MANIFEST)).unwrap();
        for target in &TARGETS {
            match target.kind {
                Kind::Interpreter { package } => {
                    let file = target.name.strip_suffix("192").unwrap_or(target.name);
                    let fuzz = root.join("crates").join(package).join("fuzz");
                    assert!(fuzz.join(format!("{file}.c")).is_file(), "{}", target.name);
                    assert!(fuzz.join("seeds").join(file).is_dir(), "{}", target.name);
                }
                Kind::Reader { seeds } => {
                    let source = format!("src/bin/{}.rs", target.name);
                    assert!(root.join("fuzz").join(&source).is_file(), "{source}");
                    assert!(
                        manifest
                            .contains(&format!("name = \"{}\"\npath = \"{source}\"", target.name)),
                        "{source} is not a binary of {READERS_MANIFEST}"
                    );
                    let seeds = std::fs::read_dir(root.join(seeds)).unwrap();
                    assert!(seeds.count() > 1, "{}", target.name);
                }
            }
        }
    }

    #[test]
    fn the_targets_of_ruby_1_9_2_are_in_its_crate() {
        for target in &TARGETS {
            if let Kind::Interpreter { package } = target.kind {
                assert_eq!(
                    package == "vkrgss-ruby192-sys",
                    target.name.ends_with("192"),
                    "{}",
                    target.name
                );
            }
        }
    }

    #[test]
    fn a_target_of_a_file_reader_is_built_instrumented_and_run_from_its_seeds() {
        let arguments = reader_build_arguments("jpeg", "x86_64-unknown-linux-gnu");
        assert_eq!(
            arguments,
            [
                "build",
                "--manifest-path",
                "fuzz/Cargo.toml",
                "--locked",
                "--release",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--bin",
                "jpeg"
            ]
        );
        assert!(COVERAGE_FLAGS.contains(&"-Cpasses=sancov-module"));
        let dir = Path::new("/r/target/tmp/fuzz/jpeg");
        let run = reader_run_arguments(30, dir, Path::new("/r/seeds"));
        assert_eq!(
            run,
            [
                "-max_total_time=30",
                "-max_len=65536",
                "-rss_limit_mb=8192",
                "-artifact_prefix=/r/target/tmp/fuzz/jpeg/artifacts/",
                "/r/target/tmp/fuzz/jpeg/corpus",
                "/r/seeds"
            ]
        );
        assert!(host_triple().unwrap().contains('-'));
    }
}
