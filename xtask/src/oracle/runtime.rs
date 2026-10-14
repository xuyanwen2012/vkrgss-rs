//! The runtime of a game, RGSS1 of RPG Maker XP, RGSS2 of RPG Maker VX or RGSS3 of RPG Maker
//! VX Ace, and what the harness does differently for each.

use anyhow::{Result, bail};

use crate::{rgss3a, rgssad};

/// The runtime a game was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Runtime {
    Rgss1,
    Rgss2,
    Rgss3,
}

impl Runtime {
    /// Reads the value of `--runtime`.
    pub(super) fn parse(text: &str) -> Result<Runtime> {
        match text {
            "rgss1" => Ok(Runtime::Rgss1),
            "rgss2" => Ok(Runtime::Rgss2),
            "rgss3" => Ok(Runtime::Rgss3),
            _ => bail!("--runtime: `{text}` is none of rgss1, rgss2 and rgss3"),
        }
    }

    /// The value of `--runtime` that names this runtime, to the harness and to vkrgss alike.
    pub(super) fn name(self) -> &'static str {
        match self {
            Runtime::Rgss1 => "rgss1",
            Runtime::Rgss2 => "rgss2",
            Runtime::Rgss3 => "rgss3",
        }
    }

    /// The runtime that the extension of the script archive `name` tells, if it tells one.
    pub(super) fn of_scripts(name: &str) -> Option<Runtime> {
        let (_, extension) = name.rsplit_once('.')?;
        match extension.to_ascii_lowercase().as_str() {
            "rxdata" => Some(Runtime::Rgss1),
            "rvdata" => Some(Runtime::Rgss2),
            "rvdata2" => Some(Runtime::Rgss3),
            _ => None,
        }
    }

    /// The width and height of a frame at the start, which is the size of the game window.
    /// A game of RGSS3 may resize its screen.
    pub(super) fn frame_size(self) -> (u32, u32) {
        match self {
            Runtime::Rgss1 => (640, 480),
            Runtime::Rgss2 | Runtime::Rgss3 => (544, 416),
        }
    }

    /// The registry key below `HKEY_LOCAL_MACHINE\Software`, in the 32-bit view, whose values
    /// are the directories of the run-time packages. A value has the name that `Game.ini`
    /// gives the package: with `RTP1` to `RTP3` in RGSS1, with `RTP` in RGSS2 and RGSS3.
    pub(super) fn rtp_key(self) -> &'static str {
        match self {
            Runtime::Rgss1 => "Enterbrain\\RGSS\\RTP",
            Runtime::Rgss2 => "Enterbrain\\RGSS2\\RTP",
            Runtime::Rgss3 => "Enterbrain\\RGSS3\\RTP",
        }
    }

    /// The name of the encrypted archive in a game directory.
    pub(super) fn archive_name(self) -> &'static str {
        match self {
            Runtime::Rgss1 => rgssad::FILE_NAME,
            Runtime::Rgss2 => rgssad::FILE_NAME_RGSS2,
            Runtime::Rgss3 => rgss3a::FILE_NAME,
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
        assert_eq!(
            Runtime::of_scripts("Data\\Scripts.rvdata2"),
            Some(Runtime::Rgss3)
        );
        assert_eq!(Runtime::of_scripts("Data\\Code.bin"), None);
        assert_eq!(Runtime::of_scripts("Scripts"), None);
        assert!(Runtime::parse("rgss2").is_ok());
        assert_eq!(Runtime::parse("rgss3").unwrap(), Runtime::Rgss3);
        for runtime in [Runtime::Rgss1, Runtime::Rgss2, Runtime::Rgss3] {
            assert_eq!(Runtime::parse(runtime.name()).unwrap(), runtime);
        }
        assert!(Runtime::parse("vx").is_err());
    }
}
