//! The encrypted archive of a game of RGSS3 (`Game.rgss3a`, RGSSAD version 3).
//!
//! After the eight bytes of the header come a seed, little endian, and a table. The table key
//! is `seed * 9 + 3` and stays the same for the whole table. A record of the table is four
//! 32-bit little-endian words XORed with the table key: the offset of the data from the start
//! of the file, their size, the key of the entry and the length of its name; then the name,
//! byte `i` XORed with byte `i % 4` of the table key, little endian. The table ends with a
//! record whose first word decrypts to 0, of which the editor writes 16 bytes. The data of an
//! entry are XORed with the word cipher of version 1, starting at the key of the entry.
//!
//! The layout is described in the measurements of 2026-10-13 kept outside the repository
//! (`ace-v0-REPORT.md`, point 2). An archive is read into its seed, its entries in table
//! order and its end record, and written with the data in table order right after the end
//! record, which is how the editor lays them out: such an archive is written back byte for
//! byte. This reader and writer are independent of the ones in `vkrgss-vfs`.

use anyhow::{Context, Result, ensure};

use crate::rgssad;

/// The name of the archive in the directory of a game of RGSS3.
pub(crate) const FILE_NAME: &str = "Game.rgss3a";

const HEADER: [u8; 8] = *b"RGSSAD\0\x03";

/// The length of the header and the seed, after which the table starts.
const TABLE_START: usize = HEADER.len() + 4;

/// One file of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The name as stored: a path with backslashes, usually UTF-8.
    pub name: Vec<u8>,
    /// The key the word cipher of the data starts at.
    pub key: u32,
    /// The data, decrypted.
    pub data: Vec<u8>,
}

/// An archive as it was read, with what is needed to write it back as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Archive {
    pub seed: u32,
    /// The entries in the order of the table.
    pub entries: Vec<Entry>,
    /// The bytes of the record that ends the table, as stored: from the record whose first
    /// word decrypts to 0 up to the first byte of data.
    pub end: Vec<u8>,
}

#[cfg(test)]
impl Archive {
    /// A new archive, whose table ends with four words that decrypt to 0.
    pub(crate) fn new(seed: u32, entries: Vec<Entry>) -> Archive {
        let end = table_key(seed).to_le_bytes().repeat(4);
        Archive { seed, entries, end }
    }
}

fn table_key(seed: u32) -> u32 {
    seed.wrapping_mul(9).wrapping_add(3)
}

/// XORs a name with the bytes of the table key, in turn.
fn crypt_name(name: &[u8], key: u32) -> Vec<u8> {
    let key = key.to_le_bytes();
    name.iter()
        .zip(key.iter().cycle())
        .map(|(byte, key)| byte ^ key)
        .collect()
}

/// Reads an archive.
pub(crate) fn read(bytes: &[u8]) -> Result<Archive> {
    ensure!(
        bytes.starts_with(&HEADER),
        "not an RGSSAD version 3 archive"
    );
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|word| u32::from_le_bytes(word.try_into().expect("four bytes")))
            .context("the table runs past the end of the archive")
    };
    let seed = word(HEADER.len())?;
    let key = table_key(seed);
    let mut at = TABLE_START;
    // Pairs of where the data of an entry lie and the entry with its name and key.
    let mut records = Vec::new();
    while word(at)? ^ key != 0 {
        let [offset, size, file_key, length] = [0, 4, 8, 12].map(|word_at| word(at + word_at));
        let (offset, size, length) = (offset? ^ key, size? ^ key, length? ^ key);
        let name = bytes
            .get(at + 16..)
            .and_then(|rest| rest.get(..length as usize))
            .context("the table runs past the end of the archive")?;
        at += 16 + name.len();
        records.push(((offset as usize, size as usize), file_key? ^ key, name));
    }
    let first_data = records
        .iter()
        .map(|((offset, _), _, _)| *offset)
        .min()
        .unwrap_or(bytes.len());
    // Data that lie inside the end record leave it its first word.
    let end = bytes
        .get(at..first_data.max(at + 4))
        .context("the data of an entry lie outside the archive")?
        .to_vec();
    let mut entries = Vec::new();
    for ((offset, size), file_key, name) in records {
        let data = offset
            .checked_add(size)
            .and_then(|end| bytes.get(offset..end))
            .with_context(|| {
                format!(
                    "the data of `{}` lie outside the archive",
                    String::from_utf8_lossy(&crypt_name(name, key))
                )
            })?;
        entries.push(Entry {
            name: crypt_name(name, key),
            key: file_key,
            data: rgssad::crypt(data, file_key).collect(),
        });
    }
    Ok(Archive { seed, entries, end })
}

/// Writes an archive: the header, the seed, the table, the end record as kept and the data
/// of the entries in the order of the table.
pub(crate) fn write(archive: &Archive) -> Vec<u8> {
    let key = table_key(archive.seed);
    let table: usize = archive
        .entries
        .iter()
        .map(|entry| 16 + entry.name.len())
        .sum();
    let mut offset = TABLE_START + table + archive.end.len();
    let mut bytes = HEADER.to_vec();
    bytes.extend(archive.seed.to_le_bytes());
    let word = |value: usize| u32::try_from(value).expect("an archive is shorter than 4 GiB");
    for entry in &archive.entries {
        let size = entry.data.len();
        for value in [word(offset), word(size), entry.key, word(entry.name.len())] {
            bytes.extend((value ^ key).to_le_bytes());
        }
        bytes.extend(crypt_name(&entry.name, key));
        offset += size;
    }
    bytes.extend_from_slice(&archive.end);
    for entry in &archive.entries {
        bytes.extend(rgssad::crypt(&entry.data, entry.key));
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three entries with the keys of the corpus, of lengths around a word.
    fn entries() -> Vec<Entry> {
        let entry = |name: &str, key, data: &[u8]| Entry {
            name: name.as_bytes().to_vec(),
            key,
            data: data.to_vec(),
        };
        vec![
            entry("Data\\Scripts.rvdata2", 41, b"\x04\x08[\x00"),
            entry("Graphics\\Titles1\\题目.png", 18467, b"12345"),
            entry("Data\\Empty.rvdata2", 6334, b""),
        ]
    }

    /// The round-trip scenario of `oracle-harness`.
    #[test]
    fn an_archive_is_read_back_as_written() {
        let archive = Archive::new(0x2a38, entries());
        let read = read(&write(&archive)).unwrap();
        assert_eq!(read.seed, 0x2a38);
        assert_eq!(read.entries, entries());
        assert_eq!(read, archive);
    }

    /// The byte-for-byte scenario of `oracle-harness`, on an archive laid out by hand.
    #[test]
    fn an_archive_with_an_end_record_of_its_own_is_written_back_byte_for_byte() {
        let seed = 0x350a_u32;
        let key = seed * 9 + 3;
        let mut bytes = b"RGSSAD\0\x03".to_vec();
        bytes.extend(seed.to_le_bytes());
        let (name, data) = (b"Data\\a", b"1234567");
        let offset = 12 + 16 + name.len() + 16;
        for value in [offset as u32, data.len() as u32, 77, name.len() as u32] {
            bytes.extend((value ^ key).to_le_bytes());
        }
        bytes.extend(
            name.iter()
                .zip(key.to_le_bytes().iter().cycle())
                .map(|(b, k)| b ^ k),
        );
        bytes.extend(key.to_le_bytes());
        bytes.extend(*b"twelve bytes");
        bytes.extend(rgssad::crypt(data, 77));

        let archive = read(&bytes).unwrap();

        assert_eq!(archive.entries[0].data, data);
        assert_eq!(archive.end[4..], *b"twelve bytes");
        assert_eq!(write(&archive), bytes);
    }

    /// The format scenario of `oracle-harness`: the values are computed here from the seed.
    #[test]
    fn the_header_and_the_first_record_are_as_the_format_says() {
        let seed = 0xdead_2a38_u32;
        let entry = entries().swap_remove(1);
        let bytes = write(&Archive::new(seed, vec![entry.clone()]));

        assert_eq!(bytes[..8], *b"RGSSAD\0\x03");
        assert_eq!(bytes[8..12], seed.to_le_bytes());
        let key = seed.wrapping_mul(9).wrapping_add(3);
        let words: Vec<u32> = bytes[12..28]
            .chunks(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()) ^ key)
            .collect();
        let name = entry.name.len() as u32;
        // The data follow the record, its name and an end record of 16 bytes.
        assert_eq!(words, [12 + 16 + name + 16, 5, 18467, name]);
        let data = &bytes[words[0] as usize..];
        assert_eq!(data.len(), 5);
        let mut plain = data.to_vec();
        for (i, byte) in plain.iter_mut().enumerate() {
            // The first word under the key of the entry, then a byte under the next key.
            let word_key: u32 = if i < 4 { 18467 } else { 18467 * 7 + 3 };
            *byte ^= word_key.to_le_bytes()[i % 4];
        }
        assert_eq!(plain, b"12345");
    }

    /// The not-an-archive-of-version-3 scenario of `oracle-harness`.
    #[test]
    fn a_file_that_is_no_complete_archive_of_version_3_is_refused() {
        let version_1 = rgssad::write(&[rgssad::Entry {
            name: b"Data\\a".to_vec(),
            data: b"12345".to_vec(),
        }]);
        assert!(read(&version_1).is_err());

        let bytes = write(&Archive::new(0x2a38, entries()));
        let table_end = 12 + entries().iter().map(|e| 16 + e.name.len()).sum::<usize>() + 4;
        for length in 0..table_end {
            assert!(read(&bytes[..length]).is_err(), "{length}");
        }
        // The data of the last entry, which is empty, end with the file.
        for length in table_end..bytes.len() {
            assert!(read(&bytes[..length]).is_err(), "{length}");
        }
        let empty = write(&Archive::new(7, Vec::new()));
        assert_eq!(read(&empty).unwrap(), Archive::new(7, Vec::new()));
    }
}
