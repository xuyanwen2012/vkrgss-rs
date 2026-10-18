//! Asking for a game directory on macOS.
//!
//! The app started without a game directory, as a double-click starts it, asks for one with
//! the system's folder chooser, through `osascript` ([`SCRIPT`]), whose cancel ends the
//! program quietly. The directory chosen is kept in [`LAST_FILE`] of the user's
//! `~/Library/Application Support/vkrgss/` and offered the next time as the chooser's default
//! location. Only the program of an app bundle, `<x>.app/Contents/MacOS/vkrgss`, asks: started
//! elsewhere without a game directory, it runs the working directory, as on Linux.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// The script of the chooser: the POSIX path of the folder chosen, at the default location
/// its argument names if it has one.
const SCRIPT: &str = r#"on run argv
	set question to "Choose the folder of a game of RPG Maker XP, VX or VX Ace"
	if (count of argv) is 0 then
		return POSIX path of (choose folder with prompt question)
	end if
	return POSIX path of (choose folder with prompt question default location (POSIX file (item 1 of argv)))
end run"#;

/// The error number of AppleScript for a cancel by the user.
const CANCELLED: &str = "(-128)";

/// The file that keeps the directory chosen last, in the user's directory of the program.
const LAST_FILE: &str = "last-game-dir";

/// Whether the program lies in an app bundle, as `<x>.app/Contents/MacOS/<name>`.
pub(crate) fn in_app_bundle() -> bool {
    let Ok(exe) = std::env::current_exe().and_then(std::fs::canonicalize) else {
        return false;
    };
    let (Some(macos), Some(contents)) = (exe.parent(), exe.parent().and_then(Path::parent)) else {
        return false;
    };
    macos.file_name() == Some(OsStr::new("MacOS"))
        && contents.file_name() == Some(OsStr::new("Contents"))
        && contents.parent().and_then(Path::extension) == Some(OsStr::new("app"))
}

/// Asks for a game directory with the folder chooser, at the directory chosen last, and keeps
/// the one chosen. `None` when the user cancels.
pub(crate) fn ask() -> Result<Option<PathBuf>> {
    let last = last_file();
    let default = last
        .as_deref()
        .and_then(|file| std::fs::read(file).ok())
        .map(|bytes| PathBuf::from(OsString::from_vec(bytes)))
        .filter(|dir| dir.is_dir());
    let Some(chosen) = choose(default.as_deref())? else {
        return Ok(None);
    };
    if let Some(file) = last {
        let write = || -> std::io::Result<()> {
            std::fs::create_dir_all(file.parent().expect("a file below a directory"))?;
            std::fs::write(&file, chosen.as_os_str().as_bytes())
        };
        if let Err(error) = write() {
            tracing::warn!(
                "cannot keep the game directory in {}: {error}",
                file.display()
            );
        }
    }
    Ok(Some(chosen))
}

/// The file that keeps the directory chosen last, if the user's home is known.
fn last_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|home| !home.is_empty())?;
    Some(
        Path::new(&home)
            .join("Library/Application Support/vkrgss")
            .join(LAST_FILE),
    )
}

/// Asks for a folder with `osascript`, offering `default` first; `None` when the user cancels.
fn choose(default: Option<&Path>) -> Result<Option<PathBuf>> {
    let output = Command::new("osascript")
        .args(["-e", SCRIPT])
        .args(default)
        .output()
        .context("cannot start osascript to ask for a game directory")?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        if message.contains(CANCELLED) {
            return Ok(None);
        }
        bail!(
            "the folder chooser ended with {}: {}",
            output.status,
            message.trim()
        );
    }
    let mut path = output.stdout;
    if path.last() == Some(&b'\n') {
        path.pop();
    }
    if path.is_empty() {
        bail!("the folder chooser named no folder");
    }
    Ok(Some(PathBuf::from(OsString::from_vec(path))))
}
