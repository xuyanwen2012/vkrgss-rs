//! The runtime a game was made for, and how `Game.ini` tells it.

use crate::error::VfsError;
use crate::ini::GameIni;

/// A version of the original runtime: the RGSS a game was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Runtime {
    /// RGSS1, of RPG Maker XP.
    Rgss1,
    /// RGSS2, of RPG Maker VX.
    Rgss2,
    /// RGSS3, of RPG Maker VX Ace.
    Rgss3,
}

impl Runtime {
    /// Every runtime, in order.
    pub const ALL: [Runtime; 3] = [Runtime::Rgss1, Runtime::Rgss2, Runtime::Rgss3];

    /// The name of the runtime in options and messages.
    pub fn name(self) -> &'static str {
        match self {
            Runtime::Rgss1 => "rgss1",
            Runtime::Rgss2 => "rgss2",
            Runtime::Rgss3 => "rgss3",
        }
    }

    /// The name of the editor whose games the runtime plays, as people know it.
    pub fn product(self) -> &'static str {
        match self {
            Runtime::Rgss1 => "RPG Maker XP",
            Runtime::Rgss2 => "RPG Maker VX",
            Runtime::Rgss3 => "RPG Maker VX Ace",
        }
    }

    /// The name of the encrypted archive of a game of this runtime that the reader of
    /// RGSSAD version 1 reads. The archive of RGSS3 is another format and has none.
    pub fn archive_name(self) -> Option<&'static str> {
        match self {
            Runtime::Rgss1 => Some("Game.rgssad"),
            Runtime::Rgss2 => Some("Game.rgss2a"),
            Runtime::Rgss3 => None,
        }
    }

    /// The runtime of the game whose configuration file is `ini`: what a recognised
    /// `Library` and the extension of `Scripts` say. One of them alone decides; when they
    /// differ, or neither says anything, the result is an error that names both values.
    pub fn detect(ini: &GameIni) -> Result<Runtime, VfsError> {
        match (of_library(&ini.library), of_scripts(&ini.scripts)) {
            (Some(library), Some(scripts)) if library == scripts => Ok(library),
            (Some(runtime), None) | (None, Some(runtime)) => Ok(runtime),
            _ => Err(VfsError::UndecidedRuntime {
                library: ini.library.clone(),
                scripts: ini.scripts.clone(),
            }),
        }
    }
}

/// The runtime a `Library` value names: its file name is `RGSS`, a digit 1 to 3, two more
/// digits, an optional letter and `.dll`, in any letter case.
fn of_library(library: &str) -> Option<Runtime> {
    let file = library.rsplit(['\\', '/']).next().unwrap_or(library);
    let file = file.to_ascii_lowercase();
    let version = file.strip_prefix("rgss")?.strip_suffix(".dll")?.as_bytes();
    let digits = match version {
        [digits @ .., letter] if letter.is_ascii_alphabetic() => digits,
        digits => digits,
    };
    let [major, minor @ ..] = digits else {
        return None;
    };
    if minor.len() != 2 || !minor.iter().all(u8::is_ascii_digit) {
        return None;
    }
    match major {
        b'1' => Some(Runtime::Rgss1),
        b'2' => Some(Runtime::Rgss2),
        b'3' => Some(Runtime::Rgss3),
        _ => None,
    }
}

/// The runtime the extension of a `Scripts` value names, in any letter case.
fn of_scripts(scripts: &str) -> Option<Runtime> {
    let scripts = scripts.to_ascii_lowercase();
    if scripts.ends_with(".rxdata") {
        Some(Runtime::Rgss1)
    } else if scripts.ends_with(".rvdata") {
        Some(Runtime::Rgss2)
    } else if scripts.ends_with(".rvdata2") {
        Some(Runtime::Rgss3)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(library: &str, scripts: &str) -> Result<Runtime, VfsError> {
        let text = format!("[Game]\nLibrary={library}\nScripts={scripts}\n");
        Runtime::detect(&GameIni::parse(text.as_bytes(), None).unwrap())
    }

    #[test]
    fn both_agree() {
        let runtime = detect("RGSS202E.dll", "Data\\Scripts.rvdata");
        assert_eq!(runtime.unwrap(), Runtime::Rgss2);
        let runtime = detect("rgss104e.DLL", "data\\scripts.RXDATA");
        assert_eq!(runtime.unwrap(), Runtime::Rgss1);
    }

    #[test]
    fn one_source() {
        for library in ["Player.dll", ""] {
            let runtime = detect(library, "Data\\Scripts.rxdata");
            assert_eq!(runtime.unwrap(), Runtime::Rgss1);
        }
        let runtime = detect("RGSS200J.dll", "Data\\Code.bin");
        assert_eq!(runtime.unwrap(), Runtime::Rgss2);
    }

    #[test]
    fn library_in_a_folder() {
        let runtime = detect("System\\RGSS301.dll", "Data\\Scripts.rvdata2");
        assert_eq!(runtime.unwrap(), Runtime::Rgss3);
        let runtime = detect("System/RGSS300.dll", "Data\\Code.bin");
        assert_eq!(runtime.unwrap(), Runtime::Rgss3);
    }

    #[test]
    fn conflict_and_nothing_recognised() {
        for (library, scripts) in [
            ("RGSS104E.dll", "Data\\Scripts.rvdata"),
            ("Player.dll", "Data\\Code.bin"),
        ] {
            let error = detect(library, scripts).unwrap_err();
            assert!(matches!(error, VfsError::UndecidedRuntime { .. }));
            let message = error.to_string();
            assert!(message.contains(library), "{message}");
            assert!(message.contains(scripts), "{message}");
        }
    }

    #[test]
    fn a_library_is_recognised_by_its_whole_name() {
        for library in [
            "RGSS.dll",
            "RGSS4.dll",
            "RGSS402E.dll",
            "RGSS10.dll",
            "RGSS1040E.dll",
            "RGSS104EE.dll",
            "RGSS1x4.dll",
            "RGSS104E",
            "MyRGSS104E.dll",
        ] {
            assert_eq!(of_library(library), None, "{library}");
        }
    }
}
