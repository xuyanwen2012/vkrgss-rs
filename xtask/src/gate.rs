//! The merge gate: one command that every change must pass.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// One check of the gate: a name used in reports and the command behind it.
struct Step {
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

fn steps() -> [Step; 4] {
    [
        Step {
            name: "fmt",
            program: "cargo",
            args: &["fmt", "--all", "--check"],
        },
        Step {
            name: "clippy",
            program: "cargo",
            args: &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        },
        Step {
            name: "test",
            program: "cargo",
            args: &["test", "--workspace"],
        },
        Step {
            name: "openspec",
            program: "openspec",
            args: &["validate", "--all"],
        },
    ]
}

/// Runs the steps in order and stops at the first one that fails, naming it.
pub fn run(root: &Path) -> Result<()> {
    for step in steps() {
        let command_line = format!("{} {}", step.program, step.args.join(" "));
        eprintln!("gate: running step `{}` ({command_line})", step.name);
        // Nested cargo calls go through the cargo that is running us.
        let program = match step.program {
            "cargo" => crate::cargo(),
            other => other.to_owned(),
        };
        let status = Command::new(program)
            .args(step.args)
            .current_dir(root)
            .status()
            .with_context(|| {
                format!(
                    "gate: FAILED at step `{}`: could not start `{command_line}`",
                    step.name
                )
            })?;
        if !status.success() {
            bail!(
                "gate: FAILED at step `{}`: `{command_line}` ended with {status}",
                step.name
            );
        }
    }
    eprintln!("gate: all steps passed");
    Ok(())
}
