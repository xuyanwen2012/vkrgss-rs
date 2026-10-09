//! The font database: every face the engine knows, grouped by family name.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use swash::FontRef;

use crate::sources::{FontSources, SystemFonts};
use crate::substitution::family_key;

/// Weights from this value up count as bold.
const BOLD_THRESHOLD: u16 = 600;
const REGULAR_WEIGHT: u16 = 400;
const BOLD_WEIGHT: u16 = 700;

/// One face of a font file.
#[derive(Debug, Clone)]
pub(crate) struct FaceRecord {
    id: fontdb::ID,
    /// The file the face is in.
    pub(crate) path: PathBuf,
    /// The face's index in the file, for collections.
    pub(crate) index: u32,
    /// Every family name the face declares, its English one first.
    pub(crate) families: Vec<String>,
    pub(crate) weight: u16,
    pub(crate) italic: bool,
    normal_width: bool,
    /// Position of the caller directory the face came from, or `u32::MAX` for a system font.
    tier: u32,
}

impl FaceRecord {
    pub(crate) fn is_bold(&self) -> bool {
        self.weight >= BOLD_THRESHOLD
    }
}

/// The faces of all sources, in a fixed order, with the family lookup.
pub(crate) struct FontIndex {
    db: fontdb::Database,
    /// Caller faces before system faces, each group ordered by path and face index, so that
    /// the order does not depend on how a directory happens to be listed.
    records: Vec<FaceRecord>,
    /// Family key to the records of that family. Holds only the faces of the family's first
    /// source: a family in a caller directory hides the system's faces of the same name.
    by_family: HashMap<String, Vec<usize>>,
    /// All records, upright regular faces first: the order of the last-resort glyph search.
    scan_order: Vec<usize>,
}

impl FontIndex {
    pub(crate) fn new(sources: &FontSources) -> Self {
        let mut db = fontdb::Database::new();
        let mut tiers: HashMap<fontdb::ID, u32> = HashMap::new();
        let mut seen: HashSet<fontdb::ID> = HashSet::new();
        let mut mark = |db: &fontdb::Database, tier: u32| {
            for face in db.faces() {
                if seen.insert(face.id) {
                    tiers.insert(face.id, tier);
                }
            }
        };
        for (position, directory) in sources.directories.iter().enumerate() {
            db.load_fonts_dir(directory);
            mark(&db, u32::try_from(position).unwrap_or(u32::MAX - 1));
        }
        match &sources.system {
            SystemFonts::None => {}
            SystemFonts::Installed => db.load_system_fonts(),
            SystemFonts::Directories(directories) => {
                for directory in directories {
                    db.load_fonts_dir(directory);
                }
            }
        }
        mark(&db, u32::MAX);

        let mut records: Vec<FaceRecord> = db
            .faces()
            .filter_map(|face| {
                let fontdb::Source::File(path) = &face.source else {
                    return None;
                };
                Some(FaceRecord {
                    id: face.id,
                    path: path.clone(),
                    index: face.index,
                    families: face.families.iter().map(|(name, _)| name.clone()).collect(),
                    weight: face.weight.0,
                    italic: face.style != fontdb::Style::Normal,
                    normal_width: face.stretch == fontdb::Stretch::Normal,
                    tier: tiers.get(&face.id).copied().unwrap_or(u32::MAX),
                })
            })
            .collect();
        records.sort_by(|a, b| (a.tier, &a.path, a.index).cmp(&(b.tier, &b.path, b.index)));

        let mut by_family: HashMap<String, Vec<usize>> = HashMap::new();
        for (position, record) in records.iter().enumerate() {
            for family in &record.families {
                let list = by_family.entry(family_key(family)).or_default();
                // Records are sorted by tier, so the first one of a family sets its tier.
                let same_source = list
                    .first()
                    .is_none_or(|&first| records[first].tier == record.tier);
                if same_source && !list.contains(&position) {
                    list.push(position);
                }
            }
        }

        let mut scan_order: Vec<usize> = (0..records.len()).collect();
        scan_order.sort_by_key(|&position| {
            let record = &records[position];
            let plain = !record.italic && record.normal_width && record.weight == REGULAR_WEIGHT;
            (!plain, position)
        });

        Self {
            db,
            records,
            by_family,
            scan_order,
        }
    }

    pub(crate) fn record(&self, position: usize) -> &FaceRecord {
        &self.records[position]
    }

    pub(crate) fn has_family(&self, key: &str) -> bool {
        self.by_family.contains_key(key)
    }

    /// The first family name of every face that is not hidden by another source, sorted and
    /// without duplicates.
    pub(crate) fn families(&self) -> Vec<String> {
        let visible: HashSet<usize> = self.by_family.values().flatten().copied().collect();
        let mut names: Vec<String> = visible
            .into_iter()
            .filter_map(|position| self.records[position].families.first().cloned())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// The face of the family `key` that suits the style best: a face with the requested
    /// slant and weight class when there is one, otherwise the closest, the regular face
    /// before any other.
    pub(crate) fn pick(&self, key: &str, bold: bool, italic: bool) -> Option<usize> {
        let target = if bold { BOLD_WEIGHT } else { REGULAR_WEIGHT };
        self.by_family
            .get(key)?
            .iter()
            .copied()
            .min_by_key(|&position| {
                let record = &self.records[position];
                (
                    record.italic != italic,
                    record.is_bold() != bold,
                    !record.normal_width,
                    record.weight.abs_diff(target),
                    position,
                )
            })
    }

    /// The first face in last-resort order that has a glyph for `ch`. Reads every font file
    /// until one matches, so callers cache the answer.
    pub(crate) fn find_char(&self, ch: char) -> Option<usize> {
        self.scan_order.iter().copied().find(|&position| {
            let record = &self.records[position];
            self.db
                .with_face_data(record.id, |data, index| {
                    FontRef::from_index(data, index as usize)
                        .is_some_and(|font| font.charmap().map(ch) != 0)
                })
                .unwrap_or(false)
        })
    }

    /// The faces in last-resort order.
    pub(crate) fn scan_order(&self) -> &[usize] {
        &self.scan_order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An index over made-up faces of one family, `(weight, italic)` each.
    fn family(faces: &[(u16, bool)]) -> FontIndex {
        let records: Vec<FaceRecord> = faces
            .iter()
            .enumerate()
            .map(|(position, &(weight, italic))| FaceRecord {
                id: fontdb::ID::dummy(),
                path: PathBuf::from(format!("{position}.ttf")),
                index: 0,
                families: vec!["Family".to_owned()],
                weight,
                italic,
                normal_width: true,
                tier: u32::MAX,
            })
            .collect();
        FontIndex {
            db: fontdb::Database::new(),
            by_family: HashMap::from([("family".to_owned(), (0..records.len()).collect())]),
            scan_order: (0..records.len()).collect(),
            records,
        }
    }

    fn picked(index: &FontIndex, bold: bool, italic: bool) -> (u16, bool) {
        let record = index.record(index.pick("family", bold, italic).unwrap());
        (record.weight, record.italic)
    }

    #[test]
    fn styled_faces_are_selected_when_present() {
        let index = family(&[(700, true), (400, true), (700, false), (400, false)]);
        assert_eq!(picked(&index, false, false), (400, false));
        assert_eq!(picked(&index, true, false), (700, false));
        assert_eq!(picked(&index, false, true), (400, true));
        assert_eq!(picked(&index, true, true), (700, true));
    }

    #[test]
    fn the_regular_face_is_selected_when_a_style_is_absent() {
        let index = family(&[(400, true), (400, false)]);
        assert_eq!(picked(&index, true, false), (400, false));
        assert_eq!(picked(&index, true, true), (400, true));

        let index = family(&[(700, false), (400, false)]);
        assert_eq!(picked(&index, false, true), (400, false));
        assert_eq!(picked(&index, true, true), (700, false));

        let index = family(&[(400, false)]);
        for (bold, italic) in [(true, false), (false, true), (true, true)] {
            assert_eq!(picked(&index, bold, italic), (400, false));
        }
    }

    #[test]
    fn the_closest_weight_is_selected_among_several() {
        let index = family(&[(200, false), (250, false), (500, false), (900, false)]);
        assert_eq!(picked(&index, false, false), (500, false));
        assert_eq!(picked(&index, true, false), (900, false));
        assert!(index.pick("other", false, false).is_none());
    }
}
