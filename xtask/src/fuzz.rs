//! The fuzz targets of the embedded interpreters: one command, run by hand and not by the
//! gate.
//!
//! A target feeds arbitrary bytes to `Marshal.load`, to the parser or to the regular
//! expression engine under libFuzzer and the sanitizers. The targets, their seeds and the
//! test that builds and runs one are in the crate of the interpreter, `vkrgss-ruby-sys` for
//! Ruby 1.8.7 and `vkrgss-ruby192-sys` for Ruby 1.9.2; this command checks its arguments and
//! the compiler and runs that test. See `docs/development.md`.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// One target: its name, which the test of its crate knows it by, and that crate.
#[derive(Debug, PartialEq, Eq)]
struct Target {
    name: &'static str,
    package: &'static str,
}

/// The targets. Those of Ruby 1.8.7 are each the name of a C file under
/// `crates/vkrgss-ruby-sys/fuzz/`; those of Ruby 1.9.2 add `192` to the name of a C file
/// under `crates/vkrgss-ruby192-sys/fuzz/`.
const TARGETS: [Target; 6] = [
    Target {
        name: "marshal",
        package: "vkrgss-ruby-sys",
    },
    Target {
        name: "parse",
        package: "vkrgss-ruby-sys",
    },
    Target {
        name: "regex",
        package: "vkrgss-ruby-sys",
    },
    Target {
        name: "marshal192",
        package: "vkrgss-ruby192-sys",
    },
    Target {
        name: "parse192",
        package: "vkrgss-ruby192-sys",
    },
    Target {
        name: "regex192",
        package: "vkrgss-ruby192-sys",
    },
];

/// The seconds a target runs for unless `--seconds` says otherwise.
const DEFAULT_SECONDS: u64 = 60;

/// The variables through which the test of the crate gets the target and the time.
const TARGET_VARIABLE: &str = "VKRGSS_FUZZ_TARGET";
const SECONDS_VARIABLE: &str = "VKRGSS_FUZZ_SECONDS";

/// The compiler of the instrumented builds, as in the tests of both crates.
const COMPILER: &str = "clang";

pub(crate) const USAGE: &str =
    "usage: cargo xtask fuzz <marshal|parse|regex|marshal192|parse192|regex192> [--seconds N]";

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

/// The arguments of cargo that run the target: the ignored test of `tests/fuzz.rs` of its
/// crate, with its output passed through, because the fuzzer reports its progress and its
/// findings there.
fn cargo_arguments(target: &Target) -> [&'static str; 11] {
    [
        "test",
        "-p",
        target.package,
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
        cargo_arguments(options.target).join(" ")
    );
    let status = Command::new(crate::cargo())
        .args(cargo_arguments(options.target))
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
    fn an_unknown_target_is_refused_with_the_names_of_the_six() {
        for arguments in [&["nothing"][..], &[]] {
            let error = parse(arguments).unwrap_err().to_string();
            for target in &TARGETS {
                assert!(error.contains(&format!("`{}`", target.name)), "{error}");
            }
        }
        for name in ["marshal192", "parse192", "regex192"] {
            assert!(USAGE.contains(name), "{USAGE}");
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
            let arguments = cargo_arguments(target(name));
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
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates");
        for target in &TARGETS {
            let file = target.name.strip_suffix("192").unwrap_or(target.name);
            let fuzz = crates.join(target.package).join("fuzz");
            assert!(fuzz.join(format!("{file}.c")).is_file(), "{}", target.name);
            assert!(fuzz.join("seeds").join(file).is_dir(), "{}", target.name);
        }
    }

    #[test]
    fn the_targets_of_ruby_1_9_2_are_in_its_crate() {
        for target in &TARGETS {
            assert_eq!(
                target.package == "vkrgss-ruby192-sys",
                target.name.ends_with("192"),
                "{}",
                target.name
            );
        }
    }
}
