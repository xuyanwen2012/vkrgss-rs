//! The version of a runtime DLL, read from the bytes of the file.

use std::fmt;

/// The signature of the version record of a Windows executable (`VS_FIXEDFILEINFO`).
const SIGNATURE: u32 = 0xFEEF_04BD;
/// The structure version that follows the signature.
const STRUCTURE_VERSION: u32 = 0x0001_0000;

/// The file version of a runtime DLL: its four numbers, as in 3.0.1.1.
///
/// The name of the file does not tell the version: a game may ship the DLL 3.0.1.1 under
/// the name `RGSS300.dll`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LibraryVersion(pub [u16; 4]);

impl LibraryVersion {
    /// The version in the version record of the file whose contents are `bytes`: the two
    /// 32-bit words after the first signature that the structure version follows, all
    /// little-endian. `None` for bytes without such a record or that end inside it. The
    /// executable format is not parsed; the record is found by its signature.
    pub fn read(bytes: &[u8]) -> Option<LibraryVersion> {
        let word = |at: usize| {
            let word = bytes.get(at..at.checked_add(4)?)?;
            Some(u32::from_le_bytes(word.try_into().ok()?))
        };
        (0..bytes.len()).find_map(|at| {
            if word(at)? != SIGNATURE || word(at + 4)? != STRUCTURE_VERSION {
                return None;
            }
            let (high, low) = (word(at + 8)?, word(at + 12)?);
            Some(LibraryVersion([
                (high >> 16) as u16,
                high as u16,
                (low >> 16) as u16,
                low as u16,
            ]))
        })
    }
}

impl fmt::Display for LibraryVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [major, minor, build, revision] = self.0;
        write!(f, "{major}.{minor}.{build}.{revision}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bytes that hold `words` little-endian after `before` other bytes.
    fn file(before: usize, words: &[u32]) -> Vec<u8> {
        let mut bytes: Vec<u8> = (0..before).map(|at| (at % 251) as u8).collect();
        for word in words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn the_two_versions_of_rgss3() {
        let early = file(
            1003,
            &[SIGNATURE, STRUCTURE_VERSION, 0x0003_0000, 0x0000_0001],
        );
        assert_eq!(
            LibraryVersion::read(&early),
            Some(LibraryVersion([3, 0, 0, 1]))
        );
        let late = file(
            1003,
            &[SIGNATURE, STRUCTURE_VERSION, 0x0003_0000, 0x0001_0001],
        );
        assert_eq!(
            LibraryVersion::read(&late),
            Some(LibraryVersion([3, 0, 1, 1]))
        );
        assert_eq!(LibraryVersion([3, 0, 1, 1]).to_string(), "3.0.1.1");
    }

    #[test]
    fn the_record_may_be_followed_by_more() {
        let mut bytes = file(8, &[SIGNATURE, STRUCTURE_VERSION, 0x0002_0000, 0x0002_0001]);
        bytes.extend_from_slice(&[0xAB; 40]);
        assert_eq!(
            LibraryVersion::read(&bytes),
            Some(LibraryVersion([2, 0, 2, 1]))
        );
    }

    #[test]
    fn bytes_without_a_record_give_no_version() {
        assert_eq!(LibraryVersion::read(&[]), None);
        assert_eq!(LibraryVersion::read(&[0; 1000]), None);
        // The file ends 3 bytes after the signature.
        let mut cut = file(100, &[SIGNATURE]);
        cut.extend_from_slice(&[0, 0, 1]);
        assert_eq!(LibraryVersion::read(&cut), None);
        // The file ends inside the version.
        let mut cut = file(100, &[SIGNATURE, STRUCTURE_VERSION, 0x0003_0000]);
        cut.extend_from_slice(&[1, 0, 0]);
        assert_eq!(LibraryVersion::read(&cut), None);
    }

    #[test]
    fn a_signature_without_the_structure_version_is_passed_over() {
        let bytes = file(
            64,
            &[
                SIGNATURE,
                0x0002_0000,
                0x0009_0009,
                0x0009_0009,
                SIGNATURE,
                STRUCTURE_VERSION,
                0x0003_0000,
                0x0001_0001,
            ],
        );
        assert_eq!(
            LibraryVersion::read(&bytes),
            Some(LibraryVersion([3, 0, 1, 1]))
        );
    }
}
