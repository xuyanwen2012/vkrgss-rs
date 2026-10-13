//! The tests under collector stress: one command, run by hand and not by the gate.
//!
//! With `VKXP_GC_STRESS=<n>` the embedded interpreter collects at every n-th allocation, so
//! a value that C or Rust code keeps alive only through a pointer into it is lost at once
//! instead of on some other machine. The last step is the same for the second interpreter,
//! Ruby 1.9.2, whose own `GC.stress` does the collecting. See `docs/development.md`.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};

/// The variable `vkxp-ruby-sys` reads when it starts an interpreter.
const VARIABLE: &str = "VKXP_GC_STRESS";

/// The variable through which the stressed test of `vkxp-ruby-sys` gets its skip list.
const SKIP_VARIABLE: &str = "VKXP_GC_STRESS_SKIP";

/// The `n` of the upstream suites in the sanitized build: the smallest with which the step
/// ends within about half an hour on the development machine.
const SUITES_EVERY: u32 = 32;

/// The `n` of the tests of the binding.
const BINDING_EVERY: u32 = 1;

/// The `n` of the tests of the executable that start a guest.
const GUEST_EVERY: u32 = 1;

/// The tests left out, each with its reason.
const SKIP_LIST: &str = include_str!("stress.skip");

/// One test of the skip list: the step it belongs to and its name there.
#[derive(Debug, PartialEq, Eq)]
struct Skipped {
    step: String,
    test: String,
    reason: String,
}

/// Parses the skip list: entries separated by blank lines, each made of `#` lines that give
/// the reason and of `<step> <test>` lines. A block of comment lines alone is not an entry.
fn parse_skip_list(text: &str) -> Result<Vec<Skipped>> {
    let mut skipped = Vec::new();
    for block in text.split("\n\n") {
        let mut reason = Vec::new();
        let mut tests = Vec::new();
        for line in block.lines().map(str::trim) {
            if let Some(comment) = line.strip_prefix('#') {
                reason.push(comment.trim());
            } else if !line.is_empty() {
                let Some((step, test)) = line.split_once(' ') else {
                    bail!("stress: skip list line `{line}` is not `<step> <test>`");
                };
                tests.push((step, test.trim()));
            }
        }
        let reason = reason.join(" ").trim().to_owned();
        for (step, test) in tests {
            skipped.push(Skipped {
                step: step.to_owned(),
                test: test.to_owned(),
                reason: reason.clone(),
            });
        }
    }
    Ok(skipped)
}

/// One run of tests: a name used in reports, the arguments of cargo and the environment
/// variables set for it.
struct Step {
    name: &'static str,
    args: Vec<String>,
    env: Vec<(&'static str, String)>,
}

/// The name of the step that runs the slow files of the stressed selection of
/// `vkxp-ruby192-sys`. The test reads the skip list itself, for the lines of this step.
const SUITES_192: &str = "suites-1.9.2";

/// The four runs. Everything else of the environment is inherited, so the restriction of
/// the Vulkan loader (`VK_LOADER_DRIVERS_SELECT`) that the tests of the executable ask for
/// reaches them.
fn steps(skipped: &[Skipped]) -> [Step; 4] {
    let tests_of = |step: &str| -> Vec<&str> {
        skipped
            .iter()
            .filter(|entry| entry.step == step)
            .map(|entry| entry.test.as_str())
            .collect()
    };
    // `--exact` makes `--skip` leave out the one test of that name and no other.
    let cargo_test = |first: &[&str], step: &str| -> Vec<String> {
        let mut args: Vec<String> = first.iter().map(|&arg| arg.to_owned()).collect();
        let tests = tests_of(step);
        if !tests.is_empty() {
            args.extend(["--".to_owned(), "--exact".to_owned()]);
            for test in tests {
                args.extend(["--skip".to_owned(), test.to_owned()]);
            }
        }
        args
    };
    [
        // The two ignored tests of `tests/sanitized.rs` that run under the switch: the
        // upstream suites, and the check that a lost value is reported at all.
        Step {
            name: "suites",
            args: [
                "test",
                "-p",
                "vkxp-ruby-sys",
                "--locked",
                "--test",
                "sanitized",
                "--",
                "--ignored",
                "under_collector_stress",
            ]
            .map(str::to_owned)
            .to_vec(),
            env: vec![
                (VARIABLE, SUITES_EVERY.to_string()),
                (SKIP_VARIABLE, tests_of("suites").join(" ")),
            ],
        },
        Step {
            name: "binding",
            args: cargo_test(
                &["test", "-p", "vkxp-binding", "--locked", "--no-fail-fast"],
                "binding",
            ),
            env: vec![(VARIABLE, BINDING_EVERY.to_string())],
        },
        Step {
            name: "guest",
            args: cargo_test(
                &["test", "-p", "vkxp", "--locked", "--test", "end_to_end"],
                "guest",
            ),
            env: vec![(VARIABLE, GUEST_EVERY.to_string())],
        },
        // The files of Ruby 1.9.2's own suite that are too slow under `GC.stress = true` for
        // the workspace tests, where the quick ones run. The ordinary build: no sanitizer,
        // and no variable, because every test process switches the stress on itself.
        Step {
            name: SUITES_192,
            args: [
                "test",
                "-p",
                "vkxp-ruby192-sys",
                "--locked",
                "--test",
                "upstream_stress",
                "--",
                "--ignored",
            ]
            .map(str::to_owned)
            .to_vec(),
            env: Vec::new(),
        },
    ]
}

/// Runs every step, also after one failed, and names all that failed. Each step's duration
/// is printed when it ends, and all of them once more at the end.
pub(crate) fn run(root: &Path) -> Result<()> {
    let skipped = parse_skip_list(SKIP_LIST)?;
    for entry in &skipped {
        eprintln!(
            "stress: step `{}` leaves out {}: {}",
            entry.step, entry.test, entry.reason
        );
    }
    let mut durations = Vec::new();
    let mut failed = Vec::new();
    for step in steps(&skipped) {
        let command_line = format!("cargo {}", step.args.join(" "));
        let variables: Vec<String> = step
            .env
            .iter()
            .map(|(name, value)| format!("{name}='{value}'"))
            .collect();
        eprintln!(
            "stress: running step `{}` ({} {command_line})",
            step.name,
            variables.join(" ")
        );
        let started = Instant::now();
        let status = Command::new(crate::cargo())
            .args(&step.args)
            .envs(step.env.iter().map(|(name, value)| (name, value)))
            .current_dir(root)
            .status();
        let elapsed = started.elapsed();
        durations.push((step.name, elapsed));
        eprintln!(
            "stress: step `{}` took {}",
            step.name,
            format_duration(elapsed)
        );
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => failed.push(format!(
                "`{}` ({command_line} ended with {status})",
                step.name
            )),
            Err(error) => failed.push(format!("`{}` (could not start: {error})", step.name)),
        }
    }
    for (name, duration) in &durations {
        eprintln!("stress: {name:<16} {:>8}", format_duration(*duration));
    }
    if failed.is_empty() {
        eprintln!("stress: all steps passed");
        Ok(())
    } else {
        Err(anyhow!("stress: FAILED at step {}", failed.join(", ")))
    }
}

fn format_duration(duration: Duration) -> String {
    format!("{:.1} s", duration.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_are_the_four_runs_in_order_three_of_them_under_the_switch() {
        let steps = steps(&[]);
        let names: Vec<&str> = steps.iter().map(|step| step.name).collect();
        assert_eq!(names, ["suites", "binding", "guest", "suites-1.9.2"]);
        for (step, every) in steps.iter().zip([SUITES_EVERY, BINDING_EVERY, GUEST_EVERY]) {
            assert!(every >= 1, "{}", step.name);
            assert!(
                step.env.contains(&(VARIABLE, every.to_string())),
                "{}",
                step.name
            );
            assert!(step.args.contains(&"--locked".to_owned()), "{}", step.name);
        }
        assert_eq!(
            steps[1].args,
            ["test", "-p", "vkxp-binding", "--locked", "--no-fail-fast"]
        );
        assert_eq!(
            steps[2].args,
            ["test", "-p", "vkxp", "--locked", "--test", "end_to_end"]
        );
    }

    /// The step of Ruby 1.9.2 runs the ignored test of the stressed selection in the
    /// ordinary build, without the switch of the other interpreter.
    #[test]
    fn the_step_of_ruby_1_9_2_runs_the_slow_files_of_the_stressed_selection() {
        let [.., suites_192] = steps(&[]);
        assert_eq!(
            suites_192.args,
            [
                "test",
                "-p",
                "vkxp-ruby192-sys",
                "--locked",
                "--test",
                "upstream_stress",
                "--",
                "--ignored"
            ]
        );
        assert!(suites_192.env.is_empty());
    }

    /// The files that did not finish under stress in the study are named for that step.
    #[test]
    fn the_committed_skip_list_names_the_unfinished_files_of_ruby_1_9_2() {
        let skipped = parse_skip_list(SKIP_LIST).unwrap();
        for file in ["test_integer_comb.rb", "test_m17n_comb.rb", "test_yield.rb"] {
            let entry = skipped
                .iter()
                .find(|entry| entry.step == SUITES_192 && entry.test == format!("ruby/{file}"))
                .unwrap_or_else(|| panic!("{file} is not named for the step"));
            assert!(
                entry.reason.contains("did not finish in 14 minutes"),
                "{entry:?}"
            );
        }
    }

    #[test]
    fn the_suites_step_runs_the_ignored_stressed_tests_of_the_sanitized_build() {
        let [suites, ..] = steps(&[]);
        assert_eq!(
            suites.args,
            [
                "test",
                "-p",
                "vkxp-ruby-sys",
                "--locked",
                "--test",
                "sanitized",
                "--",
                "--ignored",
                "under_collector_stress"
            ]
        );
    }

    #[test]
    fn skipped_tests_reach_their_step_and_no_other() {
        let skipped = parse_skip_list(
            "# About the file.\n\n# Measures time,\n# so it fails.\nsuites ruby/test_a.rb#test_b\n\
             binding module::test_c\n\n# Another reason.\nsuites sample\nguest test_d\n",
        )
        .unwrap();
        assert_eq!(skipped.len(), 4);
        assert_eq!(skipped[0].reason, "Measures time, so it fails.");
        assert_eq!(skipped[1].reason, "Measures time, so it fails.");
        assert_eq!(skipped[3].reason, "Another reason.");

        let [suites, binding, guest, _] = steps(&skipped);
        assert!(
            suites
                .env
                .contains(&(SKIP_VARIABLE, "ruby/test_a.rb#test_b sample".to_owned()))
        );
        assert_eq!(
            binding.args[5..],
            ["--", "--exact", "--skip", "module::test_c"]
        );
        assert_eq!(guest.args[6..], ["--", "--exact", "--skip", "test_d"]);
    }

    #[test]
    fn a_skip_list_line_without_a_step_is_refused() {
        assert!(parse_skip_list("# Reason.\nruby/test_a.rb\n").is_err());
    }

    #[test]
    fn the_committed_skip_list_names_known_steps_and_gives_reasons() {
        for entry in parse_skip_list(SKIP_LIST).unwrap() {
            assert!(
                ["suites", "binding", "guest", SUITES_192].contains(&entry.step.as_str()),
                "{entry:?}"
            );
            assert!(!entry.reason.is_empty(), "{entry:?}");
        }
    }
}
