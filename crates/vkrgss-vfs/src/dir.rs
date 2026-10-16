//! Directory listings matched without regard to letter case.

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// How long before the moment of reading a directory's modification time must lie for the
/// listing to be trusted. It covers the coarsest clock a file system may take its times from:
/// a few milliseconds on kernels without multigrain timestamps, and 1 or 2 seconds on FAT and
/// some network file systems.
const SETTLED: Duration = Duration::from_secs(2);

/// The entries of one directory, indexed by lower-cased name.
#[derive(Debug)]
pub(crate) struct Listing {
    /// Lower-cased name to the real names that fold to it, sorted.
    by_lower: HashMap<String, Vec<String>>,
    /// The modification time of the directory, read just before its entries were.
    modified: SystemTime,
    /// The time of the clock just before `modified` was read.
    read_at: SystemTime,
}

impl Listing {
    /// Lists `dir`. Entries whose names are not valid Unicode are left out: no script path can
    /// name them.
    pub(crate) fn read(dir: &Path) -> io::Result<Self> {
        let read_at = SystemTime::now();
        let modified = modified(dir)?;
        let mut by_lower: HashMap<String, Vec<String>> = HashMap::new();
        for entry in std::fs::read_dir(dir)? {
            if let Ok(name) = entry?.file_name().into_string() {
                by_lower.entry(name.to_lowercase()).or_default().push(name);
            }
        }
        for names in by_lower.values_mut() {
            names.sort_unstable();
        }
        Ok(Self {
            by_lower,
            modified,
            read_at,
        })
    }

    /// A listing without entries, for something that cannot be listed.
    pub(crate) fn empty() -> Self {
        Self {
            by_lower: HashMap::new(),
            modified: SystemTime::UNIX_EPOCH,
            read_at: SystemTime::UNIX_EPOCH,
        }
    }

    /// Whether `dir`, the directory this listing was read from, still has the modification
    /// time it had then, and that time lay at least [`SETTLED`] before the listing was read.
    /// Adding, removing and renaming an entry all change the time, unless the change falls
    /// within the same tick of the file system's clock as the time read with the listing; a
    /// listing read that close to a change, or one whose time lies after the moment of reading,
    /// is therefore never current, as git treats a "racy" entry of its index.
    pub(crate) fn is_current(&self, dir: &Path) -> bool {
        let settled = self
            .read_at
            .duration_since(self.modified)
            .is_ok_and(|age| age >= SETTLED);
        settled && modified(dir).is_ok_and(|now| now == self.modified)
    }

    /// Real names that match `name` without regard to case: the exact match first if there is
    /// one, then the others in sorted order.
    pub(crate) fn matches(&self, name: &str) -> Vec<&str> {
        let Some(names) = self.by_lower.get(&name.to_lowercase()) else {
            return Vec::new();
        };
        let exact = names.iter().filter(|real| *real == name);
        let others = names.iter().filter(|real| *real != name);
        exact.chain(others).map(String::as_str).collect()
    }
}

fn modified(dir: &Path) -> io::Result<SystemTime> {
    std::fs::metadata(dir)?.modified()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lists a new directory whose modification time is set to `ago` before now.
    fn listed(ago: Duration) -> (tempfile::TempDir, Listing) {
        let dir = tempfile::tempdir().unwrap();
        let time = SystemTime::now() - ago;
        std::fs::File::open(dir.path())
            .unwrap()
            .set_modified(time)
            .unwrap();
        let listing = Listing::read(dir.path()).unwrap();
        (dir, listing)
    }

    #[test]
    fn a_listing_of_a_settled_directory_is_current() {
        let (dir, listing) = listed(Duration::from_secs(60));
        assert!(listing.is_current(dir.path()));
    }

    #[test]
    fn a_listing_read_close_to_a_change_is_not_current() {
        let (dir, listing) = listed(Duration::ZERO);
        assert!(!listing.is_current(dir.path()));
    }

    #[test]
    fn a_listing_of_a_directory_with_a_time_in_the_future_is_not_current() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::File::open(dir.path())
            .unwrap()
            .set_modified(SystemTime::now() + Duration::from_secs(60))
            .unwrap();
        let listing = Listing::read(dir.path()).unwrap();
        assert!(!listing.is_current(dir.path()));
    }

    #[test]
    fn a_listing_of_a_changed_directory_is_not_current() {
        let (dir, listing) = listed(Duration::from_secs(60));
        std::fs::write(dir.path().join("new"), b"").unwrap();
        assert!(!listing.is_current(dir.path()));
    }
}
