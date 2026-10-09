//! The vkxp side of a comparison: a headless run that dumps the frames asked for.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

use super::report;

/// Environment variables that restrict which drivers the Vulkan loader loads.
const ISOLATION_VARIABLES: [&str; 3] = [
    "VK_LOADER_DRIVERS_SELECT",
    "VK_DRIVER_FILES",
    "VK_ICD_FILENAMES",
];

/// Whether the environment restricts the Vulkan loader to chosen drivers.
pub fn loader_is_restricted() -> bool {
    ISOLATION_VARIABLES
        .iter()
        .any(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()))
}

/// Builds the `vkxp` executable of this workspace and returns its path.
pub fn build(root: &Path) -> Result<PathBuf> {
    let output = Command::new(crate::cargo())
        .args(["build", "--package", "vkxp", "--message-format=json"])
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .context("could not start `cargo build --package vkxp`")?;
    ensure!(
        output.status.success(),
        "`cargo build --package vkxp` ended with {}",
        output.status
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact")
        .filter(|message| message["target"]["name"] == "vkxp")
        .find_map(|message| message["executable"].as_str().map(PathBuf::from))
        .context("`cargo build --package vkxp` did not name the executable")
}

/// What a run of vkxp needs besides the game.
pub struct Run<'a> {
    pub executable: &'a Path,
    /// The directory of each run-time package, by its name.
    pub rtps: &'a [(String, PathBuf)],
    pub builtin_scripts: Option<&'a Path>,
}

impl Run<'_> {
    /// Runs the game in `game` headless and writes each of `frames`, which are in ascending
    /// order, as the picture `out` holds for vkxp.
    pub fn capture(&self, game: &Path, frames: &[u64], out: &Path) -> Result<()> {
        let mut command = Command::new(self.executable);
        command.arg(game).arg("--headless");
        for (name, dir) in self.rtps {
            command
                .arg("--rtp")
                .arg(format!("{name}={}", dir.display()));
        }
        if let Some(file) = self.builtin_scripts {
            command.arg("--builtin-scripts").arg(file);
        }
        let last = frames.last().context("no frame to capture")?;
        command.arg("--frames").arg((last + 1).to_string());
        for &frame in frames {
            let file = out.join(report::vkxp_file(frame));
            command
                .arg("--dump")
                .arg(format!("{frame}={}", file.display()));
        }
        if !loader_is_restricted() {
            // Software rendering unless the caller chose drivers: the comparison is of
            // pixels, and no GPU of the machine is opened by accident.
            command.env("VK_LOADER_DRIVERS_SELECT", "*lvp*");
        }
        let status = command
            .status()
            .with_context(|| format!("could not start {}", self.executable.display()))?;
        if !status.success() {
            bail!("vkxp ended with {status}; its messages are above");
        }
        Ok(())
    }
}
