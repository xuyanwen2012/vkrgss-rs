//! The encrypted archive of a game: RGSSAD version 1 (`Game.rgssad`, `Game.rgss2a`) or
//! version 3 (`Game.rgss3a`).
//!
//! Version 1: after the eight header bytes the file is a run of entries: the length of the
//! name, the name, the size of the data, the data. Lengths, names and sizes are XORed with a
//! 32-bit key that advances after every length, every name byte and every size. The data of
//! an entry is XORed, one little-endian 32-bit word at a time, with a copy of the key as it
//! stands after the size; that copy advances after every word and the main key does not.
//!
//! Version 3: after the eight header bytes come a 32-bit seed and a table. The table key is
//! seed * 9 + 3 and stays the same for the whole table. A record is four words XORed with
//! it (the offset of the data in the file, its size, its key, the length of the name) and
//! the name, byte i XORed with byte i mod 4 of the table key. The table ends at a record
//! whose offset is 0; nothing behind that word is read as table. The data of an entry lies
//! at its offset and is XORed word by word as in version 1, starting at the key of its
//! record.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use crate::error::VfsError;
use crate::files::{candidates, components};

const HEADER: [u8; 8] = *b"RGSSAD\0\x01";
const HEADER_THREE: [u8; 8] = *b"RGSSAD\0\x03";
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

/// The version of RGSSAD an archive is read as. The runtime of the game chooses it, not the
/// header of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Version {
    /// `Game.rgssad` and `Game.rgss2a`.
    One,
    /// `Game.rgss3a`.
    Three,
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

    /// A word of the table of version 3, whose key does not advance.
    fn word_three(&mut self) -> Result<u32, Fault> {
        if self.left() < 4 {
            return Err(Fault::Damaged(
                "the entry table of version 3 runs past the end",
            ));
        }
        let mut bytes = [0; 4];
        self.reader.read_exact(&mut bytes)?;
        self.position += 4;
        Ok(u32::from_le_bytes(bytes) ^ self.key)
    }

    /// The name of a record of version 3, `length` bytes long.
    fn name_three(&mut self, length: u32) -> Result<Vec<u8>, Fault> {
        if u64::from(length) > self.left() {
            return Err(Fault::Damaged(
                "an entry name of version 3 runs past the end",
            ));
        }
        let mut name = vec![0; length as usize];
        self.reader.read_exact(&mut name)?;
        self.position += u64::from(length);
        for (byte, mask) in name
            .iter_mut()
            .zip(self.key.to_le_bytes().into_iter().cycle())
        {
            *byte ^= mask;
        }
        Ok(name)
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

/// Reads the entry table of `file` into an [`Index`]. An entry whose name is not valid UTF-8
/// is left out: no script path can name it.
fn read_table(file: &File) -> Result<Index, Fault> {
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
    let mut entries = Index::default();
    while table.left() > 0 {
        let name = table.name()?;
        let entry = table.data()?;
        let Ok(name) = String::from_utf8(name) else {
            continue;
        };
        entries.insert(&name, entry);
    }
    Ok(entries)
}

/// Reads the entry table of `file` as that of RGSSAD version 3, with the rules of
/// [`read_table`] for names. The data of an entry is found by its offset alone.
fn read_table_three(file: &File) -> Result<Index, Fault> {
    let mut table = Table {
        reader: BufReader::new(file),
        position: 0,
        length: file.metadata()?.len(),
        key: 0,
    };
    let mut header = [0; HEADER_THREE.len()];
    if table.left() < HEADER_THREE.len() as u64 + 4 {
        return Err(Fault::Damaged("not an RGSSAD version 3 archive"));
    }
    table.reader.read_exact(&mut header)?;
    table.position += HEADER_THREE.len() as u64;
    if header != HEADER_THREE {
        return Err(Fault::Damaged("not an RGSSAD version 3 archive"));
    }
    let seed = table.word_three()?;
    table.key = seed.wrapping_mul(9).wrapping_add(3);
    let mut entries = Index::default();
    loop {
        let offset = table.word_three()?;
        if offset == 0 {
            break;
        }
        let size = table.word_three()?;
        let key = table.word_three()?;
        let length = table.word_three()?;
        let name = table.name_three(length)?;
        if u64::from(offset) + u64::from(size) > table.length {
            return Err(Fault::Damaged(
                "an entry of version 3 lies outside the file",
            ));
        }
        let entry = Entry {
            offset: u64::from(offset),
            size,
            key,
        };
        let Ok(name) = String::from_utf8(name) else {
            continue;
        };
        entries.insert(&name, entry);
    }
    Ok(entries)
}

/// The entries of an archive by their index names. Of entries whose names are the same once
/// normalised, the first is kept.
#[derive(Debug, Default)]
struct Index {
    entries: HashMap<String, Entry>,
    /// The entries by their index names without the extension of their last component (from
    /// its last `.`), for the data files of RGSS3. Of entries with the same stem, the first
    /// in the order of the table is kept.
    stems: HashMap<String, Entry>,
}

impl Index {
    /// Adds the entry of the name the table holds, unless the name names nothing.
    fn insert(&mut self, name: &str, entry: Entry) {
        let Some(components) = components(name) else {
            return;
        };
        let name = index_name(&components);
        if let Some((stem, extension)) = name.rsplit_once('.')
            && !extension.contains('/')
            && !stem.is_empty()
            && !stem.ends_with('/')
        {
            self.stems.entry(stem.to_owned()).or_insert(entry);
        }
        self.entries.entry(name).or_insert(entry);
    }
}

/// An open archive: the index of its entries, and the file they are read from on demand.
#[derive(Debug)]
pub(crate) struct Archive {
    path: PathBuf,
    file: Mutex<File>,
    entries: HashMap<String, Entry>,
    /// See [`Index::stems`].
    stems: HashMap<String, Entry>,
}

impl Archive {
    /// Opens the archive at `path` and reads its entry table as that of `version`.
    pub(crate) fn open(path: PathBuf, version: Version) -> Result<Self, VfsError> {
        let table = File::open(&path).map_err(Fault::Io).and_then(|file| {
            let entries = match version {
                Version::One => read_table(&file)?,
                Version::Three => read_table_three(&file)?,
            };
            Ok((entries, file))
        });
        match table {
            Ok((Index { entries, stems }, file)) => Ok(Self {
                path,
                file: Mutex::new(file),
                entries,
                stems,
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

    /// Finds the entry that `components` name as a data file of RGSS3: the entry of that
    /// name, or else the first entry whose name without the extension of its last component
    /// is that name. No extension is added.
    pub(crate) fn find_data_rgss3(&self, components: &[&str]) -> Option<Entry> {
        let name = index_name(components);
        self.entries
            .get(&name)
            .or_else(|| self.stems.get(&name))
            .copied()
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
    use proptest::prelude::*;

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

    /// The form of an RGSSAD version 3 archive that the layout leaves open.
    pub(crate) struct Three<'a> {
        pub(crate) seed: u32,
        /// The bytes between the word that ends the table and the data: none, or the twelve
        /// that the editor writes.
        pub(crate) after_end: &'a [u8],
        /// Whether the data lie in the reverse of the table's order.
        pub(crate) data_reversed: bool,
    }

    impl Three<'_> {
        /// An archive with `seed`, the data right behind the ending word, in table order.
        pub(crate) fn seed(seed: u32) -> Self {
            Three {
                seed,
                after_end: &[],
                data_reversed: false,
            }
        }

        /// Builds the archive holding `entries`: name, file key and contents of each.
        pub(crate) fn build(&self, entries: &[(&str, u32, &[u8])]) -> Vec<u8> {
            let key = self.seed.wrapping_mul(9).wrapping_add(3);
            let table: usize = entries.iter().map(|(name, ..)| 16 + name.len()).sum();
            let data_start = 12 + table + 4 + self.after_end.len();
            // The offset of each entry's data, in table order.
            let mut order: Vec<usize> = (0..entries.len()).collect();
            if self.data_reversed {
                order.reverse();
            }
            let mut offsets = vec![0; entries.len()];
            let mut next = data_start;
            for &index in &order {
                offsets[index] = next;
                next += entries[index].2.len();
            }
            let mut archive = b"RGSSAD\0\x03".to_vec();
            archive.extend(self.seed.to_le_bytes());
            for ((name, file_key, contents), offset) in entries.iter().zip(&offsets) {
                let words = [*offset, contents.len(), *file_key as usize, name.len()];
                for word in words {
                    archive.extend((u32::try_from(word).unwrap() ^ key).to_le_bytes());
                }
                let mask = key.to_le_bytes();
                archive.extend(name.bytes().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
            }
            archive.extend(key.to_le_bytes());
            archive.extend(self.after_end);
            for &index in &order {
                let (_, file_key, contents) = entries[index];
                let mut data_key = file_key;
                for word in contents.chunks(4) {
                    let mask = step(&mut data_key).to_le_bytes();
                    archive.extend(word.iter().zip(mask).map(|(byte, mask)| byte ^ mask));
                }
            }
            archive
        }
    }

    /// Writes `bytes` as `Game.rgssad` into a new directory and opens it.
    fn open(bytes: &[u8]) -> (tempfile::TempDir, Result<Archive, VfsError>) {
        open_as(bytes, Version::One)
    }

    /// Writes `bytes` as the archive of `version` into a new directory and opens it.
    fn open_as(bytes: &[u8], version: Version) -> (tempfile::TempDir, Result<Archive, VfsError>) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(match version {
            Version::One => "Game.rgssad",
            Version::Three => "Game.rgss3a",
        });
        std::fs::write(&path, bytes).unwrap();
        let archive = Archive::open(path, version);
        (dir, archive)
    }

    /// Opens `bytes` as an archive of version 3 that must be damaged, with an error that
    /// names the file.
    fn damaged_three(bytes: &[u8]) {
        let (dir, archive) = open_as(bytes, Version::Three);
        let error = archive.unwrap_err();
        let path = dir.path().join("Game.rgss3a");
        assert!(
            matches!(&error, VfsError::DamagedArchive { path: named, .. } if *named == path),
            "{} bytes",
            bytes.len()
        );
        assert!(error.to_string().contains("Game.rgss3a"));
        assert!(error.to_string().contains("version 3"));
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

    #[test]
    fn huge_declared_length() {
        // One entry named `A` that declares four gigabytes of data, in a file of one kilobyte.
        let mut bytes = build(&[("A", b"")]);
        let size = bytes.len() - 4;
        for byte in &mut bytes[size..] {
            *byte ^= 0xFF;
        }
        bytes.resize(1024, 0);
        let (_dir, archive) = open(&bytes);
        assert!(matches!(
            archive.unwrap_err(),
            VfsError::DamagedArchive { .. }
        ));
    }

    /// The 38 bytes of the worked example of the specification.
    const WORKED_EXAMPLE: [u8; 38] = [
        0x52, 0x47, 0x53, 0x53, 0x41, 0x44, 0x00, 0x03, // the header
        0x01, 0x00, 0x00, 0x00, // the seed 1: the table key is 12
        0x2D, 0x00, 0x00, 0x00, // the offset 33
        0x09, 0x00, 0x00, 0x00, // the size 5
        0x25, 0x00, 0x00, 0x00, // the file key 41
        0x0D, 0x00, 0x00, 0x00, // the name length 1
        0x4D, // `A`
        0x0C, 0x00, 0x00, 0x00, // the word that ends the table
        0x48, 0x62, 0x63, 0x64, // `abcd` XOR 41
        0x47, // `e` XOR 0x22, the low byte of 41 * 7 + 3
    ];

    #[test]
    fn worked_example_of_version_three() {
        assert_eq!(Three::seed(1).build(&[("A", 41, b"abcde")]), WORKED_EXAMPLE);
        let (_dir, archive) = open_as(&WORKED_EXAMPLE, Version::Three);
        let archive = archive.unwrap();
        assert_eq!(archive.entries.len(), 1);
        assert_eq!(read(&archive, "A"), b"abcde");
    }

    #[test]
    fn keys_measured_on_the_original() {
        let entries: [(&str, u32, &[u8]); 3] = [
            ("Data\\Actors.rvdata2", 41, b"actors"),
            ("Data\\Scripts.rvdata2", 18467, b"scripts!"),
            ("Graphics\\System\\Window.png", 6334, b"a window skin"),
        ];
        for (seed, key) in [(0x2A38, 0x17BFB_u32), (0x350A, 0x1DD5D)] {
            let bytes = Three::seed(seed).build(&entries);
            // The first word of the table is the offset of the first entry's data: the
            // header and the seed, three records and the ending word lie before it.
            let names: usize = entries.iter().map(|(name, ..)| name.len()).sum();
            let offset = u32::try_from(12 + 3 * 16 + names + 4).unwrap();
            assert_eq!(bytes[12..16], (offset ^ key).to_le_bytes());
            let (_dir, archive) = open_as(&bytes, Version::Three);
            let archive = archive.unwrap();
            assert_eq!(archive.entries.len(), entries.len());
            for (name, _, contents) in entries {
                assert_eq!(read(&archive, name), contents, "{name}");
            }
        }
    }

    #[test]
    fn end_record_of_the_editor() {
        let entries: [(&str, u32, &[u8]); 2] = [
            ("Data\\Scripts.rvdata2", 41, b"scripts"),
            ("Data\\Map001.rvdata2", 18467, b"a map"),
        ];
        let twelve = [
            0xD3, 0x7C, 0x01, 0x00, 0x2A, 0x2A, 0x01, 0x00, 0xFF, 0, 7, 9,
        ];
        for after_end in [&twelve[..], &[]] {
            let form = Three {
                after_end,
                ..Three::seed(0x2A38)
            };
            let (_dir, archive) = open_as(&form.build(&entries), Version::Three);
            let archive = archive.unwrap();
            assert_eq!(archive.entries.len(), entries.len());
            for (name, _, contents) in entries {
                assert_eq!(read(&archive, name), contents, "{name}");
            }
        }
    }

    #[test]
    fn names_and_sizes_of_version_three() {
        let long: Vec<u8> = (0..=255u8).cycle().take(70_001).collect();
        let entries: [(&str, u32, &[u8]); 4] = [
            ("Data\\Empty.rvdata2", 41, b""),
            ("Audio\\SE\\One.ogg", 18467, b"x"),
            ("Graphics\\Titles\\标题.png", 6334, b"12345"),
            ("Data\\Long.rvdata2", 0xFFFF_FFF0, &long),
        ];
        let (_dir, archive) = open_as(&Three::seed(0x350A).build(&entries), Version::Three);
        let archive = archive.unwrap();
        assert_eq!(archive.entries.len(), entries.len());
        for (name, _, contents) in entries {
            assert_eq!(read(&archive, name), contents, "{name}");
        }
        let entry = archive.find(&components("graphics/titles/标题").unwrap(), &["png"]);
        assert_eq!(archive.read(entry.unwrap()).unwrap(), b"12345");
    }

    #[test]
    fn data_in_another_order_than_the_table() {
        let form = Three {
            data_reversed: true,
            ..Three::seed(7)
        };
        let bytes = form.build(&[("First", 41, b"the first"), ("Second", 18467, b"second")]);
        // The data of the second record lies right behind the table.
        let second = u32::try_from(12 + 16 + 5 + 16 + 6 + 4).unwrap();
        assert_eq!(bytes[12 + 16 + 5..][..4], (second ^ 66).to_le_bytes());
        let (_dir, archive) = open_as(&bytes, Version::Three);
        let archive = archive.unwrap();
        assert_eq!(read(&archive, "First"), b"the first");
        assert_eq!(read(&archive, "Second"), b"second");
    }

    #[test]
    fn first_of_two_names_and_names_that_are_not_text_in_version_three() {
        let mut bytes = Three::seed(1).build(&[
            ("Data\\A", 41, b"first"),
            ("data/a", 18467, b"second"),
            ("B", 6334, b"left out"),
        ]);
        // The name of the third record becomes a byte that is not UTF-8.
        let name = 12 + (16 + 6) + (16 + 6) + 16;
        bytes[name] = 0xFF ^ 12;
        let (_dir, archive) = open_as(&bytes, Version::Three);
        let archive = archive.unwrap();
        assert_eq!(archive.entries.len(), 1);
        assert_eq!(read(&archive, "Data/A"), b"first");
    }

    #[test]
    fn wrong_header_of_version_three() {
        let whole = Three::seed(1).build(&[("Data\\Scripts.rvdata2", 41, b"scripts")]);
        let mut other_version = whole.clone();
        other_version[7] = 2;
        for bytes in [
            &build(&[("Data\\Scripts.rvdata2", b"scripts")])[..],
            &other_version,
            b"PK\x03\x04 something else",
            &whole[..11],
            &whole[..8],
            b"",
        ] {
            damaged_three(bytes);
        }
    }

    #[test]
    fn table_of_version_three_without_an_end() {
        let entries: [(&str, u32, &[u8]); 2] = [
            ("Data\\Scripts.rvdata2", 41, b"scripts"),
            ("Data\\Map001.rvdata2", 18467, b"a map of some length"),
        ];
        let whole = Three::seed(0x2A38).build(&entries);
        let end = 12 + (16 + 20) + (16 + 19);
        // Inside the words of a record, inside a name, and right before the ending word.
        for length in [
            12,
            14,
            12 + 16,
            12 + 16 + 7,
            12 + 36 + 9,
            end - 1,
            end,
            end + 3,
        ] {
            damaged_three(&whole[..length]);
        }
        // A table of no record at all, cut before its ending word.
        let empty = Three::seed(0x2A38).build(&[]);
        assert_eq!(empty.len(), 16);
        damaged_three(&empty[..12]);
        let (_dir, archive) = open_as(&empty, Version::Three);
        assert!(archive.unwrap().entries.is_empty());
    }

    #[test]
    fn entry_of_version_three_outside_the_file() {
        let whole = Three::seed(1).build(&[("A", 41, b"abcde")]);
        // The last byte of the data is missing.
        damaged_three(&whole[..whole.len() - 1]);
        // The offset is one byte further than it may be.
        let mut moved = whole.clone();
        moved[12] ^= 33 ^ 34;
        damaged_three(&moved);
        // An offset and a size whose sum does not fit 32 bits.
        let mut wrapped = whole.clone();
        for byte in &mut wrapped[12..16] {
            *byte ^= 0xFF;
        }
        damaged_three(&wrapped);
    }

    #[test]
    fn huge_declared_lengths_in_version_three() {
        // One entry named `A` without data, in a file of one kilobyte; then its name or its
        // data declared as four gigabytes.
        let whole = Three::seed(1).build(&[("A", 41, b"")]);
        for word in [12 + 12, 12 + 4] {
            let mut bytes = whole.clone();
            for byte in &mut bytes[word..word + 4] {
                *byte ^= 0xFF;
            }
            bytes.resize(1024, 0);
            damaged_three(&bytes);
        }
    }

    /// Opens `bytes` as an archive and reads every entry it lists. Each step may fail; none
    /// may panic, and no entry may be longer than the file.
    fn open_and_read_all(bytes: &[u8], version: Version) -> Result<(), TestCaseError> {
        let (_dir, archive) = open_as(bytes, version);
        let Ok(archive) = archive else {
            return Ok(());
        };
        for entry in archive.entries.values() {
            prop_assert!(entry.size as usize <= bytes.len());
            if let Ok(data) = archive.read(*entry) {
                prop_assert_eq!(data.len(), entry.size as usize);
            }
        }
        Ok(())
    }

    /// A valid archive of a few entries with short names and contents.
    fn valid_archive() -> impl Strategy<Value = Vec<u8>> {
        let entry = ("[A-Za-z]{1,8}", prop::collection::vec(any::<u8>(), 0..40));
        prop::collection::vec(entry, 1..5).prop_map(|entries| {
            let entries: Vec<(&str, &[u8])> = entries
                .iter()
                .map(|(name, contents)| (name.as_str(), contents.as_slice()))
                .collect();
            build(&entries)
        })
    }

    /// A valid archive of version 3 of a few entries, in each form the builder knows.
    fn valid_archive_three() -> impl Strategy<Value = Vec<u8>> {
        let entry = (
            "[A-Za-z]{1,8}",
            any::<u32>(),
            prop::collection::vec(any::<u8>(), 0..40),
        );
        (
            any::<u32>(),
            prop::collection::vec(entry, 1..5),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(seed, entries, editor, data_reversed)| {
                let entries: Vec<(&str, u32, &[u8])> = entries
                    .iter()
                    .map(|(name, key, contents)| (name.as_str(), *key, contents.as_slice()))
                    .collect();
                let form = Three {
                    seed,
                    after_end: if editor { &[0x5A; 12] } else { &[] },
                    data_reversed,
                };
                form.build(&entries)
            })
    }

    proptest! {
        #[test]
        fn random_bytes_behind_the_header(tail in prop::collection::vec(any::<u8>(), 0..600)) {
            open_and_read_all(&[&HEADER[..], &tail].concat(), Version::One)?;
        }

        #[test]
        fn random_bytes_behind_the_header_of_version_three(
            tail in prop::collection::vec(any::<u8>(), 0..600),
        ) {
            open_and_read_all(&[&HEADER_THREE[..], &tail].concat(), Version::Three)?;
        }

        #[test]
        fn flipped_bytes_in_a_valid_archive_of_version_three(
            mut bytes in valid_archive_three(),
            flips in prop::collection::vec((any::<prop::sample::Index>(), 1..=255u8), 1..4),
        ) {
            for (at, mask) in flips {
                let at = at.index(bytes.len());
                bytes[at] ^= mask;
            }
            open_and_read_all(&bytes, Version::Three)?;
        }

        #[test]
        fn valid_archive_of_version_three_cut_short(
            bytes in valid_archive_three(),
            cut in any::<prop::sample::Index>(),
        ) {
            open_and_read_all(&bytes[..cut.index(bytes.len() + 1)], Version::Three)?;
        }

        #[test]
        fn valid_archive_of_version_three_comes_back(bytes in valid_archive_three()) {
            let (_dir, archive) = open_as(&bytes, Version::Three);
            prop_assert!(archive.is_ok());
        }

        #[test]
        fn flipped_bytes_in_a_valid_archive(
            mut bytes in valid_archive(),
            flips in prop::collection::vec((any::<prop::sample::Index>(), 1..=255u8), 1..4),
        ) {
            for (at, mask) in flips {
                let at = at.index(bytes.len());
                bytes[at] ^= mask;
            }
            open_and_read_all(&bytes, Version::One)?;
        }

        #[test]
        fn valid_archive_cut_short(
            bytes in valid_archive(),
            cut in any::<prop::sample::Index>(),
        ) {
            open_and_read_all(&bytes[..cut.index(bytes.len() + 1)], Version::One)?;
        }
    }
}
