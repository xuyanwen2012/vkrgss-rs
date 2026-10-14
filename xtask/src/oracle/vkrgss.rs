//! The vkrgss side of a comparison: a headless run that dumps the frames asked for.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

use super::report;
use super::runtime::Runtime;

/// Environment variables that restrict which drivers the Vulkan loader loads.
const ISOLATION_VARIABLES: [&str; 3] = [
    "VK_LOADER_DRIVERS_SELECT",
    "VK_DRIVER_FILES",
    "VK_ICD_FILENAMES",
];

/// Whether the environment restricts the Vulkan loader to chosen drivers.
pub(super) fn loader_is_restricted() -> bool {
    ISOLATION_VARIABLES
        .iter()
        .any(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()))
}

/// The guest executable that `vkrgss` starts for a game of RGSS3, found beside its own file.
const GUEST_RGSS3: &str = "vkrgss-guest-rgss3";

/// Builds the `vkrgss` executable of this workspace and returns its path. For a game of RGSS3
/// the guest executable of RGSS3 is built in the same call, so that it lies beside `vkrgss`.
pub(super) fn build(root: &Path, runtime: Runtime) -> Result<PathBuf> {
    let mut packages = vec!["vkrgss"];
    if runtime == Runtime::Rgss3 {
        packages.push(GUEST_RGSS3);
    }
    let mut command = Command::new(crate::cargo());
    command.arg("build");
    let mut name = "cargo build".to_owned();
    for package in &packages {
        command.args(["--package", package]);
        name += &format!(" --package {package}");
    }
    let output = command
        .arg("--message-format=json")
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("could not start `{name}`"))?;
    ensure!(
        output.status.success(),
        "`{name}` ended with {}",
        output.status
    );
    let executable = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact")
        .filter(|message| message["target"]["name"] == "vkrgss")
        .find_map(|message| message["executable"].as_str().map(PathBuf::from))
        .with_context(|| format!("`{name}` did not name the executable"))?;
    if runtime == Runtime::Rgss3 {
        let guest = executable.with_file_name(GUEST_RGSS3);
        ensure!(
            guest.is_file(),
            "{}: there is no guest executable of RGSS3 beside vkrgss after `{name}`; \
             build it with `cargo build -p {GUEST_RGSS3}`",
            guest.display()
        );
    }
    Ok(executable)
}

/// What a run of vkrgss needs besides the game.
pub(super) struct Run<'a> {
    pub executable: &'a Path,
    /// The runtime the harness decided on, which vkrgss is told rather than left to decide
    /// again from `Game.ini`.
    pub runtime: Runtime,
    /// The directory of each run-time package, by its name.
    pub rtps: &'a [(String, PathBuf)],
    /// A directory of fonts that vkrgss searches before those of the system: the fonts shared
    /// with the original, which vkrgss then sizes by their own line boxes as the original does.
    pub font_dir: Option<&'a Path>,
    pub builtin_scripts: Option<&'a Path>,
}

impl Run<'_> {
    /// Runs the game in `game` headless and writes each of `frames`, which are in ascending
    /// order, as the picture `out` holds for vkrgss.
    pub(super) fn capture(&self, game: &Path, frames: &[u64], out: &Path) -> Result<()> {
        let status = self
            .command(game, frames, out)?
            .status()
            .with_context(|| format!("could not start {}", self.executable.display()))?;
        if !status.success() {
            bail!("vkrgss ended with {status}; its messages are above");
        }
        Ok(())
    }

    /// The command of the run that `capture` describes.
    pub(super) fn command(&self, game: &Path, frames: &[u64], out: &Path) -> Result<Command> {
        let mut command = Command::new(self.executable);
        command
            .arg(game)
            .arg("--headless")
            .arg("--runtime")
            .arg(self.runtime.name());
        for (name, dir) in self.rtps {
            command
                .arg("--rtp")
                .arg(format!("{name}={}", dir.display()));
        }
        if let Some(dir) = self.font_dir {
            command.arg("--font-dir").arg(dir).arg("--font-own-metrics");
        }
        if let Some(file) = self.builtin_scripts {
            command.arg("--builtin-scripts").arg(file);
        }
        let last = frames.last().context("no frame to capture")?;
        command.arg("--frames").arg((last + 1).to_string());
        for &frame in frames {
            let file = out.join(report::vkrgss_file(frame));
            command
                .arg("--dump")
                .arg(format!("{frame}={}", file.display()));
        }
        if !loader_is_restricted() {
            // Software rendering unless the caller chose drivers: the comparison is of
            // pixels, and no GPU of the machine is opened by accident.
            command.env("VK_LOADER_DRIVERS_SELECT", "*lvp*");
        }
        Ok(command)
    }
}
