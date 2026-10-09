//! The encrypted archive of a game, RGSSAD version 1.
//!
//! After the eight header bytes the file is a run of entries: the length of the name, the
//! name, the size of the data, the data. Lengths, names and sizes are XORed with a 32-bit key
//! that advances after every length, every name byte and every size. The data of an entry is
//! XORed, one little-endian 32-bit word at a time, with a copy of the key as it stands after
//! the size; that copy advances after every word and the main key does not.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use crate::error::VfsError;
use crate::files::{candidates, components};

const HEADER: [u8; 8] = *b"RGSSAD\0\x01";
const INITIAL_KEY: u32 = 0xDEAD_CAFE;

fn advance(key: u32) -> u32 {
    key.wrapping_mul(7).wrapping_add(3)
}

/// XORs `data` with the key stream of an entry's data. The operation is its own inverse.
fn crypt(data: &mut [u8], mut key: u32) {
    for word in data.chunks_mut(4) {
        for (byte, mask) in word.iter_mut().zip(key.to_le_bytes()) {
            *byte ^= mask;
        }
        key = advance(key);
    }
}

/// The name under which an entry is indexed: lower case, components joined by `/`.
fn index_name(components: &[&str]) -> String {
    components.join("/").to_lowercase()
}

/// Where the data of one entry lies and the key that decrypts it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Entry {
    offset: u64,
    size: u32,
    key: u32,
}

/// Why the entry table could not be read.
enum Fault {
    Damaged(&'static str),
    Io(io::Error),
}

impl From<io::Error> for Fault {
    fn from(error: io::Error) -> Self {
        Fault::Io(error)
    }
}

/// The entry table while it is read: a position in a file of known length, and the key.
struct Table<'a> {
    reader: BufReader<&'a File>,
    position: u64,
    length: u64,
    key: u32,
}

impl Table<'_> {
    fn left(&self) -> u64 {
        self.length - self.position
    }

    fn word(&mut self) -> Result<u32, Fault> {
        if self.left() < 4 {
            return Err(Fault::Damaged("the entry table runs past the end"));
        }
        let mut bytes = [0; 4];
        self.reader.read_exact(&mut bytes)?;
        self.position += 4;
        let word = u32::from_le_bytes(bytes) ^ self.key;
        self.key = advance(self.key);
        Ok(word)
    }

    fn name(&mut self) -> Result<Vec<u8>, Fault> {
        let length = self.word()?;
        if u64::from(length) > self.left() {
            return Err(Fault::Damaged("an entry name runs past the end"));
        }
        let mut name = vec![0; length as usize];
        self.reader.read_exact(&mut name)?;
        self.position += u64::from(length);
        for byte in &mut name {
            *byte ^= self.key.to_le_bytes()[0];
            self.key = advance(self.key);
        }
        Ok(name)
    }

    /// Reads the size of an entry's data and steps over the data.
    fn data(&mut self) -> Result<Entry, Fault> {
        let size = self.word()?;
        if u64::from(size) > self.left() {
            return Err(Fault::Damaged("an entry declares more bytes than are left"));
        }
        let entry = Entry {
            offset: self.position,
            size,
            key: self.key,
        };
        self.reader.seek_relative(i64::from(size))?;
        self.position += u64::from(size);
        Ok(entry)
    }
}

/// Reads the entry table of `file`. Of entries whose names are the same once normalised, the
/// first is kept. An entry whose name is not valid UTF-8 is left out: no script path can name
/// it.
fn read_table(file: &File) -> Result<HashMap<String, Entry>, Fault> {
    let mut table = Table {
        reader: BufReader::new(file),
        position: 0,
        length: file.metadata()?.len(),
        key: INITIAL_KEY,
    };
    let mut header = [0; HEADER.len()];
    if table.left() < HEADER.len() as u64 {
        return Err(Fault::Damaged("not an RGSSAD version 1 archive"));
    }
    table.reader.read_exact(&mut header)?;
    table.position += HEADER.len() as u64;
    if header != HEADER {
        return Err(Fault::Damaged("not an RGSSAD version 1 archive"));
    }
    let mut entries = HashMap::new();
    while table.left() > 0 {
        let name = table.name()?;
        let entry = table.data()?;
        let Ok(name) = String::from_utf8(name) else {
            continue;
        };
        if let Some(components) = components(&name) {
            entries.entry(index_name(&components)).or_insert(entry);
        }
    }
    Ok(entries)
}

/// An open archive: the index of its entries, and the file they are read from on demand.
#[derive(Debug)]
pub(crate) struct Archive {
    path: PathBuf,
    file: Mutex<File>,
    entries: HashMap<String, Entry>,
}

impl Archive {
    /// Opens the archive at `path` and reads its entry table.
    pub(crate) fn open(path: PathBuf) -> Result<Self, VfsError> {
        let table = File::open(&path)
            .map_err(Fault::Io)
            .and_then(|file| Ok((read_table(&file)?, file)));
        match table {
            Ok((entries, file)) => Ok(Self {
                path,
                file: Mutex::new(file),
                entries,
            }),
            Err(Fault::Damaged(reason)) => Err(VfsError::DamagedArchive { path, reason }),
            Err(Fault::Io(source)) => Err(VfsError::Io { path, source }),
        }
    }

    /// Finds the entry that `components` name: as given first, then with each of `extensions`
    /// appended.
    pub(crate) fn find(&self, components: &[&str], extensions: &[&str]) -> Option<Entry> {
        let (name, directories) = components.split_last()?;
        candidates(name, extensions).find_map(|candidate| {
            let mut components = directories.to_vec();
            components.push(&candidate);
            self.entries.get(&index_name(&components)).copied()
        })
    }

    /// The decrypted bytes of `entry`.
    pub(crate) fn read(&self, entry: Entry) -> Result<Vec<u8>, VfsError> {
        let mut data = vec![0; entry.size as usize];
        let mut file = self.file.lock().unwrap_or_else(PoisonError::into_inner);
        file.seek(SeekFrom::Start(entry.offset))
            .and_then(|_| file.read_exact(&mut data))
            .map_err(|source| VfsError::Io {
                path: self.path.clone(),
                source,
            })?;
        crypt(&mut data, entry.key);
        Ok(data)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The key as it stands, which is then advanced.
    fn step(key: &mut u32) -> u32 {
        let current = *key;
        *key = key.wrapping_mul(7).wrapping_add(3);
        current
    }

    /// Builds an RGSSAD version 1 archive holding `entries`, names and contents as given.
    pub(crate) fn build(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut key: u32 = 0xDEAD_CAFE;
        let mut archive = b"RGSSAD\0\x01".to_vec();
        for (name, contents) in entries {
            let length = u32::try_from(name.len()).unwrap();
            archive.extend((length ^ step(&mut key)).to_le_bytes());
            archive.extend(name.bytes().map(|byte| byte ^ step(&mut key) as u8));
            let size = u32::try_from(contents.len()).unwrap();
            archive.extend((size ^ step(&mut key)).to_le_bytes());
            // The data has its own copy of the key.
            let mut data_key = key;
            for word in contents.chunks(4) {
                let mask = step(&mut data_key).to_le_bytes();
                archive.extend(word.iter().zip(mask).map(|(byte, mask)| byte ^ mask));
            }
        }
        archive
    }

    /// Writes `bytes` as `Game.rgssad` into a new directory and opens it.
    fn open(bytes: &[u8]) -> (tempfile::TempDir, Result<Archive, VfsError>) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Game.rgssad");
        std::fs::write(&path, bytes).unwrap();
        let archive = Archive::open(path);
        (dir, archive)
    }

    fn read(archive: &Archive, name: &str) -> Vec<u8> {
        let entry = archive.find(&components(name).unwrap(), &[]).unwrap();
        archive.read(entry).unwrap()
    }

    #[test]
    fn helper_follows_the_format() {
        let bytes = build(&[("A", b"")]);
        // The header, then the name length 1 XOR 0xDEADCAFE.
        assert_eq!(bytes[..12], *b"RGSSAD\0\x01\xFF\xCA\xAD\xDE");
        // The key is then 0xDEADCAFE * 7 + 3 = 0x16C08CF5; `A` is 0x41.
        assert_eq!(bytes[12], 0x41 ^ 0xF5);
        // And 0x16C08CF5 * 7 + 3 = 0x9F43DAB6 for the size, zero.
        assert_eq!(bytes[13..], [0xB6, 0xDA, 0x43, 0x9F]);
    }

    #[test]
    fn names_and_bytes_come_back() {
        let long: Vec<u8> = (0..=255u8).cycle().take(70_001).collect();
        let entries: [(&str, &[u8]); 6] = [
            ("Data\\Scripts.rxdata", b"scripts!"),
            ("Graphics\\Titles\\标题.png", b"12345"),
            ("Graphics\\Pictures\\タイトル.png", b"1234567"),
            ("Data\\Empty.rxdata", b""),
            ("Audio\\SE\\One.ogg", b"x"),
            ("Data\\Long.rxdata", &long),
        ];
        let (_dir, archive) = open(&build(&entries));
        let archive = archive.unwrap();
        assert_eq!(archive.entries.len(), entries.len());
        for (name, contents) in entries {
            assert_eq!(read(&archive, name), contents, "{name}");
        }
        // Entries are read on demand, in any order and more than once.
        assert_eq!(read(&archive, "Data\\Scripts.rxdata"), b"scripts!");
    }

    #[test]
    fn names_match_like_loose_files() {
        let (_dir, archive) = open(&build(&[
            ("Graphics\\Titles\\Title.png", b"png"),
            ("Graphics\\Titles\\Title.jpg", b"jpg"),
            ("Graphics\\Titles\\Bare", b"bare"),
            ("Graphics\\Titles\\Bare.png", b"extended"),
        ]));
        let archive = archive.unwrap();
        let find = |name: &str, extensions: &[&str]| {
            let entry = archive.find(&components(name).unwrap(), extensions)?;
            Some(archive.read(entry).unwrap())
        };
        assert_eq!(find("graphics/TITLES\\title.PNG", &[]).unwrap(), b"png");
        assert_eq!(
            find("Graphics/Titles/Title", &["jpg", "png"]).unwrap(),
            b"jpg"
        );
        assert_eq!(find("Graphics/Titles/Bare", &["png"]).unwrap(), b"bare");
        assert!(find("Graphics/Titles/Title", &[]).is_none());
        assert!(find("Graphics/Titles", &["png"]).is_none());
    }

    #[test]
    fn empty_archive() {
        let (_dir, archive) = open(&build(&[]));
        assert!(archive.unwrap().entries.is_empty());
    }

    #[test]
    fn wrong_header() {
        let mut version_three = build(&[("Data\\Scripts.rxdata", b"scripts")]);
        version_three[7] = 3;
        for bytes in [
            &version_three[..],
            b"RGSSAD",
            b"",
            b"PK\x03\x04 something else",
        ] {
            let (dir, archive) = open(bytes);
            let error = archive.unwrap_err();
            let path = dir.path().join("Game.rgssad");
            assert!(
                matches!(&error, VfsError::DamagedArchive { path: named, .. } if *named == path)
            );
            assert!(error.to_string().contains("Game.rgssad"));
        }
    }

    #[test]
    fn truncated() {
        let whole = build(&[
            ("Data\\Scripts.rxdata", b"scripts"),
            ("Data\\Map001.rxdata", b"a map of some length"),
        ]);
        // Every cut inside an entry leaves a name, a size or the data short.
        for length in [
            whole.len() - 1,
            whole.len() - 20,
            whole.len() - 22,
            60,
            20,
            10,
        ] {
            let (dir, archive) = open(&whole[..length]);
            let error = archive.unwrap_err();
            let path = dir.path().join("Game.rgssad");
            assert!(
                matches!(&error, VfsError::DamagedArchive { path: named, .. } if *named == path),
                "{length}"
            );
            assert!(error.to_string().contains("Game.rgssad"));
        }
    }
}
