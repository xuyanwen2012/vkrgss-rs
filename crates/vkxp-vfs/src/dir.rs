//! Directory listings matched without regard to letter case.

use std::collections::HashMap;
use std::io;
use std::path::Path;

/// The entries of one directory, indexed by lower-cased name.
#[derive(Debug, Default)]
pub(crate) struct Listing {
    /// Lower-cased name to the real names that fold to it, sorted.
    by_lower: HashMap<String, Vec<String>>,
}

impl Listing {
    /// Lists `dir`. Entries whose names are not valid Unicode are left out: no script path can
    /// name them.
    pub(crate) fn read(dir: &Path) -> io::Result<Self> {
        let mut by_lower: HashMap<String, Vec<String>> = HashMap::new();
        for entry in std::fs::read_dir(dir)? {
            if let Ok(name) = entry?.file_name().into_string() {
                by_lower.entry(name.to_lowercase()).or_default().push(name);
            }
        }
        for names in by_lower.values_mut() {
            names.sort_unstable();
        }
        Ok(Self { by_lower })
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
