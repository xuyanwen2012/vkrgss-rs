//! The library of imported games and run-time packages in the app's private files directory.
//!
//! - `games/<number>/`: a game, under a number of its own; its title is the `Title` of its
//!   `Game.ini`.
//! - `rtp/<name>/`: a run-time package, under the name games give it in their `Game.ini`:
//!   `Standard` for XP, `RPGVX` for VX and `RPGVXAce` for VX Ace, told by the folders it
//!   holds ([`rtp_name`]). A later import of the same package replaces it.
//! - `played`: the numbers of the games, the last one played first.
//!
//! An import unpacks into a directory of its own beside the others ([`crate::unpack`]) and
//! gives it its place only once everything is written; one that fails leaves nothing behind.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vkrgss_vfs::{GameIni, VfsError};

use crate::unpack::{Kind, unpack};

const GAMES: &str = "games";
const RTPS: &str = "rtp";
const PLAYED: &str = "played";

/// Where an import is unpacked before it gets its place.
const IMPORTING: &str = ".import";

/// An imported game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    /// Its number, the name of its directory.
    pub id: u64,
    pub dir: PathBuf,
    /// The `Title` of its `Game.ini`, or a name made from its number when that has none.
    pub title: String,
}

/// What an import added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Imported {
    Game(Game),
    /// A run-time package, by its name.
    Rtp(String),
}

/// The library in a directory.
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
}

/// The name of the run-time package unpacked in `dir`, told by the folders of `Graphics`
/// that only one runtime's package has: `Autotiles` (XP), `Battlebacks1` (VX Ace), or else
/// `System` (VX). Letter case does not matter.
pub fn rtp_name(dir: &Path) -> Option<&'static str> {
    let graphics = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.file_name().eq_ignore_ascii_case("Graphics"))?
        .path();
    let folders: Vec<String> = fs::read_dir(graphics)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase())
        .collect();
    let has = |name: &str| folders.iter().any(|folder| folder == name);
    if has("autotiles") {
        Some("Standard")
    } else if has("battlebacks1") {
        Some("RPGVXAce")
    } else if has("system") {
        Some("RPGVX")
    } else {
        None
    }
}

impl Library {
    /// The library in `root`, the app's private files directory.
    pub fn new(root: &Path) -> Library {
        Library {
            root: root.to_owned(),
        }
    }

    fn games_dir(&self) -> PathBuf {
        self.root.join(GAMES)
    }

    fn rtps_dir(&self) -> PathBuf {
        self.root.join(RTPS)
    }

    /// The numbers in `played`, the last one played first.
    fn played(&self) -> Vec<u64> {
        fs::read_to_string(self.root.join(PLAYED))
            .unwrap_or_default()
            .split_whitespace()
            .filter_map(|id| id.parse().ok())
            .collect()
    }

    fn write_played(&self, played: &[u64]) -> Result<()> {
        let text: Vec<String> = played.iter().map(u64::to_string).collect();
        let path = self.root.join(PLAYED);
        fs::write(&path, text.join("\n"))
            .with_context(|| format!("cannot write {}", path.display()))
    }

    /// The imported games: those played, the last one played first, then the others, the
    /// last one imported first.
    pub fn games(&self) -> Vec<Game> {
        let mut games: Vec<Game> = fs::read_dir(self.games_dir())
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let id = entry.file_name().to_str()?.parse().ok()?;
                let dir = entry.path();
                let title = GameIni::read(&dir, None)
                    .map(|ini| ini.title)
                    .unwrap_or_default();
                let title = match title.trim() {
                    "" => format!("Game {id}"),
                    title => title.to_owned(),
                };
                Some(Game { id, dir, title })
            })
            .collect();
        let played = self.played();
        games.sort_by_key(|game| {
            let rank = played.iter().position(|id| *id == game.id);
            (rank.is_none(), rank, std::cmp::Reverse(game.id))
        });
        games
    }

    /// The imported run-time packages, by name.
    pub fn rtps(&self) -> BTreeMap<String, PathBuf> {
        fs::read_dir(self.rtps_dir())
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_str()?.to_owned();
                (!name.starts_with('.') && entry.path().is_dir()).then(|| (name, entry.path()))
            })
            .collect()
    }

    /// Records that the game `id` is played now.
    pub fn play(&self, id: u64) -> Result<()> {
        let mut played = self.played();
        played.retain(|known| *known != id);
        played.insert(0, id);
        self.write_played(&played)
    }

    /// Deletes the game `id` with everything in its directory, its saves too.
    pub fn delete(&self, id: u64) -> Result<()> {
        let dir = self.games_dir().join(id.to_string());
        fs::remove_dir_all(&dir).with_context(|| format!("cannot delete {}", dir.display()))?;
        let mut played = self.played();
        played.retain(|known| *known != id);
        self.write_played(&played)
    }

    /// Imports the zip `zip`, which holds a game or a run-time package as `kind` says.
    pub fn import(&self, zip: &Path, kind: Kind) -> Result<Imported> {
        let parent = match kind {
            Kind::Game => self.games_dir(),
            Kind::Rtp => self.rtps_dir(),
        };
        fs::create_dir_all(&parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
        let unpacked = parent.join(IMPORTING);
        if unpacked.exists() {
            fs::remove_dir_all(&unpacked)
                .with_context(|| format!("cannot remove {}", unpacked.display()))?;
        }
        let placed = unpack(zip, &unpacked, kind).and_then(|()| self.place(&unpacked, kind));
        if placed.is_err() && unpacked.exists() {
            let _ = fs::remove_dir_all(&unpacked);
        }
        placed
    }

    /// Gives the unpacked directory `unpacked` its place.
    fn place(&self, unpacked: &Path, kind: Kind) -> Result<Imported> {
        match kind {
            Kind::Game => {
                let ini = GameIni::read(unpacked, None).map_err(|error| match error {
                    VfsError::IniNotFound { .. } => anyhow::anyhow!(
                        "the zip holds no Game.ini, neither at its top nor in its one directory"
                    ),
                    error => anyhow::anyhow!("the Game.ini of the zip: {error}"),
                })?;
                let id = self.games().iter().map(|game| game.id).max().unwrap_or(0) + 1;
                let dir = self.games_dir().join(id.to_string());
                fs::rename(unpacked, &dir)
                    .with_context(|| format!("cannot move the game to {}", dir.display()))?;
                let title = match ini.title.trim() {
                    "" => format!("Game {id}"),
                    title => title.to_owned(),
                };
                Ok(Imported::Game(Game { id, dir, title }))
            }
            Kind::Rtp => {
                let Some(name) = rtp_name(unpacked) else {
                    bail!(
                        "the zip holds no run-time package: no folder of Graphics tells XP \
                         (Autotiles), VX (System) or VX Ace (Battlebacks1)"
                    );
                };
                let dir = self.rtps_dir().join(name);
                if dir.exists() {
                    fs::remove_dir_all(&dir)
                        .with_context(|| format!("cannot replace {}", dir.display()))?;
                }
                fs::rename(unpacked, &dir)
                    .with_context(|| format!("cannot move the package to {}", dir.display()))?;
                Ok(Imported::Rtp(name.to_owned()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unpack::tests::{file, zip};

    fn write_zip(dir: &Path, name: &str, items: &[crate::unpack::tests::Item<'_>]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, zip(items)).unwrap();
        path
    }

    /// Scenario "Import a zip": the game is listed under the title of its `Game.ini` and its
    /// directory is the game directory.
    #[test]
    fn an_imported_game_is_listed_under_its_title() {
        let root = tempfile::tempdir().unwrap();
        let library = Library::new(root.path());
        assert!(library.games().is_empty());
        let (title, _, _) = encoding_rs::GBK
            .encode("[Game]\r\nScripts=Data\\Scripts.rxdata\r\nTitle=群战示范工程\r\n");
        let zip = write_zip(
            root.path(),
            "a.zip",
            &[
                file("群战示范工程/Game.ini", &title),
                file("群战示范工程/Data/Scripts.rxdata", b"s"),
            ],
        );
        let imported = library.import(&zip, Kind::Game).unwrap();
        let game = Game {
            id: 1,
            dir: root.path().join("games/1"),
            title: "群战示范工程".to_owned(),
        };
        assert_eq!(imported, Imported::Game(game.clone()));
        assert_eq!(library.games(), std::slice::from_ref(&game));
        assert!(game.dir.join("Data/Scripts.rxdata").is_file());
        assert!(!root.path().join("games/.import").exists());

        // The next one gets the next number and comes first until one is played.
        let zip = write_zip(
            root.path(),
            "b.zip",
            &[file(
                "Game.ini",
                b"[Game]\r\nScripts=Data\\Scripts.rxdata\r\n",
            )],
        );
        library.import(&zip, Kind::Game).unwrap();
        let titles = |library: &Library| -> Vec<String> {
            library.games().into_iter().map(|game| game.title).collect()
        };
        assert_eq!(titles(&library), ["Game 2", "群战示范工程"]);
        library.play(1).unwrap();
        assert_eq!(titles(&library), ["群战示范工程", "Game 2"]);
        library.play(2).unwrap();
        assert_eq!(titles(&library), ["Game 2", "群战示范工程"]);
        library.delete(2).unwrap();
        assert_eq!(titles(&library), ["群战示范工程"]);
        assert!(!root.path().join("games/2").exists());
    }

    /// Scenario "A path that leaves the directory", for the library.
    #[test]
    fn a_refused_zip_leaves_nothing_behind() {
        let root = tempfile::tempdir().unwrap();
        let library = Library::new(root.path());
        let zip = write_zip(
            root.path(),
            "bad.zip",
            &[file("Game.ini", b"[Game]"), file("../escape.txt", b"x")],
        );
        let error = library.import(&zip, Kind::Game).unwrap_err();
        assert!(error.to_string().contains("leaves"), "{error}");
        assert!(library.games().is_empty());
        assert_eq!(
            fs::read_dir(root.path().join("games")).unwrap().count(),
            0,
            "nothing is left in the games directory"
        );
        assert!(!root.path().join("escape.txt").exists());

        let zip = write_zip(root.path(), "none.zip", &[file("readme.txt", b"x")]);
        let error = library.import(&zip, Kind::Game).unwrap_err();
        assert!(error.to_string().contains("no Game.ini"), "{error}");
        assert_eq!(fs::read_dir(root.path().join("games")).unwrap().count(), 0);
    }

    #[test]
    fn a_run_time_package_is_named_by_its_folders_and_replaced_by_a_later_import() {
        let root = tempfile::tempdir().unwrap();
        let library = Library::new(root.path());
        for (folder, name) in [
            ("Autotiles", "Standard"),
            ("System", "RPGVX"),
            ("Battlebacks1", "RPGVXAce"),
        ] {
            let path = format!("RTP/Graphics/{folder}/x.png");
            let zip = write_zip(root.path(), "rtp.zip", &[file(&path, b"1")]);
            assert_eq!(
                library.import(&zip, Kind::Rtp).unwrap(),
                Imported::Rtp(name.to_owned())
            );
        }
        let rtps = library.rtps();
        assert_eq!(
            rtps.keys().collect::<Vec<_>>(),
            ["RPGVX", "RPGVXAce", "Standard"]
        );
        assert!(rtps["Standard"].join("Graphics/Autotiles/x.png").is_file());

        let zip = write_zip(
            root.path(),
            "rtp.zip",
            &[file("Graphics/Autotiles/y.png", b"2")],
        );
        library.import(&zip, Kind::Rtp).unwrap();
        let standard = &library.rtps()["Standard"];
        assert!(standard.join("Graphics/Autotiles/y.png").is_file());
        assert!(!standard.join("Graphics/Autotiles/x.png").exists());

        let zip = write_zip(root.path(), "rtp.zip", &[file("Audio/BGM/a.mid", b"3")]);
        let error = library.import(&zip, Kind::Rtp).unwrap_err();
        assert!(error.to_string().contains("no run-time package"), "{error}");
        assert_eq!(library.rtps().len(), 3);
    }
}
