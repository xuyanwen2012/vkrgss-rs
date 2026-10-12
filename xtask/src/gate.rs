//! The merge gate: one command that every change must pass.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};

/// One check of the gate: a name used in reports, the command behind it and the environment
/// variables it sets for that command.
struct Step {
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
}

fn steps() -> [Step; 6] {
    [
        Step {
            name: "fmt",
            program: "cargo",
            args: &["fmt", "--all", "--check"],
            env: &[],
        },
        Step {
            name: "clippy",
            program: "cargo",
            args: &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            env: &[],
        },
        // `--no-fail-fast`: every test binary runs, so one run shows all that fail.
        Step {
            name: "test",
            program: "cargo",
            args: &["test", "--workspace", "--locked", "--no-fail-fast"],
            env: &[],
        },
        // The Vulkan tests once more without the validation layer, as players run: the layer
        // wraps handles in unique numbers and so hides a fault that needs a handle to repeat.
        // The loader does not load a disabled layer although it is installed.
        Step {
            name: "test-unvalidated",
            program: "cargo",
            args: &[
                "test",
                "-p",
                "vkxp-vk",
                "-p",
                "vkxp-host",
                "--locked",
                "--no-fail-fast",
            ],
            env: &[("VK_LOADER_LAYERS_DISABLE", "*validation*")],
        },
        Step {
            name: "doc",
            program: "cargo",
            args: &["doc", "--workspace", "--no-deps", "--locked"],
            env: &[("RUSTDOCFLAGS", "-D warnings")],
        },
        Step {
            name: "openspec",
            program: "openspec",
            args: &["validate", "--all"],
            env: &[],
        },
    ]
}

/// Runs the steps in order and stops at the first one that fails, naming it. Each step's
/// duration is printed when it ends, and all of them once more at the end, pass or fail.
pub(crate) fn run(root: &Path) -> Result<()> {
    let mut durations = Vec::new();
    let mut outcome = Ok(());
    for step in steps() {
        let started = Instant::now();
        let result = run_step(&step, root);
        let elapsed = started.elapsed();
        durations.push((step.name, elapsed));
        eprintln!(
            "gate: step `{}` took {}",
            step.name,
            format_duration(elapsed)
        );
        if result.is_err() {
            outcome = result;
            break;
        }
    }
    eprint!("{}", report(&durations));
    if outcome.is_ok() {
        eprintln!("gate: all steps passed");
    }
    outcome
}

fn run_step(step: &Step, root: &Path) -> Result<()> {
    let command_line = format!("{} {}", step.program, step.args.join(" "));
    eprintln!("gate: running step `{}` ({command_line})", step.name);
    // Nested cargo calls go through the cargo that is running us.
    let program = match step.program {
        "cargo" => crate::cargo(),
        other => other.to_owned(),
    };
    let status = Command::new(program)
        .args(step.args)
        .envs(step.env.iter().copied())
        .current_dir(root)
        .status()
        .with_context(|| {
            format!(
                "gate: FAILED at step `{}`: could not start `{command_line}`",
                step.name
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "gate: FAILED at step `{}`: `{command_line}` ended with {status}",
            step.name
        ))
    }
}

/// One line per step that ran, with the time it took.
fn report(durations: &[(&str, Duration)]) -> String {
    durations
        .iter()
        .map(|(name, duration)| format!("gate: {name:<16} {:>8}\n", format_duration(*duration)))
        .collect()
}

fn format_duration(duration: Duration) -> String {
    format!("{:.1} s", duration.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_are_the_six_checks_in_order() {
        let names: Vec<&str> = steps().iter().map(|step| step.name).collect();
        assert_eq!(
            names,
            [
                "fmt",
                "clippy",
                "test",
                "test-unvalidated",
                "doc",
                "openspec"
            ]
        );
    }

    #[test]
    fn the_audit_is_not_a_step() {
        for step in steps() {
            assert!(!step.args.contains(&"deny"), "{}", step.name);
            assert!(!step.args.contains(&"audit"), "{}", step.name);
        }
    }

    #[test]
    fn the_stress_run_is_not_a_step() {
        for step in steps() {
            assert!(!step.args.contains(&"stress"), "{}", step.name);
            for (name, _) in step.env {
                assert_ne!(*name, "VKXP_GC_STRESS", "{}", step.name);
            }
        }
    }

    #[test]
    fn the_steps_that_resolve_dependencies_hold_the_lock_file() {
        for step in steps() {
            let resolves = matches!(step.name, "clippy" | "test" | "test-unvalidated" | "doc");
            assert_eq!(step.args.contains(&"--locked"), resolves, "{}", step.name);
        }
    }

    #[test]
    fn the_documentation_step_denies_warnings() {
        let steps = steps();
        let doc = steps.iter().find(|step| step.name == "doc").unwrap();
        assert_eq!(doc.args, ["doc", "--workspace", "--no-deps", "--locked"]);
        assert_eq!(doc.env, [("RUSTDOCFLAGS", "-D warnings")]);
        let others = |step: &&Step| !matches!(step.name, "doc" | "test-unvalidated");
        for step in steps.iter().filter(others) {
            assert!(step.env.is_empty(), "{}", step.name);
        }
    }

    #[test]
    fn the_vulkan_tests_run_again_without_the_validation_layer() {
        let steps = steps();
        let position = |name| steps.iter().position(|step| step.name == name).unwrap();
        assert_eq!(position("test-unvalidated"), position("test") + 1);
        let step = &steps[position("test-unvalidated")];
        assert_eq!(
            step.args,
            [
                "test",
                "-p",
                "vkxp-vk",
                "-p",
                "vkxp-host",
                "--locked",
                "--no-fail-fast"
            ]
        );
        assert_eq!(step.env, [("VK_LOADER_LAYERS_DISABLE", "*validation*")]);
    }

    #[test]
    fn the_test_steps_run_every_test_binary() {
        for step in steps() {
            let tests = matches!(step.name, "test" | "test-unvalidated");
            assert_eq!(
                step.args.contains(&"--no-fail-fast"),
                tests,
                "{}",
                step.name
            );
        }
    }

    #[test]
    fn the_report_names_each_step_that_ran_with_its_duration() {
        let durations = [
            ("fmt", Duration::from_millis(440)),
            ("clippy", Duration::from_millis(61_300)),
        ];
        assert_eq!(
            report(&durations),
            "gate: fmt                 0.4 s\ngate: clippy             61.3 s\n"
        );
    }
}
