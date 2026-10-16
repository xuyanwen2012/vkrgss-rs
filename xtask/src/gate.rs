//! The merge gate: one command that every change must pass.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};

/// One check of the gate: a name used in reports, the command behind it and the environment
/// variables it sets for that command.
#[derive(Clone, Copy)]
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
                "vkrgss-vk",
                "-p",
                "vkrgss-host",
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

/// The parts of the gate that `cargo xtask gate --job <name>` runs one at a time, as the CI
/// workflow does in parallel jobs. Together they run exactly the checks of the gate, which a
/// test compares: every step, package and test target in one part. A package added later is
/// in `rest`; a test target added to `vkrgss-ruby192-sys` goes into [`RUBY192_TESTS`].
fn parts() -> Vec<Part> {
    let gate = steps();
    let step = |name: &str| *gate.iter().find(|step| step.name == name).unwrap();
    let test = |args| Step {
        args,
        ..step("test")
    };
    vec![
        Part {
            name: "lint",
            steps: vec![step("fmt"), step("clippy"), step("doc"), step("openspec")],
        },
        Part {
            name: "ruby",
            steps: vec![
                test(&[
                    "test",
                    "-p",
                    "vkrgss-ruby-sys",
                    "--locked",
                    "--no-fail-fast",
                ]),
                test(RUBY192_TESTS),
                test(&[
                    "test",
                    "-p",
                    "vkrgss-ruby192-sys",
                    "--doc",
                    "--locked",
                    "--no-fail-fast",
                ]),
            ],
        },
        // The quick stressed files that the workspace tests hold (requirement "Collector
        // stress"); the rest of the stress is a check outside the gate.
        Part {
            name: "ruby-stress",
            steps: vec![test(&[
                "test",
                "-p",
                "vkrgss-ruby192-sys",
                "--test",
                "upstream_stress",
                "--locked",
                "--no-fail-fast",
            ])],
        },
        Part {
            name: "vulkan",
            steps: vec![test(&[
                "test",
                "-p",
                "vkrgss-vk",
                "--locked",
                "--no-fail-fast",
            ])],
        },
        Part {
            name: "vulkan-unvalidated",
            steps: vec![step("test-unvalidated")],
        },
        Part {
            name: "rest",
            steps: vec![test(&[
                "test",
                "--workspace",
                "--exclude",
                "vkrgss-ruby-sys",
                "--exclude",
                "vkrgss-ruby192-sys",
                "--exclude",
                "vkrgss-vk",
                "--locked",
                "--no-fail-fast",
            ])],
        },
    ]
}

/// One named part of the gate: steps of the gate, of which a test step may be limited to some
/// packages or test targets.
struct Part {
    name: &'static str,
    steps: Vec<Step>,
}

/// The library and every test target of `vkrgss-ruby192-sys` but `upstream_stress`, which
/// is the part `ruby-stress`.
const RUBY192_TESTS: &[&str] = &[
    "test",
    "-p",
    "vkrgss-ruby192-sys",
    "--lib",
    "--test",
    "backports",
    "--test",
    "busy_main_thread",
    "--test",
    "calling",
    "--test",
    "classes",
    "--test",
    "encodings",
    "--test",
    "fiber_host_frames",
    "--test",
    "fork_busy_main_after_copy",
    "--test",
    "fork_frames",
    "--test",
    "fork_frames_restart",
    "--test",
    "fork_frames_stressed",
    "--test",
    "fork_thread_after_copy",
    "--test",
    "fuzz",
    "--test",
    "gc_stress_off",
    "--test",
    "gc_stress_on",
    "--test",
    "hardening",
    "--test",
    "hash_seed",
    "--test",
    "interpreter",
    "--test",
    "layout",
    "--test",
    "math",
    "--test",
    "original",
    "--test",
    "protect",
    "--test",
    "sanitized",
    "--test",
    "script_name",
    "--test",
    "signals_default",
    "--test",
    "signals_host",
    "--test",
    "stack_limit",
    "--test",
    "symbols",
    "--test",
    "threads_lost",
    "--test",
    "timer_thread",
    "--test",
    "upstream",
    "--test",
    "upstream_path",
    "--test",
    "upstream_signals",
    "--locked",
    "--no-fail-fast",
];

/// The whole gate.
pub(crate) fn run(root: &Path) -> Result<()> {
    run_steps(&steps(), root)
}

/// One part of the gate. An unknown name fails and lists the names of the parts.
pub(crate) fn run_part(name: &str, root: &Path) -> Result<()> {
    let part = part(name)?;
    eprintln!("gate: part `{}`", part.name);
    run_steps(&part.steps, root)
}

fn part(name: &str) -> Result<Part> {
    let mut parts = parts();
    match parts.iter().position(|part| part.name == name) {
        Some(position) => Ok(parts.swap_remove(position)),
        None => {
            let names: Vec<&str> = parts.iter().map(|part| part.name).collect();
            Err(anyhow!(
                "gate: no part named `{name}`; the parts are: {}",
                names.join(", ")
            ))
        }
    }
}

/// Runs the steps in order and stops at the first one that fails, naming it. Each step's
/// duration is printed when it ends, and all of them once more at the end, pass or fail.
fn run_steps(steps: &[Step], root: &Path) -> Result<()> {
    let mut durations = Vec::new();
    let mut outcome = Ok(());
    for step in steps {
        let started = Instant::now();
        let result = run_step(step, root);
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
    use std::collections::{BTreeMap, BTreeSet};

    use serde_json::Value;

    use super::*;

    /// The targets that `cargo test` builds or runs in each package of the workspace, without
    /// a selection: the library, its documentation tests, the executables, the test targets
    /// and the examples.
    fn workspace_targets() -> BTreeMap<String, BTreeSet<String>> {
        let output = Command::new(crate::cargo())
            .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
            .current_dir(crate::workspace_root())
            .output()
            .expect("cargo metadata for the workspace");
        assert!(output.status.success());
        let metadata: Value = serde_json::from_slice(&output.stdout).expect("metadata JSON");
        let mut packages = BTreeMap::new();
        for package in metadata["packages"].as_array().unwrap() {
            let mut targets = BTreeSet::new();
            for target in package["targets"].as_array().unwrap() {
                let kind = target["kind"][0].as_str().unwrap();
                let name = target["name"].as_str().unwrap();
                match kind {
                    "lib" => {
                        targets.insert("lib".to_owned());
                        if target["doctest"] == true {
                            targets.insert("doc".to_owned());
                        }
                    }
                    "bin" | "test" | "example" => {
                        targets.insert(format!("{kind} {name}"));
                    }
                    _ => {}
                }
            }
            packages.insert(package["name"].as_str().unwrap().to_owned(), targets);
        }
        packages
    }

    /// The (step, package, target) that a step of the gate or of a part checks, and the
    /// arguments that do not select packages or targets. A step other than `cargo test`
    /// checks one item, (step, "", ""), and keeps all its arguments.
    fn coverage(
        step: &Step,
        packages: &BTreeMap<String, BTreeSet<String>>,
    ) -> (Vec<(String, String, String)>, Vec<&'static str>) {
        if step.program != "cargo" || step.args.first() != Some(&"test") {
            let item = (step.name.to_owned(), String::new(), String::new());
            return (vec![item], step.args.to_vec());
        }
        let (mut workspace, mut selected, mut excluded) = (false, Vec::new(), Vec::new());
        let (mut targets, mut others) = (Vec::new(), vec!["test"]);
        let mut args = step.args[1..].iter();
        while let Some(arg) = args.next() {
            match *arg {
                "--workspace" => workspace = true,
                "-p" => selected.push(*args.next().unwrap()),
                "--exclude" => excluded.push(*args.next().unwrap()),
                "--lib" => targets.push("lib".to_owned()),
                "--doc" => targets.push("doc".to_owned()),
                "--test" => targets.push(format!("test {}", args.next().unwrap())),
                other => {
                    assert!(
                        matches!(other, "--locked" | "--no-fail-fast"),
                        "{}: `{other}` is not understood by this test",
                        step.name
                    );
                    others.push(other);
                }
            }
        }
        if workspace {
            assert!(selected.is_empty(), "{}", step.name);
            selected = packages.keys().map(String::as_str).collect();
        }
        let mut items = Vec::new();
        for package in selected.into_iter().filter(|p| !excluded.contains(p)) {
            let all = &packages[package];
            let chosen: Vec<&String> = if targets.is_empty() {
                all.iter().collect()
            } else {
                targets.iter().collect()
            };
            for target in chosen {
                assert!(all.contains(target), "{package} has no target `{target}`");
                items.push((step.name.to_owned(), package.to_owned(), target.clone()));
            }
        }
        (items, others)
    }

    #[test]
    fn the_parts_together_are_the_gate() {
        let packages = workspace_targets();
        let gate = steps();
        let mut expected = BTreeSet::new();
        for step in &gate {
            expected.extend(coverage(step, &packages).0);
        }

        let mut names = BTreeSet::new();
        let mut covered: BTreeMap<(String, String, String), Vec<&str>> = BTreeMap::new();
        for part in parts() {
            assert!(names.insert(part.name), "two parts named `{}`", part.name);
            for step in &part.steps {
                let full = gate
                    .iter()
                    .find(|full| full.name == step.name)
                    .unwrap_or_else(|| panic!("`{}` is not a step of the gate", step.name));
                assert_eq!(step.program, full.program, "{}", part.name);
                assert_eq!(step.env, full.env, "{}: {}", part.name, step.name);
                let (items, others) = coverage(step, &packages);
                assert_eq!(others, coverage(full, &packages).1, "{}", part.name);
                for item in items {
                    covered.entry(item).or_default().push(part.name);
                }
            }
        }

        let missing: Vec<_> = expected
            .difference(&covered.keys().cloned().collect())
            .cloned()
            .collect();
        assert!(missing.is_empty(), "in no part: {missing:?}");
        let extra: Vec<_> = covered
            .keys()
            .filter(|item| !expected.contains(*item))
            .collect();
        assert!(extra.is_empty(), "not in the gate: {extra:?}");
        let twice: Vec<_> = covered
            .iter()
            .filter(|(_, parts)| parts.len() > 1)
            .collect();
        assert!(twice.is_empty(), "in more than one part: {twice:?}");
    }

    #[test]
    fn an_unknown_part_fails_and_lists_the_parts() {
        let error = part("nothing").err().unwrap().to_string();
        assert_eq!(
            error,
            "gate: no part named `nothing`; the parts are: \
             lint, ruby, ruby-stress, vulkan, vulkan-unvalidated, rest"
        );
        assert_eq!(part("ruby-stress").unwrap().name, "ruby-stress");
    }

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
    fn the_fuzz_run_is_not_a_step() {
        for step in steps() {
            assert!(!step.args.contains(&"fuzz"), "{}", step.name);
            assert!(!step.args.contains(&"--ignored"), "{}", step.name);
        }
    }

    #[test]
    fn the_stress_run_is_not_a_step() {
        for step in steps() {
            assert!(!step.args.contains(&"stress"), "{}", step.name);
            for (name, _) in step.env {
                assert_ne!(*name, "VKRGSS_GC_STRESS", "{}", step.name);
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
                "vkrgss-vk",
                "-p",
                "vkrgss-host",
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
