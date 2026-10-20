//! The minimum macOS of a Mach-O file, read from its load commands: `LC_BUILD_VERSION` of the
//! platform macOS, or the older `LC_VERSION_MIN_MACOSX`. A thin file of 32 or 64 bits is read
//! in little-endian order, as Apple silicon and Intel Macs write it; a fat file holds such
//! files, and its minimum is the highest of theirs. `cargo xtask app` checks every file of the
//! bundle with it.

use std::fmt;

use anyhow::{Context, Result, bail};

const MH_MAGIC: u32 = 0xfeed_face;
const MH_MAGIC_64: u32 = 0xfeed_facf;
const FAT_MAGIC: u32 = 0xcafe_babe;
const FAT_MAGIC_64: u32 = 0xcafe_babf;
const LC_VERSION_MIN_MACOSX: u32 = 0x24;
const LC_BUILD_VERSION: u32 = 0x32;
const PLATFORM_MACOS: u32 = 1;

/// A version of macOS as a load command holds it: `xxxx.yy.zz` in the nibbles of
/// `0xXXXXYYZZ`, so that the numbers order as the versions do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version(u32);

impl Version {
    /// The version written `major.minor` or `major.minor.patch`, as `Info.plist` writes it.
    pub(crate) fn parse(text: &str) -> Result<Version> {
        let parts = text
            .split('.')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .ok();
        match parts.as_deref() {
            Some(&[major, minor]) if major <= 0xffff && minor <= 0xff => {
                Ok(Version((major << 16) | (minor << 8)))
            }
            Some(&[major, minor, patch]) if major <= 0xffff && minor <= 0xff && patch <= 0xff => {
                Ok(Version((major << 16) | (minor << 8) | patch))
            }
            _ => bail!("`{text}` is not a version of macOS"),
        }
    }
}

impl fmt::Display for Version {
    /// `major.minor`, and `.patch` when it is not 0, as `otool -l` writes it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (major, minor, patch) = (self.0 >> 16, (self.0 >> 8) & 0xff, self.0 & 0xff);
        write!(f, "{major}.{minor}")?;
        if patch != 0 {
            write!(f, ".{patch}")?;
        }
        Ok(())
    }
}

/// The minimum macOS of the Mach-O file `bytes`, or `None` when it names none; an error when
/// it is no Mach-O file or is cut short.
pub(crate) fn minimum_macos(bytes: &[u8]) -> Result<Option<Version>> {
    let (entry, width) = match be32(bytes, 0)? {
        FAT_MAGIC => (20, 4),
        FAT_MAGIC_64 => (32, 8),
        _ => return thin(bytes),
    };
    let count = be32(bytes, 4)? as usize;
    let mut highest = None;
    for index in 0..count {
        // `cputype`, `cpusubtype`, then the offset and size of the slice, each 4 or 8 bytes.
        let at = 8 + index * entry + 8;
        let (offset, size) = if width == 4 {
            (be32(bytes, at)? as u64, be32(bytes, at + 4)? as u64)
        } else {
            (be64(bytes, at)?, be64(bytes, at + 8)?)
        };
        let slice = usize::try_from(offset)
            .ok()
            .zip(usize::try_from(size).ok())
            .and_then(|(offset, size)| bytes.get(offset..offset.checked_add(size)?))
            .context("a slice of the fat Mach-O file lies past its end")?;
        highest = highest.max(thin(slice)?);
    }
    Ok(highest)
}

/// The minimum macOS of the thin Mach-O file `bytes`.
fn thin(bytes: &[u8]) -> Result<Option<Version>> {
    let header = match le32(bytes, 0)? {
        MH_MAGIC => 28,
        MH_MAGIC_64 => 32,
        _ => bail!("not a Mach-O file"),
    };
    let count = le32(bytes, 16)?;
    let mut at = header;
    let mut minimum = None;
    for _ in 0..count {
        let size = le32(bytes, at + 4)? as usize;
        if size < 8 {
            bail!("a load command of the Mach-O file is {size} bytes long");
        }
        if bytes.len() < at + size {
            bail!("the Mach-O file is cut short");
        }
        match le32(bytes, at)? {
            LC_BUILD_VERSION if le32(bytes, at + 8)? == PLATFORM_MACOS => {
                minimum = Some(Version(le32(bytes, at + 12)?));
            }
            LC_VERSION_MIN_MACOSX => minimum = Some(Version(le32(bytes, at + 8)?)),
            _ => {}
        }
        at += size;
    }
    Ok(minimum)
}

/// The four bytes of `bytes` at `at` in little-endian order.
fn le32(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(field(bytes, at)?))
}

/// The four bytes of `bytes` at `at` in big-endian order, as a fat header holds them.
fn be32(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(field(bytes, at)?))
}

/// The eight bytes of `bytes` at `at` in big-endian order.
fn be64(bytes: &[u8], at: usize) -> Result<u64> {
    Ok(u64::from_be_bytes(field(bytes, at)?))
}

/// The `N` bytes of `bytes` at `at`.
fn field<const N: usize>(bytes: &[u8], at: usize) -> Result<[u8; N]> {
    at.checked_add(N)
        .and_then(|end| bytes.get(at..end))
        .and_then(|field| field.try_into().ok())
        .context("the Mach-O file is cut short")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A thin 64-bit Mach-O file of arm64 with the load commands `commands`, each its `cmd`
    /// and the bytes after `cmdsize`.
    pub(crate) fn thin_file(commands: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let size: usize = commands.iter().map(|(_, body)| 8 + body.len()).sum();
        // magic, cputype (arm64), cpusubtype, filetype (dylib), ncmds, sizeofcmds, flags,
        // reserved.
        for word in [
            MH_MAGIC_64,
            0x0100_000c,
            0,
            6,
            commands.len() as u32,
            size as u32,
            0,
            0,
        ] {
            bytes.extend(word.to_le_bytes());
        }
        for (cmd, body) in commands {
            bytes.extend(cmd.to_le_bytes());
            bytes.extend((8 + body.len() as u32).to_le_bytes());
            bytes.extend(body);
        }
        bytes
    }

    /// The body of `LC_BUILD_VERSION` of `platform` with the minimum `minos`.
    pub(crate) fn build_version(platform: u32, minos: &str) -> (u32, Vec<u8>) {
        let mut body = Vec::new();
        // platform, minos, sdk, ntools.
        for word in [platform, Version::parse(minos).unwrap().0, 27 << 16, 0] {
            body.extend(word.to_le_bytes());
        }
        (LC_BUILD_VERSION, body)
    }

    /// A thin file for macOS `minos`.
    pub(crate) fn dylib_for(minos: &str) -> Vec<u8> {
        thin_file(&[(0x19, vec![0; 64]), build_version(PLATFORM_MACOS, minos)])
    }

    /// A fat file of 32 or 64 bits that holds `slices`.
    fn fat_file(wide: bool, slices: &[Vec<u8>]) -> Vec<u8> {
        let entry = if wide { 32 } else { 20 };
        let mut header = Vec::new();
        header.extend((if wide { FAT_MAGIC_64 } else { FAT_MAGIC }).to_be_bytes());
        header.extend((slices.len() as u32).to_be_bytes());
        let mut offset = 8 + slices.len() * entry;
        let mut body: Vec<u8> = Vec::new();
        for slice in slices {
            header.extend(0x0100_000c_u32.to_be_bytes());
            header.extend(0_u32.to_be_bytes());
            if wide {
                header.extend((offset as u64).to_be_bytes());
                header.extend((slice.len() as u64).to_be_bytes());
                header.extend([0; 8]);
            } else {
                header.extend((offset as u32).to_be_bytes());
                header.extend((slice.len() as u32).to_be_bytes());
                header.extend([0; 4]);
            }
            offset += slice.len();
            body.extend(slice);
        }
        header.extend(body);
        header
    }

    #[test]
    fn versions_read_and_write_as_otool_shows_them() {
        assert_eq!(Version::parse("14.0").unwrap(), Version(0x000e_0000));
        assert_eq!(Version::parse("10.15.7").unwrap(), Version(0x000a_0f07));
        assert_eq!(Version(0x000e_0000).to_string(), "14.0");
        assert_eq!(Version(0x000a_0f07).to_string(), "10.15.7");
        assert!(Version::parse("27.0").unwrap() > Version::parse("14.0").unwrap());
        assert!(Version::parse("11.0").unwrap() < Version::parse("11.0.1").unwrap());
        for wrong in ["14", "14.x", "14.256", "", "1.2.3.4"] {
            assert!(Version::parse(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn the_minimum_is_that_of_the_build_version_of_macos() {
        let file = dylib_for("27.0");
        assert_eq!(minimum_macos(&file).unwrap().unwrap().to_string(), "27.0");
        // Another platform's build version (2 is iOS) is no minimum of macOS.
        assert_eq!(
            minimum_macos(&thin_file(&[build_version(2, "17.0")])).unwrap(),
            None
        );
        assert_eq!(minimum_macos(&thin_file(&[])).unwrap(), None);
    }

    #[test]
    fn an_older_file_names_its_minimum_with_version_min_macosx() {
        let mut body = Version::parse("10.13").unwrap().0.to_le_bytes().to_vec();
        body.extend(0_u32.to_le_bytes());
        let file = thin_file(&[(LC_VERSION_MIN_MACOSX, body)]);
        assert_eq!(minimum_macos(&file).unwrap().unwrap().to_string(), "10.13");
    }

    #[test]
    fn a_fat_file_needs_the_highest_minimum_of_its_slices() {
        for wide in [false, true] {
            let file = fat_file(wide, &[dylib_for("11.0"), dylib_for("14.2")]);
            assert_eq!(minimum_macos(&file).unwrap().unwrap().to_string(), "14.2");
        }
        let mut cut = fat_file(false, &[dylib_for("11.0")]);
        cut.truncate(cut.len() - 1);
        assert!(minimum_macos(&cut).is_err());
    }

    #[test]
    fn a_cut_file_is_an_error() {
        let file = dylib_for("14.0");
        for length in [20, 40, file.len() - 1] {
            assert!(minimum_macos(&file[..length]).is_err(), "{length}");
        }
        let mut endless = thin_file(&[(0x19, Vec::new())]);
        endless[36..40].copy_from_slice(&0_u32.to_le_bytes());
        assert!(minimum_macos(&endless).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn this_test_executable_names_the_minimum_it_was_built_for() {
        let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let minimum = minimum_macos(&bytes).unwrap().unwrap();
        assert!(minimum <= Version::parse("14.0").unwrap(), "{minimum}");
    }
}
