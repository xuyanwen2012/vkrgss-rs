//! Run-time package names to directories.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::error::VfsError;

/// Resolves run-time package names, in order, through `table`, skipping empty names.
///
/// The original runtime looks the names up in the Windows registry; here the caller supplies
/// the table. As in the registry, a name that has no exact entry matches one that differs only
/// in ASCII letter case. A name without an entry, or whose entry is not a directory, is an
/// error.
pub fn resolve_rtps<I>(
    names: I,
    table: &BTreeMap<String, PathBuf>,
) -> Result<Vec<PathBuf>, VfsError>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut dirs = Vec::new();
    for name in names {
        let name = name.as_ref();
        if name.is_empty() {
            continue;
        }
        let dir = table
            .get(name)
            .or_else(|| {
                table
                    .iter()
                    .find(|(known, _)| known.eq_ignore_ascii_case(name))
                    .map(|(_, dir)| dir)
            })
            .ok_or_else(|| VfsError::UnknownRtp {
                name: name.to_owned(),
            })?;
        if !dir.is_dir() {
            return Err(VfsError::RtpDirectoryMissing {
                name: name.to_owned(),
                dir: dir.clone(),
            });
        }
        dirs.push(dir.clone());
    }
    Ok(dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_package() {
        let dir = tempfile::tempdir().unwrap();
        let table = BTreeMap::from([("Standard".to_owned(), dir.path().to_owned())]);
        let dirs = resolve_rtps(["Standard", "", ""], &table).unwrap();
        assert_eq!(dirs, [dir.path()]);
    }

    #[test]
    fn package_of_vx() {
        use crate::{GameIni, Runtime};

        let dir = tempfile::tempdir().unwrap();
        let table = BTreeMap::from([("RPGVX".to_owned(), dir.path().to_owned())]);
        let text = "[Game]\nRTP=RPGVX\nRTP1=Standard\nScripts=Data\\Scripts.rvdata\n";
        let ini = GameIni::parse(text.as_bytes(), None).unwrap();
        let dirs = resolve_rtps(ini.rtp_names(Runtime::Rgss2), &table).unwrap();
        assert_eq!(dirs, [dir.path()]);
        // As a game of RGSS1 it names `Standard`, which the table does not hold.
        let error = resolve_rtps(ini.rtp_names(Runtime::Rgss1), &table).unwrap_err();
        assert!(matches!(&error, VfsError::UnknownRtp { name } if name == "Standard"));
    }

    #[test]
    fn unknown_package() {
        let dir = tempfile::tempdir().unwrap();
        let table = BTreeMap::from([("Standard".to_owned(), dir.path().to_owned())]);
        let error = resolve_rtps(["Standard", "Extra"], &table).unwrap_err();
        assert!(matches!(&error, VfsError::UnknownRtp { name } if name == "Extra"));
        assert!(error.to_string().contains("Extra"));
    }

    #[test]
    fn missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("absent");
        let table = BTreeMap::from([("Standard".to_owned(), absent.clone())]);
        let error = resolve_rtps(["Standard"], &table).unwrap_err();
        assert!(
            matches!(&error, VfsError::RtpDirectoryMissing { name, dir } if name == "Standard" && *dir == absent)
        );
        assert!(error.to_string().contains("Standard"));
    }

    #[test]
    fn empty_names_are_skipped_and_order_is_kept() {
        let first = tempfile::tempdir().unwrap();
        let third = tempfile::tempdir().unwrap();
        let table = BTreeMap::from([
            ("A".to_owned(), first.path().to_owned()),
            ("C".to_owned(), third.path().to_owned()),
        ]);
        let names = ["C".to_owned(), String::new(), "a".to_owned()];
        let dirs = resolve_rtps(&names, &table).unwrap();
        assert_eq!(dirs, [third.path(), first.path()]);
        assert!(resolve_rtps(["", "", ""], &table).unwrap().is_empty());
    }
}
