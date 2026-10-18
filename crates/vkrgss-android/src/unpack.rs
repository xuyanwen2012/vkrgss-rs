//! Unpacking the zip of an import into a directory.
//!
//! A small reader of the zip format over `flate2`, for the two methods that zips of games use:
//! stored (0) and deflated (8). Every entry's CRC-32 is checked. Sizes and offsets may be
//! those of Zip64. Encrypted entries and other methods are refused with a message that names
//! them.
//!
//! The rules of an import:
//!
//! - Names are UTF-8 when the entry says so (bit 11 of its flags) or when every name is valid
//!   UTF-8; otherwise all names that do not say so are read in the first legacy encoding
//!   that accepts all of them, in the order the runtime tries for `Game.ini`
//!   ([`Encoding::LEGACY_ORDER`]): GBK, Shift-JIS, Big5. Windows writes names in the code
//!   page of its language, which is one of them for the games of these runtimes.
//! - Both slashes separate the parts of a name. A name that is absolute (it starts with a
//!   slash or a drive such as `C:`) or that has a part `..` would leave the directory: the
//!   whole zip is refused before anything is written.
//! - When the top level of the zip is one directory and that directory holds what the zip is
//!   expected to hold at its top ([`Kind`]), the zip is unpacked one level down, so that a
//!   zip of a game's folder gives the game directory itself.
//! - Symbolic links are not created: such an entry is left out.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use flate2::read::DeflateDecoder;
use vkrgss_vfs::Encoding;

/// What a zip holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A game: `Game.ini` at its top.
    Game,
    /// A run-time package: the folders `Graphics` or `Audio` at its top.
    Rtp,
}

impl Kind {
    /// Whether a top-level name of the unpacked directory is what this kind has at its top,
    /// in any letter case.
    fn marks_top(self, name: &str) -> bool {
        let names: &[&str] = match self {
            Kind::Game => &["Game.ini"],
            Kind::Rtp => &["Graphics", "Audio"],
        };
        names.iter().any(|marker| marker.eq_ignore_ascii_case(name))
    }
}

/// The signatures of the records.
const LOCAL_HEADER: u32 = 0x0403_4b50;
const CENTRAL_HEADER: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
const ZIP64_END: u32 = 0x0606_4b50;
const ZIP64_LOCATOR: u32 = 0x0706_4b50;

/// The extra field of Zip64 sizes and offsets.
const ZIP64_EXTRA: u16 = 0x0001;

/// Bits of the flags of an entry.
const FLAG_ENCRYPTED: u16 = 1;
const FLAG_UTF8: u16 = 1 << 11;

/// The host system of an entry (the high byte of "version made by") whose external attributes
/// hold a Unix mode.
const HOST_UNIX: u16 = 3;
const MODE_TYPE: u32 = 0o170_000;
const MODE_SYMLINK: u32 = 0o120_000;

/// The MS-DOS attribute of a directory.
const DOS_DIRECTORY: u32 = 0x10;

/// One entry of the central directory.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    /// The name as it is stored.
    raw_name: Vec<u8>,
    flags: u16,
    method: u16,
    crc: u32,
    compressed: u64,
    size: u64,
    /// Where the local header is.
    offset: u64,
    made_by: u16,
    external: u32,
}

impl Entry {
    fn is_directory(&self) -> bool {
        self.raw_name.ends_with(b"/")
            || self.raw_name.ends_with(b"\\")
            || (self.made_by >> 8 != HOST_UNIX && self.external & DOS_DIRECTORY != 0)
    }

    fn is_symlink(&self) -> bool {
        self.made_by >> 8 == HOST_UNIX && (self.external >> 16) & MODE_TYPE == MODE_SYMLINK
    }
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("eight bytes"))
}

/// Reads `length` bytes at `offset`.
fn read_at(file: &mut File, offset: u64, length: usize) -> io::Result<Vec<u8>> {
    let mut bytes = vec![0; length];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// The entries of the central directory of the zip `file`.
fn entries(file: &mut File) -> Result<Vec<Entry>> {
    let length = file.metadata()?.len();
    // The end record is 22 bytes and may be followed by a comment of up to 65535 bytes.
    let tail_length = length.min(22 + 65_535);
    let tail = read_at(file, length - tail_length, tail_length as usize)?;
    let Some(end) = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&at| u32_at(&tail, at) == END)
    else {
        bail!("this is not a zip: it has no end of central directory");
    };
    let mut count = u64::from(u16_at(&tail, end + 10));
    let mut directory_size = u64::from(u32_at(&tail, end + 12));
    let mut directory_offset = u64::from(u32_at(&tail, end + 16));
    let end_offset = length - tail_length + end as u64;
    if (count == 0xFFFF || directory_size == 0xFFFF_FFFF || directory_offset == 0xFFFF_FFFF)
        && end_offset >= 20
    {
        let locator = read_at(file, end_offset - 20, 20)?;
        if u32_at(&locator, 0) == ZIP64_LOCATOR {
            let record = read_at(file, u64_at(&locator, 8), 56)?;
            if u32_at(&record, 0) != ZIP64_END {
                bail!("the zip's Zip64 end of central directory is damaged");
            }
            count = u64_at(&record, 32);
            directory_size = u64_at(&record, 40);
            directory_offset = u64_at(&record, 48);
        }
    }
    if directory_offset.saturating_add(directory_size) > length {
        bail!("the zip's central directory lies past its end");
    }
    let directory = read_at(file, directory_offset, directory_size as usize)?;
    let mut entries = Vec::new();
    let mut at = 0;
    for _ in 0..count {
        if at + 46 > directory.len() || u32_at(&directory, at) != CENTRAL_HEADER {
            bail!("the zip's central directory is damaged");
        }
        let name_length = usize::from(u16_at(&directory, at + 28));
        let extra_length = usize::from(u16_at(&directory, at + 30));
        let comment_length = usize::from(u16_at(&directory, at + 32));
        let name_start = at + 46;
        let extra_start = name_start + name_length;
        let next = extra_start + extra_length + comment_length;
        if next > directory.len() {
            bail!("the zip's central directory is damaged");
        }
        let mut entry = Entry {
            raw_name: directory[name_start..extra_start].to_vec(),
            made_by: u16_at(&directory, at + 4),
            flags: u16_at(&directory, at + 8),
            method: u16_at(&directory, at + 10),
            crc: u32_at(&directory, at + 16),
            compressed: u64::from(u32_at(&directory, at + 20)),
            size: u64::from(u32_at(&directory, at + 24)),
            external: u32_at(&directory, at + 38),
            offset: u64::from(u32_at(&directory, at + 42)),
        };
        zip64_values(
            &mut entry,
            &directory[extra_start..extra_start + extra_length],
        );
        entries.push(entry);
        at = next;
    }
    Ok(entries)
}

/// Takes the sizes and the offset of `entry` from its Zip64 extra field, where its fixed
/// fields hold the marker of a value that did not fit, in the order the format gives them.
fn zip64_values(entry: &mut Entry, mut extra: &[u8]) {
    while extra.len() >= 4 {
        let id = u16_at(extra, 0);
        let length = usize::from(u16_at(extra, 2)).min(extra.len() - 4);
        let mut data = &extra[4..4 + length];
        if id == ZIP64_EXTRA {
            for value in [&mut entry.size, &mut entry.compressed, &mut entry.offset] {
                if *value == 0xFFFF_FFFF && data.len() >= 8 {
                    *value = u64_at(data, 0);
                    data = &data[8..];
                }
            }
        }
        extra = &extra[4 + length..];
    }
}

/// Decodes `bytes` in a legacy encoding, or gives `None` when they are not valid in it.
fn decode_legacy(encoding: Encoding, bytes: &[u8]) -> Option<String> {
    let decoder = match encoding {
        Encoding::Utf8 => return std::str::from_utf8(bytes).ok().map(str::to_owned),
        Encoding::Gbk => encoding_rs::GBK,
        Encoding::ShiftJis => encoding_rs::SHIFT_JIS,
        Encoding::Big5 => encoding_rs::BIG5,
    };
    decoder
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(|text| text.into_owned())
}

/// The names of the entries, decoded by the rules of the module.
fn decode_names(entries: &[Entry]) -> Result<Vec<String>> {
    let legacy: Vec<&[u8]> = entries
        .iter()
        .filter(|entry| entry.flags & FLAG_UTF8 == 0)
        .map(|entry| entry.raw_name.as_slice())
        .collect();
    let encoding = [Encoding::Utf8]
        .into_iter()
        .chain(Encoding::LEGACY_ORDER)
        .find(|encoding| {
            legacy
                .iter()
                .all(|name| decode_legacy(*encoding, name).is_some())
        });
    let Some(encoding) = encoding else {
        bail!(
            "the names in the zip are neither UTF-8 nor {}",
            Encoding::LEGACY_ORDER.map(Encoding::name).join(", nor ")
        );
    };
    entries
        .iter()
        .map(|entry| {
            let used = match entry.flags & FLAG_UTF8 {
                0 => encoding,
                _ => Encoding::Utf8,
            };
            decode_legacy(used, &entry.raw_name).with_context(|| {
                format!(
                    "the name {:?} in the zip says it is UTF-8 and is not",
                    String::from_utf8_lossy(&entry.raw_name)
                )
            })
        })
        .collect()
}

/// The parts of `name` below the directory it is unpacked into, or an error when the name
/// would leave it.
fn parts(name: &str) -> Result<Vec<&str>> {
    let leaves = || anyhow::anyhow!("the zip holds a path that leaves its directory: {name}");
    let has_drive =
        name.as_bytes().get(1) == Some(&b':') && name.as_bytes()[0].is_ascii_alphabetic();
    if name.starts_with(['/', '\\']) || has_drive {
        return Err(leaves());
    }
    let mut parts = Vec::new();
    for part in name.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => return Err(leaves()),
            part => parts.push(part),
        }
    }
    Ok(parts)
}

/// How many leading parts every path loses: one when the top level is a single directory that
/// holds what `kind` has at its top, and nothing else.
fn stripped(paths: &[(Vec<&str>, bool)], kind: Kind) -> usize {
    let mut tops = paths.iter().filter_map(|(parts, _)| parts.first());
    let Some(first) = tops.next() else {
        return 0;
    };
    let one_directory = tops.all(|top| top == first)
        && paths
            .iter()
            .all(|(parts, directory)| parts.len() > 1 || (*directory && parts.len() == 1));
    let holds_marker = paths
        .iter()
        .any(|(parts, _)| parts.len() >= 2 && kind.marks_top(parts[1]));
    usize::from(one_directory && holds_marker)
}

/// Unpacks the zip `zip` into the directory `target`, which is created and must not exist,
/// by the rules of the module for a zip of `kind`. Nothing is written when the zip is refused
/// for its names; a zip that fails later leaves what was written in `target`, which the caller
/// removes.
pub fn unpack(zip: &Path, target: &Path, kind: Kind) -> Result<()> {
    let mut file = File::open(zip).with_context(|| format!("cannot open {}", zip.display()))?;
    let entries = entries(&mut file)?;
    let names = decode_names(&entries)?;
    let mut paths = Vec::new();
    for (entry, name) in entries.iter().zip(&names) {
        paths.push((parts(name)?, entry.is_directory()));
    }
    let strip = stripped(&paths, kind);
    for entry in &entries {
        if entry.flags & FLAG_ENCRYPTED != 0 {
            bail!("the zip is encrypted, which an import cannot read");
        }
        if !matches!(entry.method, 0 | 8) && !entry.is_directory() {
            bail!(
                "the zip holds an entry packed with method {}; an import reads only stored and \
                 deflated entries",
                entry.method
            );
        }
    }
    fs::create_dir(target).with_context(|| format!("cannot create {}", target.display()))?;
    for ((entry, name), (parts, directory)) in entries.iter().zip(&names).zip(&paths) {
        if entry.is_symlink() || parts.len() <= strip {
            continue;
        }
        let path: PathBuf = parts[strip..].iter().collect();
        let path = target.join(path);
        if *directory {
            fs::create_dir_all(&path)
                .with_context(|| format!("cannot create {}", path.display()))?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
        extract(&mut file, entry, &path).with_context(|| format!("{name} in the zip"))?;
    }
    Ok(())
}

/// Writes the contents of `entry` to `path` and checks them against the entry's CRC-32.
fn extract(file: &mut File, entry: &Entry, path: &Path) -> Result<()> {
    let header = read_at(file, entry.offset, 30).context("its local header lies past the end")?;
    if u32_at(&header, 0) != LOCAL_HEADER {
        bail!("its local header is damaged");
    }
    let data = entry.offset + 30 + u64::from(u16_at(&header, 26)) + u64::from(u16_at(&header, 28));
    file.seek(SeekFrom::Start(data))?;
    let stored = Read::by_ref(file).take(entry.compressed);
    let mut reader: Box<dyn Read> = match entry.method {
        0 => Box::new(stored),
        _ => Box::new(DeflateDecoder::new(stored)),
    };
    let mut out =
        File::create(path).with_context(|| format!("cannot create {}", path.display()))?;
    let mut crc = flate2::Crc::new();
    let mut buffer = vec![0; 1 << 16];
    let mut written = 0u64;
    loop {
        let read = reader.read(&mut buffer).context("it cannot be read")?;
        if read == 0 {
            break;
        }
        crc.update(&buffer[..read]);
        out.write_all(&buffer[..read])
            .with_context(|| format!("cannot write {}", path.display()))?;
        written += read as u64;
    }
    if written != entry.size || crc.sum() != entry.crc {
        bail!("it is damaged: its contents do not match its size and checksum");
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use flate2::Compression;
    use flate2::write::DeflateEncoder;

    use super::*;

    /// One entry of a zip that [`zip`] writes: its stored name, its flags and its contents,
    /// or `None` for a directory.
    pub(crate) struct Item<'a> {
        pub(crate) name: &'a [u8],
        pub(crate) flags: u16,
        pub(crate) contents: Option<&'a [u8]>,
    }

    pub(crate) fn file<'a>(name: &'a str, contents: &'a [u8]) -> Item<'a> {
        Item {
            name: name.as_bytes(),
            flags: FLAG_UTF8,
            contents: Some(contents),
        }
    }

    /// The bytes of a zip of `items`, files deflated or, every other one, stored.
    pub(crate) fn zip(items: &[Item<'_>]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let contents = item.contents.unwrap_or_default();
            let method: u16 = if index % 2 == 0 && item.contents.is_some() {
                8
            } else {
                0
            };
            let packed = match method {
                8 => {
                    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(contents).unwrap();
                    encoder.finish().unwrap()
                }
                _ => contents.to_vec(),
            };
            let mut crc = flate2::Crc::new();
            crc.update(contents);
            let offset = out.len() as u32;
            let fields = |out: &mut Vec<u8>| {
                out.extend_from_slice(&item.flags.to_le_bytes());
                out.extend_from_slice(&method.to_le_bytes());
                out.extend_from_slice(&[0; 4]);
                out.extend_from_slice(&crc.sum().to_le_bytes());
                out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
                out.extend_from_slice(&(contents.len() as u32).to_le_bytes());
                out.extend_from_slice(&(item.name.len() as u16).to_le_bytes());
                out.extend_from_slice(&0u16.to_le_bytes());
            };
            out.extend_from_slice(&LOCAL_HEADER.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            fields(&mut out);
            out.extend_from_slice(item.name);
            out.extend_from_slice(&packed);

            directory.extend_from_slice(&CENTRAL_HEADER.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            fields(&mut directory);
            // No comment, disk 0, no internal attributes.
            directory.extend_from_slice(&[0; 6]);
            let external: u32 = if item.contents.is_none() { 0x10 } else { 0 };
            directory.extend_from_slice(&external.to_le_bytes());
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(item.name);
        }
        let directory_offset = out.len() as u32;
        out.extend_from_slice(&directory);
        out.extend_from_slice(&END.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(items.len() as u16).to_le_bytes());
        out.extend_from_slice(&(items.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&directory_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    /// Writes a zip of `items` into `dir` and unpacks it into `dir/out`.
    fn unpacked(dir: &Path, items: &[Item<'_>], kind: Kind) -> Result<PathBuf> {
        let path = dir.join("in.zip");
        fs::write(&path, zip(items)).unwrap();
        let target = dir.join("out");
        unpack(&path, &target, kind).map(|()| target)
    }

    /// Every file below `dir`, as relative paths with forward slashes, and its contents.
    fn listing(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut files = Vec::new();
        let mut pending = vec![dir.to_owned()];
        while let Some(next) = pending.pop() {
            for entry in fs::read_dir(next).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let name = path.strip_prefix(dir).unwrap().to_string_lossy();
                    files.push((name.replace('\\', "/"), fs::read(&path).unwrap()));
                }
            }
        }
        files.sort();
        files
    }

    /// Scenario "Import a zip", for the unpacking: the one directory at the top is left out.
    #[test]
    fn a_game_in_one_directory_is_unpacked_one_level_down() {
        let dir = tempfile::tempdir().unwrap();
        let ini = b"[Game]\r\nTitle=Demo\r\n".as_slice();
        let target = unpacked(
            dir.path(),
            &[
                Item {
                    name: b"Demo/",
                    flags: 0,
                    contents: None,
                },
                file("Demo/Game.ini", ini),
                file("Demo/Data\\Map001.rxdata", b"map"),
                file("Demo/Graphics/Pictures/a.png", &[7; 3000]),
            ],
            Kind::Game,
        )
        .unwrap();
        assert_eq!(
            listing(&target),
            [
                ("Data/Map001.rxdata".to_owned(), b"map".to_vec()),
                ("Game.ini".to_owned(), ini.to_vec()),
                ("Graphics/Pictures/a.png".to_owned(), vec![7; 3000]),
            ]
        );
    }

    #[test]
    fn a_zip_with_the_game_at_its_top_or_more_at_the_top_keeps_its_paths() {
        let dir = tempfile::tempdir().unwrap();
        let target = unpacked(
            dir.path(),
            &[file("Game.ini", b"[Game]"), file("Data/x", b"x")],
            Kind::Game,
        )
        .unwrap();
        assert_eq!(listing(&target).len(), 2);
        assert!(target.join("Game.ini").is_file());

        // Two directories at the top, or one without `Game.ini`: nothing is left out.
        for (index, items) in [
            vec![file("A/Game.ini", b""), file("B/x", b"")],
            vec![file("A/x", b""), file("A/Data/Game.ini", b"")],
            vec![file("A/Game.ini", b""), file("readme.txt", b"")],
        ]
        .iter()
        .enumerate()
        {
            let dir = dir.path().join(index.to_string());
            fs::create_dir(&dir).unwrap();
            let target = unpacked(&dir, items, Kind::Game).unwrap();
            assert!(!target.join("Game.ini").exists(), "{index}");
        }
    }

    #[test]
    fn an_rtp_in_one_directory_is_unpacked_one_level_down() {
        let dir = tempfile::tempdir().unwrap();
        let target = unpacked(
            dir.path(),
            &[
                file("Standard/Graphics/Titles/t.png", b"t"),
                file("Standard/Audio/BGM/b.mid", b"b"),
            ],
            Kind::Rtp,
        )
        .unwrap();
        assert!(target.join("Graphics/Titles/t.png").is_file());
        assert!(target.join("Audio/BGM/b.mid").is_file());
        // A zip of the RTP's folders themselves.
        let dir = dir.path().join("top");
        fs::create_dir(&dir).unwrap();
        let target = unpacked(&dir, &[file("Graphics/Titles/t.png", b"t")], Kind::Rtp).unwrap();
        assert!(target.join("Graphics/Titles/t.png").is_file());
    }

    /// Scenario "A path that leaves the directory".
    #[test]
    fn a_path_that_leaves_the_directory_refuses_the_zip_and_writes_nothing() {
        for name in [
            "../escape.txt",
            "Game/../../escape.txt",
            "..\\escape.txt",
            "/escape.txt",
            "\\escape.txt",
            "C:/escape.txt",
            "c:escape.txt",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let error = unpacked(
                dir.path(),
                &[file("Game.ini", b"[Game]"), file(name, b"out")],
                Kind::Game,
            )
            .unwrap_err();
            assert!(
                error.to_string().contains("leaves its directory"),
                "{name}: {error}"
            );
            assert!(!dir.path().join("out").exists(), "{name}");
            assert!(!dir.path().join("escape.txt").exists(), "{name}");
            assert!(!dir.path().parent().unwrap().join("escape.txt").exists());
        }
        // A part that merely starts with dots stays inside.
        let dir = tempfile::tempdir().unwrap();
        let target = unpacked(dir.path(), &[file("..x/.y", b"in")], Kind::Game).unwrap();
        assert!(target.join("..x/.y").is_file());
    }

    #[test]
    fn names_are_utf8_or_else_the_first_legacy_encoding_that_reads_them_all() {
        let (gbk, _, _) = encoding_rs::GBK.encode("群战示范工程/Game.ini");
        let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode("ゲーム/Data/ｶ.rxdata");
        let cases: [(&[u8], u16, &str); 4] = [
            ("日本語.txt".as_bytes(), 0, "日本語.txt"),
            ("日本語.txt".as_bytes(), FLAG_UTF8, "日本語.txt"),
            (&gbk, 0, "群战示范工程/Game.ini"),
            (&sjis, 0, "ゲーム/Data/ｶ.rxdata"),
        ];
        for (raw, flags, expected) in cases {
            let entry = Entry {
                raw_name: raw.to_vec(),
                flags,
                method: 0,
                crc: 0,
                compressed: 0,
                size: 0,
                offset: 0,
                made_by: 0,
                external: 0,
            };
            assert_eq!(decode_names(&[entry]).unwrap(), [expected]);
        }
        // A half-width katakana before a full stop is not valid GBK, so GBK does not read
        // the name; a name flagged as UTF-8 keeps its own encoding.
        let dir = tempfile::tempdir().unwrap();
        let target = unpacked(
            dir.path(),
            &[
                Item {
                    name: &sjis,
                    flags: 0,
                    contents: Some(b"kana"),
                },
                file("ゲーム/Game.ini", b"[Game]"),
            ],
            Kind::Game,
        )
        .unwrap();
        assert_eq!(fs::read(target.join("Data/ｶ.rxdata")).unwrap(), b"kana");
        assert!(target.join("Game.ini").is_file());
    }

    #[test]
    fn damaged_encrypted_and_unknown_entries_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("in.zip");
        fs::write(&path, b"not a zip at all").unwrap();
        let error = unpack(&path, &dir.path().join("a"), Kind::Game).unwrap_err();
        assert!(error.to_string().contains("not a zip"), "{error}");

        // A changed byte of the contents fails the checksum.
        let mut bytes = zip(&[file("x", b"stored contents"), file("y", b"plain")]);
        let at = bytes.windows(5).position(|w| w == b"plain").unwrap();
        bytes[at] = b'P';
        fs::write(&path, &bytes).unwrap();
        let error = unpack(&path, &dir.path().join("b"), Kind::Game).unwrap_err();
        assert!(format!("{error:#}").contains("damaged"), "{error:#}");

        let encrypted = zip(&[Item {
            name: b"x",
            flags: FLAG_ENCRYPTED,
            contents: Some(b"x"),
        }]);
        fs::write(&path, encrypted).unwrap();
        let error = unpack(&path, &dir.path().join("c"), Kind::Game).unwrap_err();
        assert!(error.to_string().contains("encrypted"), "{error}");
        assert!(!dir.path().join("c").exists());
    }
}
