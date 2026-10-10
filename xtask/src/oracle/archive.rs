//! The script archive of a game (`Scripts.rxdata`): a Ruby `Marshal` dump of an array of
//! `[Integer, String, String]`, the third element being the zlib-deflated source.
//!
//! Only the shapes such an archive uses are read and written: arrays, integers and strings.
//! The identifier of a script, a Fixnum or a Bignum, is kept as its `Marshal` bytes.
//! A string wrapped with instance variables, as a newer Ruby writes it, is read and its
//! variables are dropped.

use std::io::Write;

use anyhow::{Context, Result, bail, ensure};
use flate2::Compression;
use flate2::write::ZlibEncoder;

/// One script of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    /// The identifier as stored: the `Marshal` bytes of a Fixnum or a Bignum, with the type.
    pub id: Vec<u8>,
    /// The name as stored, in the encoding of the editor that wrote it.
    pub name: Vec<u8>,
    /// The source as stored: zlib-deflated.
    pub deflated: Vec<u8>,
}

impl Entry {
    /// An entry holding `source`.
    pub(super) fn new(id: i32, name: &str, source: &str) -> Entry {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(source.as_bytes())
            .expect("writing to memory does not fail");
        let mut stored = vec![b'i'];
        write_long(&mut stored, id);
        Entry {
            id: stored,
            name: name.as_bytes().to_vec(),
            deflated: encoder.finish().expect("writing to memory does not fail"),
        }
    }
}

/// Reads an archive.
pub(super) fn read(bytes: &[u8]) -> Result<Vec<Entry>> {
    let mut reader = Reader { bytes, at: 0 };
    ensure!(
        reader.take(2)? == [4, 8],
        "not a Marshal dump of format 4.8"
    );
    reader.expect(b'[', "an array of scripts")?;
    let count = reader.long()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        reader.expect(b'[', "an array for a script")?;
        ensure!(reader.long()? == 3, "a script is not an array of three");
        let id = reader.integer()?;
        let name = reader.string()?;
        let deflated = reader.string()?;
        entries.push(Entry { id, name, deflated });
    }
    ensure!(
        reader.at == bytes.len(),
        "data follows the array of scripts"
    );
    Ok(entries)
}

/// Writes an archive, as Ruby 1.8 does.
pub(super) fn write(entries: &[Entry]) -> Vec<u8> {
    let mut bytes = vec![4, 8, b'['];
    let count = i32::try_from(entries.len()).expect("an archive has few scripts");
    write_long(&mut bytes, count);
    for entry in entries {
        bytes.extend([b'[', 3 + 5]);
        bytes.extend_from_slice(&entry.id);
        for string in [&entry.name, &entry.deflated] {
            bytes.push(b'"');
            let length = i32::try_from(string.len()).expect("a script is shorter than 2 GiB");
            write_long(&mut bytes, length);
            bytes.extend_from_slice(string);
        }
    }
    bytes
}

/// The archive `bytes` with one more script in front of the others, which are kept as they
/// are stored.
pub(super) fn with_first(bytes: &[u8], name: &str, source: &str) -> Result<Vec<u8>> {
    let mut entries = read(bytes)?;
    entries.insert(0, Entry::new(0, name, source));
    Ok(write(&entries))
}

/// `w_long` of Ruby's `marshal.c`.
fn write_long(bytes: &mut Vec<u8>, value: i32) {
    match value {
        0 => bytes.push(0),
        1..=122 => bytes.push(value as u8 + 5),
        -123..=-1 => bytes.push((value - 5) as u8),
        _ => {
            let le = value.to_le_bytes();
            // The fewest bytes from which the sign extension gives the value back.
            let fill = if value < 0 { 0xff } else { 0 };
            let length = (1..=4).rfind(|&n| le[n - 1] != fill).unwrap_or(1);
            bytes.push(if value < 0 {
                (-(length as i8)) as u8
            } else {
                length as u8
            });
            bytes.extend_from_slice(&le[..length]);
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let bytes: &'a [u8] = self.bytes;
        let end = self.at.checked_add(count).filter(|end| *end <= bytes.len());
        let end = end.context("the archive ends early")?;
        let taken = &bytes[self.at..end];
        self.at = end;
        Ok(taken)
    }

    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn expect(&mut self, tag: u8, what: &str) -> Result<()> {
        let at = self.at;
        let found = self.byte()?;
        ensure!(
            found == tag,
            "byte {at} of the archive: expected {what}, found the type `{}`",
            found.escape_ascii()
        );
        Ok(())
    }

    /// `r_long` of Ruby's `marshal.c`.
    fn long(&mut self) -> Result<i32> {
        let first = self.byte()? as i8;
        Ok(match first {
            0 => 0,
            1..=4 => {
                let mut le = [0; 4];
                let length = first as usize;
                le[..length].copy_from_slice(self.take(length)?);
                i32::from_le_bytes(le)
            }
            -4..=-1 => {
                let mut le = [0xff; 4];
                let length = first.unsigned_abs() as usize;
                le[..length].copy_from_slice(self.take(length)?);
                i32::from_le_bytes(le)
            }
            5.. => i32::from(first) - 5,
            _ => i32::from(first) + 5,
        })
    }

    fn length(&mut self) -> Result<usize> {
        usize::try_from(self.long()?).context("a negative length")
    }

    /// The stored bytes of an integer: a Fixnum (`i` and a long) or a Bignum (`l`, a sign
    /// byte, a length in 16-bit words and the words).
    fn integer(&mut self) -> Result<Vec<u8>> {
        let bytes: &'a [u8] = self.bytes;
        let at = self.at;
        match self.byte()? {
            b'i' => {
                self.long()?;
            }
            b'l' => {
                self.byte()?;
                let words = self.length()?;
                self.take(words.checked_mul(2).context("the archive ends early")?)?;
            }
            other => bail!(
                "byte {at} of the archive: expected the number of a script, found the type `{}`",
                other.escape_ascii()
            ),
        }
        Ok(bytes[at..self.at].to_vec())
    }

    /// A string, with or without instance variables around it.
    fn string(&mut self) -> Result<Vec<u8>> {
        let with_variables = self.bytes.get(self.at) == Some(&b'I');
        if with_variables {
            self.at += 1;
        }
        self.expect(b'"', "a string")?;
        let length = self.length()?;
        let string = self.take(length)?.to_vec();
        if with_variables {
            for _ in 0..self.length()? {
                self.variable_part()?;
                self.variable_part()?;
            }
        }
        Ok(string)
    }

    /// The name or the value of an instance variable of a string, which is its encoding:
    /// a symbol, a link to one, a boolean, `nil` or a plain string.
    fn variable_part(&mut self) -> Result<()> {
        let at = self.at;
        match self.byte()? {
            b'T' | b'F' | b'0' => {}
            b';' => {
                self.long()?;
            }
            b':' | b'"' => {
                let length = self.length()?;
                self.take(length)?;
            }
            other => bail!(
                "byte {at} of the archive: the type `{}` in the instance variables of a string",
                other.escape_ascii()
            ),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use flate2::read::ZlibDecoder;

    use super::*;

    fn source(entry: &Entry) -> Vec<u8> {
        let mut source = Vec::new();
        ZlibDecoder::new(&entry.deflated[..])
            .read_to_end(&mut source)
            .expect("zlib data");
        source
    }

    #[test]
    fn longs_are_written_as_ruby_writes_them() {
        let cases: [(i32, &[u8]); 10] = [
            (0, &[0]),
            (1, &[6]),
            (122, &[127]),
            (123, &[1, 123]),
            (256, &[2, 0, 1]),
            (0x0484_eb58, &[4, 0x58, 0xeb, 0x84, 0x04]),
            (-1, &[0xfa]),
            (-123, &[0x80]),
            (-124, &[0xff, 0x84]),
            (-257, &[0xfe, 0xff, 0xfe]),
        ];
        for (value, expected) in cases {
            let mut bytes = Vec::new();
            write_long(&mut bytes, value);
            assert_eq!(bytes, expected, "{value}");
            let mut reader = Reader {
                bytes: &bytes,
                at: 0,
            };
            assert_eq!(reader.long().unwrap(), value);
            assert_eq!(reader.at, bytes.len());
        }
    }

    /// The archive round-trip scenario of `oracle-harness`.
    #[test]
    fn a_probe_is_added_in_front_and_the_others_stay_byte_for_byte() {
        let three = [
            Entry::new(0x0484_eb58, "Game_Temp", "class Game_Temp\nend\n"),
            Entry::new(7, "\u{7fa4}\u{6218}", "$x = 1"),
            Entry {
                id: vec![b'i', 2, 0x2c, 0x01],
                name: b"Main".to_vec(),
                // Stored bytes are kept even when they are not what this writer would make.
                deflated: vec![0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
            },
        ];
        let archive = write(&three);
        assert_eq!(read(&archive).unwrap(), three);

        let probed = with_first(&archive, "probe", "srand(1)").unwrap();
        let four = read(&probed).unwrap();
        assert_eq!(four.len(), 4);
        assert_eq!(four[0].name, b"probe");
        assert_eq!(source(&four[0]), b"srand(1)");
        assert_eq!(four[1..], three);
    }

    /// The scenario "Bignum identifier" of `oracle-harness`.
    #[test]
    fn a_bignum_identifier_is_kept_as_it_is_stored() {
        // Marshal.dump(3000000000) of Ruby 1.8 is "l+" with two 16-bit words.
        let large = vec![b'l', b'+', 2 + 5, 0x00, 0x5e, 0xd0, 0xb2];
        let mut second = Entry::new(0, "Large", "$y = 2");
        second.id = large.clone();
        let mut negative = Entry::new(0, "Negative", "$z = 3");
        negative.id = vec![b'l', b'-', 3 + 5, 1, 2, 3, 4, 5, 6];
        let three = [Entry::new(7, "Small", "$x = 1"), second, negative];
        let archive = write(&three);
        let entries = read(&archive).unwrap();
        assert_eq!(entries, three);
        assert_eq!(entries[1].id, large);

        let probed = with_first(&archive, "probe", "srand(1)").unwrap();
        let first = write(&read(&probed).unwrap()[..1]);
        // After the version, the array header and the probe, the bytes are those of the archive.
        assert_eq!(probed[first.len()..], archive[4..]);
        assert_eq!(read(&probed).unwrap()[1..], three);
    }

    #[test]
    fn strings_with_instance_variables_are_read() {
        // Marshal.dump([[1, "a", "b"]]) of Ruby 3, where "a" is UTF-8 and "b" is binary
        // and a third form names its encoding.
        let mut bytes = vec![4, 8, b'[', 6, b'[', 8, b'i', 6];
        bytes.extend(b"I\"\x06a\x06:\x06ET");
        bytes.extend(b"I\"\x06b\x06:\x0dencoding\"\x0eShift_JIS");
        let entries = read(&bytes).unwrap();
        assert_eq!(entries[0].name, b"a");
        assert_eq!(entries[0].deflated, b"b");

        let mut linked = vec![4, 8, b'[', 6, b'[', 8, b'i', 6];
        linked.extend(b"I\"\x06a\x06:\x06ETI\"\x06b\x06;\x00F");
        assert_eq!(read(&linked).unwrap(), entries);
    }

    #[test]
    fn what_is_not_an_archive_is_an_error() {
        assert!(read(b"").is_err());
        assert!(read(&[4, 8, b'{', 0]).is_err());
        assert!(read(&[4, 8, b'[', 6, b'[', 8, b'i', 6, b'"', 20, b'a']).is_err());
        assert!(read(&[4, 8, b'[', 6, b'[', 8, b'f', 6, b'1', b'"', 0, b'"', 0]).is_err());
        assert!(read(&[4, 8, b'[', 6, b'[', 8, b'l', b'+', 7, 0, 0x5e]).is_err());
    }
}
