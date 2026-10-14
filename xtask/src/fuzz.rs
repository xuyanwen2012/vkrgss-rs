//! The fuzz targets of the embedded interpreter: one command, run by hand and not by the
//! gate.
//!
//! A target feeds arbitrary bytes to `Marshal.load`, to the parser or to the regular
//! expression engine under libFuzzer and the sanitizers. The targets, their seeds and the
//! test that builds and runs one are in `crates/vkrgss-ruby-sys`; this command checks its
//! arguments and the compiler and runs that test. See `docs/development.md`.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// The targets, each the name of a C file under `crates/vkrgss-ruby-sys/fuzz/`.
const TARGETS: [&str; 3] = ["marshal", "parse", "regex"];

/// The seconds a target runs for unless `--seconds` says otherwise.
const DEFAULT_SECONDS: u64 = 60;

/// The variables through which the test of `vkrgss-ruby-sys` gets the target and the time.
const TARGET_VARIABLE: &str = "VKRGSS_FUZZ_TARGET";
const SECONDS_VARIABLE: &str = "VKRGSS_FUZZ_SECONDS";

/// The compiler of the instrumented builds, as in the tests of `vkrgss-ruby-sys`.
const COMPILER: &str = "clang";

pub(crate) const USAGE: &str = "usage: cargo xtask fuzz <marshal|parse|regex> [--seconds N]";

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    target: &'static str,
    seconds: u64,
}

/// Parses the arguments after `fuzz`.
pub(crate) fn parse(arguments: &[&str]) -> Result<Options> {
    let (name, rest) = match arguments {
        [name, rest @ ..] => (*name, rest),
        [] => bail!("fuzz: no target given; the targets are {}", targets()),
    };
    let Some(target) = TARGETS.iter().copied().find(|target| *target == name) else {
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
    TARGETS.map(|target| format!("`{target}`")).join(", ")
}

/// The arguments of cargo that run the target: the ignored test of `tests/fuzz.rs`, with
/// its output passed through, because the fuzzer reports its progress and its findings
/// there.
fn cargo_arguments() -> [&'static str; 11] {
    [
        "test",
        "-p",
        "vkrgss-ruby-sys",
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

/// The environment of that test.
fn environment(options: &Options) -> [(&'static str, String); 2] {
    [
        (TARGET_VARIABLE, options.target.to_owned()),
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
    check_compiler(root)?;
    let variables: Vec<String> = environment(options)
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    eprintln!(
        "fuzz: running `{}` for {} s ({} cargo {})",
        options.target,
        options.seconds,
        variables.join(" "),
        cargo_arguments().join(" ")
    );
    let status = Command::new(crate::cargo())
        .args(cargo_arguments())
        .envs(environment(options))
        .current_dir(root)
        .status()
        .context("fuzz: cargo could not be started")?;
    if !status.success() {
        bail!(
            "fuzz: the target `{}` did not end cleanly ({status}); a finding is reported \
             above and its input is under target/tmp/fuzz/{}/artifacts/",
            options.target,
            options.target
        );
    }
    eprintln!("fuzz: `{}` ended without a finding", options.target);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_runs_for_a_minute_unless_told_otherwise() {
        assert_eq!(
            parse(&["marshal"]).unwrap(),
            Options {
                target: "marshal",
                seconds: 60
            }
        );
        assert_eq!(
            parse(&["regex", "--seconds", "30"]).unwrap(),
            Options {
                target: "regex",
                seconds: 30
            }
        );
    }

    #[test]
    fn an_unknown_target_is_refused_with_the_names_of_the_three() {
        for arguments in [&["nothing"][..], &[]] {
            let error = parse(arguments).unwrap_err().to_string();
            for target in TARGETS {
                assert!(error.contains(&format!("`{target}`")), "{error}");
            }
        }
    }

    #[test]
    fn a_wrong_time_and_stray_arguments_are_refused() {
        assert!(parse(&["parse", "--seconds", "0"]).is_err());
        assert!(parse(&["parse", "--seconds", "soon"]).is_err());
        assert!(parse(&["parse", "--seconds"]).is_err());
        assert!(parse(&["parse", "regex"]).is_err());
    }

    #[test]
    fn the_run_is_the_ignored_test_of_the_interpreter_crate_with_both_variables() {
        let arguments = cargo_arguments();
        assert_eq!(
            arguments[..6],
            [
                "test",
                "-p",
                "vkrgss-ruby-sys",
                "--locked",
                "--test",
                "fuzz"
            ]
        );
        assert!(arguments.contains(&"--ignored") && arguments.contains(&"--nocapture"));
        let options = parse(&["parse", "--seconds", "5"]).unwrap();
        assert_eq!(
            environment(&options),
            [
                ("VKRGSS_FUZZ_TARGET", "parse".to_owned()),
                ("VKRGSS_FUZZ_SECONDS", "5".to_owned())
            ]
        );
    }

    #[test]
    fn every_target_has_its_file_and_its_seeds() {
        let fuzz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/vkrgss-ruby-sys/fuzz");
        for target in TARGETS {
            assert!(fuzz.join(format!("{target}.c")).is_file(), "{target}");
            assert!(fuzz.join("seeds").join(target).is_dir(), "{target}");
        }
    }
}
