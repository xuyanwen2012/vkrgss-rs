//! The encrypted archive of a game (`Game.rgssad`, RGSSAD version 1), and
//! `cargo xtask pack-rgssad`, which packs a copy of a game into one.
//!
//! After the eight bytes of the header the file is a sequence of entries. A 32-bit key
//! starts at `0xDEADCAFE` and is advanced by `key * 7 + 3` after every use: the length of a
//! name is XORed with it, then each byte of the name with its low byte, then the size of the
//! data. The data is XORed word by word, little endian, with a copy of the key that is
//! advanced after every word, while the key itself stays as it was after the size.
//!
//! This reader and writer are independent of the ones in `vkxp-vfs`, so that the harness and
//! the runtime check each other.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

/// The name of the archive in a game directory.
pub(crate) const FILE_NAME: &str = "Game.rgssad";

const HEADER: [u8; 8] = *b"RGSSAD\0\x01";
const INITIAL_KEY: u32 = 0xDEAD_CAFE;

/// The folders of a game that `pack` moves into the archive.
const PACKED_FOLDERS: [&str; 2] = ["Data", "Graphics"];

/// One file of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The name as stored: a path with backslashes, usually UTF-8.
    pub name: Vec<u8>,
    pub data: Vec<u8>,
}

/// The key as it stands, which is then advanced.
fn step(key: &mut u32) -> u32 {
    let current = *key;
    *key = key.wrapping_mul(7).wrapping_add(3);
    current
}

/// XORs `data` with the key stream that starts at `key`. The operation is its own inverse.
fn crypt(data: &[u8], mut key: u32) -> impl Iterator<Item = u8> {
    data.chunks(4).flat_map(move |word| {
        let mask = step(&mut key).to_le_bytes();
        word.iter().zip(mask).map(|(byte, mask)| byte ^ mask)
    })
}

/// Reads an archive. The entries are in the order of the file.
pub(crate) fn read(bytes: &[u8]) -> Result<Vec<Entry>> {
    ensure!(
        bytes.starts_with(&HEADER),
        "not an RGSSAD version 1 archive"
    );
    let mut rest = &bytes[HEADER.len()..];
    let mut take = |count: usize| {
        let (taken, left) = rest
            .split_at_checked(count)
            .context("the archive ends inside an entry")?;
        rest = left;
        anyhow::Ok(taken)
    };
    let mut key = INITIAL_KEY;
    let mut entries = Vec::new();
    loop {
        let Ok(length) = take(4) else {
            // Fewer than four bytes are left: the end of the file, if none is.
            ensure!(take(1).is_err(), "the archive ends inside an entry");
            return Ok(entries);
        };
        let length = u32::from_le_bytes(length.try_into().expect("four bytes")) ^ step(&mut key);
        let name = take(length as usize)?
            .iter()
            .map(|byte| byte ^ step(&mut key) as u8)
            .collect();
        let size = u32::from_le_bytes(take(4)?.try_into().expect("four bytes")) ^ step(&mut key);
        let data = crypt(take(size as usize)?, key).collect();
        entries.push(Entry { name, data });
    }
}

/// Writes an archive holding `entries` in their order.
pub(crate) fn write(entries: &[Entry]) -> Vec<u8> {
    let mut key = INITIAL_KEY;
    let mut bytes = HEADER.to_vec();
    for entry in entries {
        let length = u32::try_from(entry.name.len()).expect("a name is shorter than 4 GiB");
        bytes.extend((length ^ step(&mut key)).to_le_bytes());
        bytes.extend(entry.name.iter().map(|byte| byte ^ step(&mut key) as u8));
        let size = u32::try_from(entry.data.len()).expect("a file is shorter than 4 GiB");
        bytes.extend((size ^ step(&mut key)).to_le_bytes());
        bytes.extend(crypt(&entry.data, key));
    }
    bytes
}

/// Whether the entry name `stored` is the path `wanted`, without regard to the case of the
/// letters and with either separator.
pub(crate) fn names_match(stored: &[u8], wanted: &str) -> bool {
    let normal = |name: &str| name.replace('/', "\\").to_lowercase();
    normal(&String::from_utf8_lossy(stored)) == normal(wanted)
}

/// `cargo xtask pack-rgssad`: packs the `Data` and `Graphics` folders of the game in `game`
/// into `Game.rgssad` and removes them. The directory is changed in place, so it is meant to
/// be a copy.
pub(crate) fn pack(game: &Path) -> Result<()> {
    let children = children(game)?;
    let child = |name: &str| {
        children
            .iter()
            .find(|(found, _)| found.eq_ignore_ascii_case(name))
    };
    ensure!(
        child("Game.ini").is_some(),
        "{}: no Game.ini, so this is not a game",
        game.display()
    );
    if let Some((_, archive)) = child(FILE_NAME) {
        bail!("{}: the game is already packed", archive.display());
    }
    let mut entries = Vec::new();
    let mut folders = Vec::new();
    for (name, folder) in PACKED_FOLDERS.iter().filter_map(|name| child(name)) {
        collect(folder, name, &mut entries)?;
        folders.push(folder);
    }
    let archive = game.join(FILE_NAME);
    fs::write(&archive, write(&entries)).with_context(|| archive.display().to_string())?;
    for folder in folders {
        fs::remove_dir_all(folder).with_context(|| folder.display().to_string())?;
    }
    eprintln!(
        "pack-rgssad: wrote {} with {} files",
        archive.display(),
        entries.len()
    );
    Ok(())
}

/// The entries of `dir` with their names, in the order of the names.
fn children(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut children = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let path = entry.with_context(|| dir.display().to_string())?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("{}: the name is not UTF-8", path.display()))?;
        children.push((name.to_owned(), path));
    }
    children.sort();
    Ok(children)
}

/// Adds every file below `dir` to `entries`, under `prefix` and its path below `dir`.
fn collect(dir: &Path, prefix: &str, entries: &mut Vec<Entry>) -> Result<()> {
    for (name, path) in children(dir)? {
        let name = format!("{prefix}\\{name}");
        if path.is_dir() {
            collect(&path, &name, entries)?;
        } else {
            let data = fs::read(&path).with_context(|| path.display().to_string())?;
            entries.push(Entry {
                name: name.into_bytes(),
                data,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// A game with files in both folders, in subfolders, of lengths around a word.
    fn generated(dir: &Path) -> BTreeMap<&'static str, Vec<u8>> {
        let files = BTreeMap::from([
            ("Data\\Scripts.rxdata", b"\x04\x08[\x00".to_vec()),
            ("Data\\Map001.rxdata", (0..=255).collect()),
            ("Data\\Empty.rxdata", Vec::new()),
            ("Graphics\\Titles\\题目.png", b"12345".to_vec()),
            ("Graphics\\Tilesets\\Deep\\a.png", b"1234567".to_vec()),
        ]);
        for (name, data) in &files {
            let path = dir.join(name.replace('\\', "/"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, data).unwrap();
        }
        fs::write(dir.join("Game.ini"), "[Game]\r\n").unwrap();
        fs::create_dir_all(dir.join("Audio")).unwrap();
        fs::write(dir.join("Audio/a.ogg"), "loose").unwrap();
        files
    }

    #[test]
    fn the_start_of_an_archive_is_the_header_and_the_first_length_under_the_first_key() {
        let entry = Entry {
            name: b"a".to_vec(),
            data: b"12345".to_vec(),
        };
        let bytes = write(std::slice::from_ref(&entry));
        // 1 XOR 0xDEADCAFE, little endian.
        assert_eq!(bytes[..12], *b"RGSSAD\0\x01\xFF\xCA\xAD\xDE");
        assert_eq!(bytes.len(), 8 + 4 + 1 + 4 + 5);
        assert_eq!(read(&bytes).unwrap(), [entry]);
    }

    #[test]
    fn a_file_that_is_no_complete_archive_is_refused() {
        let entry = Entry {
            name: b"Data\\a".to_vec(),
            data: b"12345".to_vec(),
        };
        let bytes = write(&[entry]);
        assert!(read(b"RGSSAD\0\x03").is_err());
        assert!(read(&bytes[..4]).is_err());
        for length in HEADER.len() + 1..bytes.len() {
            assert!(read(&bytes[..length]).is_err(), "{length}");
        }
        assert_eq!(read(&HEADER).unwrap(), []);
    }

    #[test]
    fn names_are_compared_without_case_and_with_either_separator() {
        assert!(names_match(b"Data\\Scripts.rxdata", "data/SCRIPTS.rxdata"));
        assert!(names_match(b"data/scripts.rxdata", "Data\\Scripts.rxdata"));
        assert!(!names_match(b"Data\\Scripts.rxdata", "Scripts.rxdata"));
    }

    /// The round-trip scenario of `oracle-harness`.
    #[test]
    fn a_packed_game_holds_every_file_of_the_two_folders_and_the_folders_are_gone() {
        let dir = tempfile::tempdir().unwrap();
        let files = generated(dir.path());

        pack(dir.path()).unwrap();

        let entries = read(&fs::read(dir.path().join(FILE_NAME)).unwrap()).unwrap();
        let found: BTreeMap<&str, Vec<u8>> = entries
            .iter()
            .map(|entry| (str::from_utf8(&entry.name).unwrap(), entry.data.clone()))
            .collect();
        assert_eq!(entries.len(), files.len());
        assert_eq!(found, files);
        assert!(!dir.path().join("Data").exists());
        assert!(!dir.path().join("Graphics").exists());
        assert_eq!(fs::read(dir.path().join("Audio/a.ogg")).unwrap(), b"loose");
    }

    /// The already-packed scenario of `oracle-harness`; `main` turns the error into the
    /// exit status.
    #[test]
    fn a_game_that_has_an_archive_or_no_game_ini_is_refused_and_left_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let files = generated(dir.path());
        fs::write(dir.path().join("game.RGSSAD"), "an archive").unwrap();

        let error = pack(dir.path()).unwrap_err();

        assert!(format!("{error:#}").contains("already packed"), "{error:#}");
        let archive = fs::read(dir.path().join("game.RGSSAD")).unwrap();
        assert_eq!(archive, b"an archive");
        assert!(!dir.path().join(FILE_NAME).exists());
        for (name, data) in &files {
            let path = dir.path().join(name.replace('\\', "/"));
            assert_eq!(fs::read(path).unwrap(), *data, "{name}");
        }

        fs::remove_file(dir.path().join("game.RGSSAD")).unwrap();
        fs::remove_file(dir.path().join("Game.ini")).unwrap();
        let error = pack(dir.path()).unwrap_err();
        assert!(format!("{error:#}").contains("Game.ini"), "{error:#}");
        assert!(!dir.path().join(FILE_NAME).exists());
        assert!(dir.path().join("Data/Scripts.rxdata").is_file());
    }
}
