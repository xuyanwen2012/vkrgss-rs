//! The game configuration file `Game.ini`.

use std::io;
use std::path::Path;

use crate::dir::Listing;
use crate::error::VfsError;
use crate::runtime::Runtime;

/// A text encoding the configuration file can be stored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    Utf8,
    Gbk,
    ShiftJis,
    Big5,
}

impl Encoding {
    /// Encodings tried in this order when the file is not valid UTF-8. Short titles are often
    /// valid in more than one of them, so the order is a default, not a detection.
    pub const LEGACY_ORDER: [Encoding; 3] = [Encoding::Gbk, Encoding::ShiftJis, Encoding::Big5];

    /// The name of the encoding, as used in messages.
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Gbk => "GBK",
            Encoding::ShiftJis => "Shift-JIS",
            Encoding::Big5 => "Big5",
        }
    }

    /// Decodes `bytes`, or returns `None` if any of them is invalid in this encoding. A UTF-8
    /// byte-order mark is dropped.
    fn decode(self, bytes: &[u8]) -> Option<String> {
        let decoder = match self {
            Encoding::Utf8 => {
                let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
                return std::str::from_utf8(bytes).ok().map(str::to_owned);
            }
            Encoding::Gbk => encoding_rs::GBK,
            Encoding::ShiftJis => encoding_rs::SHIFT_JIS,
            Encoding::Big5 => encoding_rs::BIG5,
        };
        decoder
            .decode_without_bom_handling_and_without_replacement(bytes)
            .map(|text| text.into_owned())
    }
}

/// Decodes the contents of a configuration file: with `encoding` if the caller names one,
/// otherwise as UTF-8 if valid, otherwise with the first legacy encoding that accepts the
/// whole file.
fn decode(bytes: &[u8], encoding: Option<Encoding>) -> Result<String, VfsError> {
    let candidates: &[Encoding] = match &encoding {
        Some(named) => std::slice::from_ref(named),
        None => &[
            Encoding::Utf8,
            Encoding::LEGACY_ORDER[0],
            Encoding::LEGACY_ORDER[1],
            Encoding::LEGACY_ORDER[2],
        ],
    };
    candidates
        .iter()
        .find_map(|candidate| candidate.decode(bytes))
        .ok_or_else(|| VfsError::IniEncoding {
            tried: candidates
                .iter()
                .map(|candidate| candidate.name())
                .collect::<Vec<_>>()
                .join(", "),
        })
}

/// The values of the `Game` section of `Game.ini`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameIni {
    /// `Library`: the name of the original runtime DLL. Empty if absent.
    pub library: String,
    /// `Scripts`: the script-style path of the script archive.
    pub scripts: String,
    /// `Title`: the window title. Empty if absent.
    pub title: String,
    /// `RTP`: the name of the run-time package of a game of RGSS2. Empty if absent.
    pub rtp: String,
    /// `RTP1`, `RTP2` and `RTP3`: names of the run-time packages of a game of RGSS1. Empty
    /// where absent.
    pub rtps: [String; 3],
}

impl GameIni {
    /// The names of the run-time packages of the game, in order, as a game of `runtime`
    /// names them: `RTP1` to `RTP3` in RGSS1 and the one value `RTP` otherwise. Empty names
    /// are among them.
    pub fn rtp_names(&self, runtime: Runtime) -> &[String] {
        match runtime {
            Runtime::Rgss1 => &self.rtps,
            Runtime::Rgss2 | Runtime::Rgss3 => std::slice::from_ref(&self.rtp),
        }
    }

    /// Reads `Game.ini` from `game_dir`, whatever the letter case of its name. `encoding`
    /// overrides the detection of the text encoding.
    pub fn read(game_dir: &Path, encoding: Option<Encoding>) -> Result<Self, VfsError> {
        let not_found = || VfsError::IniNotFound {
            dir: game_dir.to_owned(),
        };
        let listing = match Listing::read(game_dir) {
            Ok(listing) => listing,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(not_found()),
            Err(source) => {
                return Err(VfsError::Io {
                    path: game_dir.to_owned(),
                    source,
                });
            }
        };
        let path = listing
            .matches("Game.ini")
            .into_iter()
            .map(|name| game_dir.join(name))
            .find(|path| path.is_file())
            .ok_or_else(not_found)?;
        let bytes = std::fs::read(&path).map_err(|source| VfsError::Io { path, source })?;
        Self::parse(&bytes, encoding)
    }

    /// Parses the contents of a configuration file. `encoding` overrides the detection of the
    /// text encoding.
    pub fn parse(bytes: &[u8], encoding: Option<Encoding>) -> Result<Self, VfsError> {
        let text = decode(bytes, encoding)?;
        let mut ini = GameIni {
            library: String::new(),
            scripts: String::new(),
            title: String::new(),
            rtp: String::new(),
            rtps: Default::default(),
        };
        let mut seen = [false; 7];
        let mut in_game_section = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with(';') {
                continue;
            }
            if let Some(rest) = line.strip_prefix('[') {
                let section = rest.split(']').next().unwrap_or(rest);
                in_game_section = section.trim().eq_ignore_ascii_case("Game");
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if !in_game_section {
                continue;
            }
            let slots: [(&str, &mut String); 7] = {
                let [rtp1, rtp2, rtp3] = &mut ini.rtps;
                [
                    ("Library", &mut ini.library),
                    ("Scripts", &mut ini.scripts),
                    ("Title", &mut ini.title),
                    ("RTP", &mut ini.rtp),
                    ("RTP1", rtp1),
                    ("RTP2", rtp2),
                    ("RTP3", rtp3),
                ]
            };
            // As in the original's profile reader, the first occurrence of a key wins.
            for ((name, slot), seen) in slots.into_iter().zip(&mut seen) {
                if !*seen && key.trim().eq_ignore_ascii_case(name) {
                    *slot = value.trim().to_owned();
                    *seen = true;
                }
            }
        }
        if ini.scripts.is_empty() {
            return Err(VfsError::MissingKey { key: "Scripts" });
        }
        Ok(ini)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPICAL: &str = "[Game]\nLibrary=RGSS103J.dll\nScripts=Data\\Scripts.rxdata\nTitle=Project1\nRTP1=Standard\n";

    fn typical() -> GameIni {
        GameIni {
            library: "RGSS103J.dll".into(),
            scripts: "Data\\Scripts.rxdata".into(),
            title: "Project1".into(),
            rtp: String::new(),
            rtps: ["Standard".into(), String::new(), String::new()],
        }
    }

    fn with_title(title: &[u8]) -> Vec<u8> {
        let mut bytes = b"[Game]\r\nScripts=Data\\Scripts.rxdata\r\nTitle=".to_vec();
        bytes.extend_from_slice(title);
        bytes.extend_from_slice(b"\r\n");
        bytes
    }

    #[test]
    fn typical_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.ini"), TYPICAL).unwrap();
        assert_eq!(GameIni::read(dir.path(), None).unwrap(), typical());
    }

    #[test]
    fn file_of_vx() {
        let text =
            "[Game]\r\nRTP=RPGVX\r\nLibrary=RGSS202E.dll\r\nScripts=Data\\Scripts.rvdata\r\n";
        let ini = GameIni::parse(text.as_bytes(), None).unwrap();
        assert_eq!(ini.rtp, "RPGVX");
        assert_eq!(ini.rtps, [const { String::new() }; 3]);
        assert_eq!(ini.library, "RGSS202E.dll");
        assert_eq!(ini.scripts, "Data\\Scripts.rvdata");
        assert_eq!(ini.rtp_names(Runtime::Rgss2), ["RPGVX"]);
        assert_eq!(ini.rtp_names(Runtime::Rgss1), ["", "", ""]);
    }

    #[test]
    fn case_and_line_endings() {
        let dir = tempfile::tempdir().unwrap();
        let text = "; comment\r\n[Other]\r\nscripts=wrong\r\n[GAME]\r\n  library = RGSS103J.dll \r\n\
                    scripts=Data\\Scripts.rxdata\r\nTITLE=Project1\r\nrtp1=Standard\r\nRTP2=\r\n";
        std::fs::write(dir.path().join("game.INI"), text).unwrap();
        assert_eq!(GameIni::read(dir.path(), None).unwrap(), typical());
    }

    #[test]
    fn no_scripts_entry() {
        let error = GameIni::parse(b"[Game]\nTitle=Project1\n", None).unwrap_err();
        assert!(matches!(error, VfsError::MissingKey { key: "Scripts" }));
        assert!(error.to_string().contains("Scripts"));
        // A key outside the Game section does not count.
        let error = GameIni::parse(b"[Other]\nScripts=a\n[Game]\n", None).unwrap_err();
        assert!(matches!(error, VfsError::MissingKey { key: "Scripts" }));
    }

    #[test]
    fn missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let error = GameIni::read(dir.path(), None).unwrap_err();
        assert!(matches!(error, VfsError::IniNotFound { .. }));
        let error = GameIni::read(&dir.path().join("absent"), None).unwrap_err();
        assert!(matches!(error, VfsError::IniNotFound { .. }));
    }

    #[test]
    fn first_occurrence_of_a_key_wins() {
        let ini = GameIni::parse(b"[Game]\nScripts=a\nScripts=b\n", None).unwrap();
        assert_eq!(ini.scripts, "a");
    }

    #[test]
    fn utf8_title() {
        let ini = GameIni::parse(&with_title("工程 プロジェクト".as_bytes()), None).unwrap();
        assert_eq!(ini.title, "工程 プロジェクト");
    }

    #[test]
    fn utf8_byte_order_mark_is_ignored() {
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice(&with_title("工程".as_bytes()));
        let ini = GameIni::parse(&bytes, None).unwrap();
        assert_eq!(ini.title, "工程");
        assert_eq!(ini.scripts, "Data\\Scripts.rxdata");
    }

    #[test]
    fn gbk_title() {
        let ini = GameIni::parse(&with_title(&[0xB9, 0xA4, 0xB3, 0xCC]), None).unwrap();
        assert_eq!(ini.title, "\u{5DE5}\u{7A0B}");
    }

    #[test]
    fn caller_names_the_encoding() {
        // "日本" in Shift-JIS; the same bytes are two valid GBK characters.
        let bytes = with_title(&[0x93, 0xFA, 0x96, 0x7B]);
        let detected = GameIni::parse(&bytes, None).unwrap();
        assert_ne!(detected.title, "日本");
        assert_eq!(detected.title.chars().count(), 2);
        let named = GameIni::parse(&bytes, Some(Encoding::ShiftJis)).unwrap();
        assert_eq!(named.title, "日本");
    }

    #[test]
    fn later_legacy_encodings_are_reached() {
        // A lone half-width katakana byte is invalid in GBK and valid in Shift-JIS.
        let ini = GameIni::parse(&with_title(&[0xB1]), None).unwrap();
        assert_eq!(ini.title, "\u{FF71}");
    }

    #[test]
    fn undecodable_file_is_an_error() {
        let bytes = with_title(&[0xFF, 0xFF]);
        let error = GameIni::parse(&bytes, None).unwrap_err();
        assert!(matches!(error, VfsError::IniEncoding { .. }));
        let error = GameIni::parse(&bytes, Some(Encoding::Utf8)).unwrap_err();
        assert!(error.to_string().contains("UTF-8"));
    }
}
