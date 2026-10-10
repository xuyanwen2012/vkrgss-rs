//! The dependency audit: `cargo deny` with the configuration in `deny.toml` at the root
//! (security advisories, licences, sources). It needs the network for the advisory database
//! and is not a step of the merge gate.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};

/// The arguments of the check, after the cargo executable. An advisory that only says a
/// crate is unmaintained is a warning; the configuration file cannot say that, the command
/// line can.
const CHECK: [&str; 5] = ["deny", "--locked", "check", "--warn", "unmaintained"];

/// What is said when `cargo-deny` is not installed.
const MISSING: &str = "audit: `cargo-deny` is not installed; install it with \
                       `cargo install --locked cargo-deny` and run `cargo xtask audit` again";

/// Runs the check in `root` through the cargo executable `cargo`.
pub(crate) fn run(cargo: &str, root: &Path) -> Result<()> {
    // Cargo has no `deny` subcommand of its own: asking for its version fails exactly when
    // `cargo-deny` is not installed.
    let installed = Command::new(cargo)
        .args(["deny", "--version"])
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("audit: could not start `{cargo}`"))?
        .success();
    if !installed {
        bail!(MISSING);
    }
    let command_line = format!("cargo {}", CHECK.join(" "));
    eprintln!("audit: running `{command_line}`");
    let status = Command::new(cargo)
        .args(CHECK)
        .current_dir(root)
        .status()
        .with_context(|| format!("audit: could not start `{command_line}`"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("audit: `{command_line}` ended with {status}"))
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    /// Writes a shell script that stands in for cargo and returns the directory and its path.
    fn fake_cargo(script: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cargo");
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let path = path.to_str().unwrap().to_owned();
        (dir, path)
    }

    #[test]
    fn a_missing_tool_is_an_error_that_says_how_to_install_it() {
        // What cargo does with a subcommand it does not know.
        let (dir, cargo) = fake_cargo("echo 'error: no such command: `deny`' >&2; exit 101");
        let error = run(&cargo, dir.path()).unwrap_err().to_string();
        assert!(error.contains("`cargo-deny` is not installed"), "{error}");
        assert!(
            error.contains("cargo install --locked cargo-deny"),
            "{error}"
        );
    }

    #[test]
    fn the_check_holds_the_lock_file_and_its_status_is_the_outcome() {
        let (dir, cargo) = fake_cargo("echo \"$*\" >> calls; test \"$2\" = --version");
        let error = run(&cargo, dir.path()).unwrap_err().to_string();
        assert!(
            error.contains("`cargo deny --locked check --warn unmaintained` ended"),
            "{error}"
        );
        let calls = std::fs::read_to_string(dir.path().join("calls")).unwrap();
        assert_eq!(
            calls,
            "deny --version\ndeny --locked check --warn unmaintained\n"
        );

        let (dir, cargo) = fake_cargo("exit 0");
        run(&cargo, dir.path()).unwrap();
    }

    #[test]
    fn the_configuration_is_committed_at_the_root() {
        let root = crate::workspace_root();
        let config = std::fs::read_to_string(root.join("deny.toml")).unwrap();
        for table in ["[advisories]", "[licenses]", "[sources]"] {
            assert!(config.contains(table), "{table}");
        }
    }
}
