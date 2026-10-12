//! The CI workflow has the gate on x86_64 and, allowed to fail, on aarch64.

use std::fs;
use std::path::Path;

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

/// The lines of a job from `steps:` on, without comments and blank lines.
fn steps<'a>(job: &[&'a str]) -> Vec<&'a str> {
    job.iter()
        .copied()
        .skip_while(|line| line.trim() != "steps:")
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .collect()
}

#[test]
fn the_aarch64_job_has_the_steps_of_the_gate_and_may_fail() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/ci.yml");
    let workflow = fs::read_to_string(path).unwrap();
    let gate = job(&workflow, "gate");
    let aarch64 = job(&workflow, "gate-aarch64");

    assert!(gate.contains(&"    runs-on: ubuntu-24.04"));
    assert!(!gate.iter().any(|line| line.contains("continue-on-error")));
    assert!(aarch64.contains(&"    runs-on: ubuntu-24.04-arm"));
    assert!(aarch64.contains(&"    continue-on-error: true"));
    assert!(
        steps(&gate)
            .iter()
            .any(|line| line.contains("cargo xtask gate"))
    );
    assert_eq!(steps(&aarch64), steps(&gate));
}
