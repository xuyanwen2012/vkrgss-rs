//! Directory listings matched without regard to letter case.

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::time::SystemTime;

/// The entries of one directory, indexed by lower-cased name.
#[derive(Debug)]
pub(crate) struct Listing {
    /// Lower-cased name to the real names that fold to it, sorted.
    by_lower: HashMap<String, Vec<String>>,
    /// The modification time of the directory, read just before its entries were.
    modified: SystemTime,
}

impl Listing {
    /// Lists `dir`. Entries whose names are not valid Unicode are left out: no script path can
    /// name them.
    pub(crate) fn read(dir: &Path) -> io::Result<Self> {
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
        Ok(Self { by_lower, modified })
    }

    /// A listing without entries, for something that cannot be listed.
    pub(crate) fn empty() -> Self {
        Self {
            by_lower: HashMap::new(),
            modified: SystemTime::UNIX_EPOCH,
        }
    }

    /// Whether `dir`, the directory this listing was read from, still has the modification
    /// time it had then. Adding, removing and renaming an entry all change it.
    pub(crate) fn is_current(&self, dir: &Path) -> bool {
        modified(dir).is_ok_and(|now| now == self.modified)
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
