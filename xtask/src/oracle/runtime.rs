//! The runtime of a game, RGSS1 of RPG Maker XP or RGSS2 of RPG Maker VX, and what the
//! harness does differently for each.

use anyhow::{Result, bail};

use crate::rgssad;

/// The runtime a game was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Runtime {
    Rgss1,
    Rgss2,
}

impl Runtime {
    /// Reads the value of `--runtime`.
    pub(super) fn parse(text: &str) -> Result<Runtime> {
        match text {
            "rgss1" => Ok(Runtime::Rgss1),
            "rgss2" => Ok(Runtime::Rgss2),
            _ => bail!("--runtime: `{text}` is neither rgss1 nor rgss2"),
        }
    }

    /// The runtime that the extension of the script archive `name` tells, if it tells one.
    pub(super) fn of_scripts(name: &str) -> Option<Runtime> {
        let (_, extension) = name.rsplit_once('.')?;
        match extension.to_ascii_lowercase().as_str() {
            "rxdata" => Some(Runtime::Rgss1),
            "rvdata" => Some(Runtime::Rgss2),
            _ => None,
        }
    }

    /// The width and height of a frame, which is the size of the game window.
    pub(super) fn frame_size(self) -> (u32, u32) {
        match self {
            Runtime::Rgss1 => (640, 480),
            Runtime::Rgss2 => (544, 416),
        }
    }

    /// The registry key below `HKEY_LOCAL_MACHINE\Software`, in the 32-bit view, whose values
    /// are the directories of the run-time packages. A value has the name that `Game.ini`
    /// gives the package: with `RTP1` to `RTP3` in RGSS1, with `RTP` in RGSS2.
    pub(super) fn rtp_key(self) -> &'static str {
        match self {
            Runtime::Rgss1 => "Enterbrain\\RGSS\\RTP",
            Runtime::Rgss2 => "Enterbrain\\RGSS2\\RTP",
        }
    }

    /// The name of the encrypted archive in a game directory.
    pub(super) fn archive_name(self) -> &'static str {
        match self {
            Runtime::Rgss1 => rgssad::FILE_NAME,
            Runtime::Rgss2 => rgssad::FILE_NAME_RGSS2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_of_the_script_archive_tells_the_runtime() {
        assert_eq!(
            Runtime::of_scripts("Data\\Scripts.rxdata"),
            Some(Runtime::Rgss1)
        );
        assert_eq!(
            Runtime::of_scripts("Data\\Scripts.RVDATA"),
            Some(Runtime::Rgss2)
        );
        assert_eq!(Runtime::of_scripts("Data\\Code.bin"), None);
        assert_eq!(Runtime::of_scripts("Scripts"), None);
        assert!(Runtime::parse("rgss2").is_ok());
        assert!(Runtime::parse("vx").is_err());
    }
}
