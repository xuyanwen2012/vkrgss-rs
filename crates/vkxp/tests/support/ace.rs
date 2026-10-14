//! Games of RGSS3 written by test code, without an interpreter: `Game.ini`, the stand-in
//! built-in scripts of the binding's test support beside the game directory, and the script
//! archive `Data/Scripts.rvdata2` as a `Marshal` dump of version 4.8.
//!
//! The archive is an Array of `[Fixnum, String, String]`: the identifier, the name and the
//! zlib stream of the source in UTF-8. As the editor of RPG Maker VX Ace writes them, the
//! two strings carry the instance variable `:E` set to true, which labels them UTF-8; a game
//! made with [`AceGame::plain`] writes them without it.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::ZlibEncoder;
use vkxp_binding::testing::host::RGSS3_STAND_IN;

use super::TestGame;

/// A game of RGSS3 made for one test.
pub(crate) struct AceGame {
    dir: PathBuf,
}

impl AceGame {
    /// Creates the game `name` afresh below the directory of the test files. Its window
    /// title is `title` and its archive holds `scripts` as pairs of name and source, with
    /// labelled strings.
    pub(crate) fn new(name: &str, title: &str, scripts: &[(&str, &str)]) -> AceGame {
        AceGame::write(name, title, scripts, true)
    }

    /// Creates a game as [`AceGame::new`] does, with the strings of its archive written
    /// without an encoding.
    pub(crate) fn plain(name: &str, title: &str, scripts: &[(&str, &str)]) -> AceGame {
        AceGame::write(name, title, scripts, false)
    }

    fn write(name: &str, title: &str, scripts: &[(&str, &str)], labelled: bool) -> AceGame {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("rgss3-games")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
        }
        fs::create_dir_all(dir.join("Data")).expect("the game directory is made");
        let game = AceGame { dir };
        let ini = format!(
            "[Game]\r\nRTP=\r\nLibrary=System\\RGSS301.dll\r\n\
             Scripts=Data\\Scripts.rvdata2\r\nTitle={title}\r\n"
        );
        fs::write(game.dir.join("Game.ini"), ini).expect("Game.ini is written");
        fs::write(game.builtin_scripts(), RGSS3_STAND_IN).expect("the stand-in is written");
        fs::write(game.archive(), script_archive(scripts, labelled))
            .expect("the script archive is written");
        game
    }

    /// The game directory.
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// The built-in scripts file of this game, outside its directory.
    pub(crate) fn builtin_scripts(&self) -> PathBuf {
        self.dir.with_extension("rpg.rb")
    }

    /// The script archive of this game.
    pub(crate) fn archive(&self) -> PathBuf {
        self.dir.join("Data").join("Scripts.rvdata2")
    }
}

impl TestGame for AceGame {
    fn dir(&self) -> &Path {
        AceGame::dir(self)
    }

    fn builtin_scripts(&self) -> PathBuf {
        AceGame::builtin_scripts(self)
    }
}

/// The bytes of the script archive of `scripts`.
pub(crate) fn script_archive(scripts: &[(&str, &str)], labelled: bool) -> Vec<u8> {
    let mut dump = Dump::default();
    dump.bytes.extend([4, 8, b'[']);
    dump.long(scripts.len() as i64);
    for (id, (name, source)) in scripts.iter().enumerate() {
        dump.bytes.push(b'[');
        dump.long(3);
        dump.bytes.push(b'i');
        dump.long(id as i64);
        dump.string(name.as_bytes(), labelled);
        dump.string(&deflate(source.as_bytes()), labelled);
    }
    dump.bytes
}

/// The zlib stream of `bytes`, as `Zlib::Deflate.deflate` writes one.
fn deflate(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).expect("deflate into memory");
    encoder.finish().expect("deflate into memory")
}

/// A `Marshal` dump being written.
#[derive(Default)]
struct Dump {
    bytes: Vec<u8>,
    /// Whether the symbol `:E` was written, which a later use names by its index 0.
    encoding_symbol: bool,
}

impl Dump {
    /// An integer in the packed form of `Marshal`.
    fn long(&mut self, value: i64) {
        match value {
            0 => self.bytes.push(0),
            1..=122 => self.bytes.push((value + 5) as u8),
            -123..=-1 => self.bytes.push((value - 5) as u8),
            _ => {
                let bytes = value.to_le_bytes();
                let length = if value > 0 {
                    bytes.iter().rposition(|byte| *byte != 0).unwrap_or(0) + 1
                } else {
                    bytes.iter().rposition(|byte| *byte != 0xFF).unwrap_or(0) + 1
                };
                let marker = if value > 0 {
                    length as i8
                } else {
                    -(length as i8)
                };
                self.bytes.push(marker as u8);
                self.bytes.extend(&bytes[..length]);
            }
        }
    }

    /// A String, with `:E` true when `labelled`.
    fn string(&mut self, bytes: &[u8], labelled: bool) {
        if labelled {
            self.bytes.push(b'I');
        }
        self.bytes.push(b'"');
        self.long(bytes.len() as i64);
        self.bytes.extend(bytes);
        if labelled {
            self.long(1);
            if self.encoding_symbol {
                self.bytes.push(b';');
                self.long(0);
            } else {
                self.bytes.push(b':');
                self.long(1);
                self.bytes.push(b'E');
                self.encoding_symbol = true;
            }
            self.bytes.push(b'T');
        }
    }
}
