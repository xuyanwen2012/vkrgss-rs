//! The workflows: the gate on x86_64 and, allowed to fail, on aarch64; the release only for a
//! tag; the checks outside the gate once a day; and in all of them inputs that do not move.

use std::fs;
use std::path::{Path, PathBuf};

use regex_lite::Regex;

fn workflows_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows")
}

fn read_workflow(name: &str) -> String {
    let path = workflows_dir().join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Every workflow file with its name.
fn workflows() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(workflows_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".yml") || name.ends_with(".yaml"))
        .map(|name| {
            let text = read_workflow(&name);
            (name, text)
        })
        .collect();
    files.sort();
    files
}

/// The lines of the job `name` in the workflow text: from the line after its key to the
/// next line that is indented as little as the key.
fn job<'a>(workflow: &'a str, name: &str) -> Vec<&'a str> {
    let key = format!("  {name}:");
    workflow
        .lines()
        .skip_while(|line| *line != key)
        .skip(1)
        .take_while(|line| line.is_empty() || line.starts_with("    "))
        .collect()
}

/// The names of the jobs of a workflow: the keys indented by two spaces below `jobs:`.
fn job_names(workflow: &str) -> Vec<&str> {
    workflow
        .lines()
        .skip_while(|line| *line != "jobs:")
        .skip(1)
        .filter_map(|line| {
            let key = line.strip_prefix("  ")?.strip_suffix(':')?;
            (!key.starts_with([' ', '#'])).then_some(key)
        })
        .collect()
}

/// The lines of a job from `steps:` on, without comments and blank lines.
fn steps<'a>(job: &[&'a str]) -> Vec<&'a str> {
    job.iter()
        .copied()
        .skip_while(|line| line.trim() != "steps:")
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .collect()
}

/// The lines of a workflow without comment lines.
fn code_lines(workflow: &str) -> impl Iterator<Item = &str> {
    workflow
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
}

#[test]
fn the_aarch64_job_has_the_steps_of_the_gate_and_may_fail() {
    let workflow = read_workflow("ci.yml");
    let gate = job(&workflow, "gate");
    let aarch64 = job(&workflow, "gate-aarch64");

    assert!(gate.contains(&"    runs-on: ubuntu-24.04"));
    assert!(!gate.iter().any(|line| line.contains("continue-on-error")));
    assert!(aarch64.contains(&"    runs-on: ubuntu-24.04-arm"));
    assert!(aarch64.contains(&"    continue-on-error: true"));
    assert!(
        steps(&gate)
            .iter()
            .any(|line| line.contains("VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate"))
    );
    assert_eq!(steps(&aarch64), steps(&gate));
}

#[test]
fn the_release_is_created_only_for_a_tag() {
    let workflow = read_workflow("release.yml");
    let package = job(&workflow, "package");
    let create = package
        .iter()
        .position(|line| line.trim() == "- name: Create the release")
        .expect("a step that creates the release");
    let condition = package[create + 1].trim();
    assert!(
        condition.starts_with("if: ") && condition.contains("github.ref_type == 'tag'"),
        "{condition}"
    );
}

#[test]
fn every_action_is_named_by_a_commit_hash() {
    let pinned = Regex::new(r"^-? ?uses: [\w.-]+/[\w./-]+@[0-9a-f]{40} # v\S+$").unwrap();
    for (name, workflow) in workflows() {
        for line in code_lines(&workflow).filter(|line| line.contains("uses:")) {
            assert!(pinned.is_match(line.trim()), "{name}: `{}`", line.trim());
        }
    }
}

#[test]
fn the_openspec_cli_is_installed_at_a_fixed_version() {
    let versioned = Regex::new(r"@fission-ai/openspec@\d+\.\d+\.\d+(\s|$)").unwrap();
    let mut installs = 0;
    for (name, workflow) in workflows() {
        for line in code_lines(&workflow).filter(|line| line.contains("@fission-ai/openspec")) {
            installs += 1;
            assert!(versioned.is_match(line), "{name}: `{}`", line.trim());
        }
    }
    assert!(installs > 0, "no workflow installs the OpenSpec CLI");
}

#[test]
fn a_run_is_cancelled_only_for_a_pull_request() {
    for (name, workflow) in workflows() {
        for line in code_lines(&workflow).filter(|line| line.contains("cancel-in-progress:")) {
            assert_eq!(
                line.trim(),
                "cancel-in-progress: ${{ github.event_name == 'pull_request' }}",
                "{name}"
            );
        }
    }
    assert!(read_workflow("ci.yml").contains("cancel-in-progress:"));
}

#[test]
fn every_job_names_the_c_compiler() {
    for (name, workflow) in workflows() {
        let names = job_names(&workflow);
        assert!(!names.is_empty(), "{name} has no jobs");
        for job_name in names {
            let lines = job(&workflow, job_name);
            let env = lines
                .iter()
                .position(|line| *line == "    env:")
                .unwrap_or_else(|| panic!("{name}: the job `{job_name}` has no `env`"));
            assert_eq!(
                lines.get(env + 1),
                Some(&"      CC: gcc-13"),
                "{name}: the job `{job_name}` does not set `CC`"
            );
            if let Some(install) = lines
                .iter()
                .position(|line| line.contains("apt-get install"))
            {
                assert!(
                    lines[install..]
                        .iter()
                        .take_while(|line| !line.trim().starts_with('-'))
                        .any(|line| line.split_whitespace().any(|word| word == "gcc-13")),
                    "{name}: the job `{job_name}` does not install `gcc-13`"
                );
            }
        }
    }
}

#[test]
fn the_nightly_workflow_runs_the_checks_outside_the_gate() {
    let workflow = read_workflow("nightly.yml");
    let on: Vec<&str> = workflow
        .lines()
        .skip_while(|line| *line != "on:")
        .skip(1)
        .take_while(|line| line.is_empty() || line.starts_with(' '))
        .collect();
    assert!(on.contains(&"  schedule:"), "{on:?}");
    assert!(on.contains(&"  workflow_dispatch:"), "{on:?}");
    assert!(!on.iter().any(|line| line.contains("pull_request")));

    let runs = |job_name: &str, command: &str| {
        let lines = job(&workflow, job_name);
        assert!(
            steps(&lines).join(" ").contains(command),
            "the job `{job_name}` does not run `{command}`"
        );
    };
    runs("audit", "cargo xtask audit");
    runs(
        "stress",
        "VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask stress",
    );
    runs(
        "sanitized",
        "cargo test -p vkrgss-ruby-sys --locked --test sanitized -- \
         --ignored upstream_suites_pass_under_the_sanitizers",
    );
    runs("fuzz", "cargo xtask fuzz ${{ matrix.target }} --seconds");
    assert!(
        job(&workflow, "fuzz").contains(&"        target: [marshal, parse, regex]"),
        "the job `fuzz` does not run each target"
    );
    let audit = job(&workflow, "audit");
    assert!(
        audit
            .iter()
            .any(|line| line.contains("cargo install --locked cargo-deny@")),
        "cargo-deny is not installed at a fixed version"
    );
}
