//! The line boxes of families a system does not have.

use std::collections::HashMap;

use crate::substitution::family_key;

/// The line box of a family: the ascent and descent that the requested size refers to, in
/// font units as the font file states them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineBox {
    /// Distance from the baseline to the top of the line box.
    pub ascent: u16,
    /// Distance from the baseline to the bottom of the line box, positive.
    pub descent: u16,
    /// Font units per em.
    pub units_per_em: u16,
}

impl LineBox {
    const fn new(ascent: u16, descent: u16, units_per_em: u16) -> Self {
        Self {
            ascent,
            descent,
            units_per_em,
        }
    }

    /// The height of the line box in font units.
    fn height(self) -> f32 {
        f32::from(self.ascent) + f32::from(self.descent)
    }

    /// Pixels per em at which the line box is `size` pixels high.
    pub(crate) fn pixels_per_em(self, size: u32) -> f32 {
        size as f32 * f32::from(self.units_per_em) / self.height()
    }

    /// Distance in whole pixels from the top of a line box `size` pixels high to the baseline.
    pub(crate) fn baseline(self, size: u32) -> i32 {
        (size as f32 * f32::from(self.ascent) / self.height()).round() as i32
    }
}

/// The classic CJK fonts of Windows: a 256-unit em that the line box fills exactly.
const CJK_BITMAP_ERA: LineBox = LineBox::new(220, 36, 256);

/// Built-in line boxes: the names that share a line box, then the box. The values are the
/// OS/2 `usWinAscent` and `usWinDescent` and the units per em of the Windows fonts.
const BUILT_IN: &[(&[&str], LineBox)] = &[
    // Checked against the metric-compatible Liberation Sans in a test.
    (&["Arial"], LineBox::new(1854, 434, 2048)),
    // PLACEHOLDER: from memory of the font's metric tables, to be confirmed by reading the
    // font file or by measuring the original runtime.
    (&["Times New Roman"], LineBox::new(1825, 443, 2048)),
    // PLACEHOLDER: from memory, as above.
    (&["Courier New"], LineBox::new(1705, 615, 2048)),
    // PLACEHOLDER: from memory, as above.
    (&["Tahoma"], LineBox::new(2049, 423, 2048)),
    // PLACEHOLDER: from memory, as above.
    (&["Verdana"], LineBox::new(2059, 430, 2048)),
    // PLACEHOLDER: from memory, as above.
    (
        &[
            "SimSun",
            "宋体",
            "NSimSun",
            "新宋体",
            "SimHei",
            "黑体",
            "KaiTi_GB2312",
            "楷体_GB2312",
            "FangSong_GB2312",
            "仿宋_GB2312",
        ],
        CJK_BITMAP_ERA,
    ),
    // PLACEHOLDER: from memory, as above.
    (
        &[
            "MS Gothic",
            "ＭＳ ゴシック",
            "MS PGothic",
            "ＭＳ Ｐゴシック",
            "MS UI Gothic",
            "MS Mincho",
            "ＭＳ 明朝",
            "MS PMincho",
            "ＭＳ Ｐ明朝",
        ],
        CJK_BITMAP_ERA,
    ),
];

/// Maps a family name to its line box.
#[derive(Debug, Clone)]
pub(crate) struct LineBoxTable {
    /// Family key to line box.
    entries: HashMap<String, LineBox>,
}

impl LineBoxTable {
    /// The built-in list with `extra` entries taking precedence over the built-in box of the
    /// same name, and an earlier entry over a later one. A box without height or without an
    /// em cannot scale anything and is left out.
    pub(crate) fn new(extra: &[(String, LineBox)]) -> Self {
        let mut entries = HashMap::new();
        for (name, line_box) in extra {
            if line_box.height() > 0.0 && line_box.units_per_em > 0 {
                entries.entry(family_key(name)).or_insert(*line_box);
            }
        }
        for (names, line_box) in BUILT_IN {
            for name in *names {
                entries.entry(family_key(name)).or_insert(*line_box);
            }
        }
        Self { entries }
    }

    /// The line box of the family key `key`, when it is known.
    pub(crate) fn get(&self, key: &str) -> Option<LineBox> {
        self.entries.get(key).copied()
    }
}

/// The built-in line box of `family`. `None` when the list has no entry for the name.
pub fn default_line_box(family: &str) -> Option<LineBox> {
    let key = family_key(family);
    BUILT_IN
        .iter()
        .find(|(names, _)| names.iter().any(|name| family_key(name) == key))
        .map(|(_, line_box)| *line_box)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_EM: LineBox = LineBox::new(800, 200, 1000);

    #[test]
    fn lookup_ignores_letter_case() {
        let table = LineBoxTable::new(&[("My Game Font".to_owned(), ONE_EM)]);
        for name in [
            "SimHei",
            "simhei",
            "SIMHEI",
            " SimHei ",
            "黑体",
            "ms gothic",
        ] {
            assert_eq!(default_line_box(name), Some(CJK_BITMAP_ERA), "{name}");
            assert_eq!(table.get(&family_key(name)), Some(CJK_BITMAP_ERA), "{name}");
        }
        assert_eq!(table.get(&family_key("MY GAME FONT")), Some(ONE_EM));
        assert_eq!(default_line_box("My Game Font"), None);
        assert_eq!(table.get(&family_key("Microsoft YaHei")), None);
    }

    #[test]
    fn caller_entry_takes_precedence() {
        let arial = default_line_box("Arial").unwrap();
        assert_ne!(arial, ONE_EM);
        let table = LineBoxTable::new(&[
            ("ARIAL".to_owned(), ONE_EM),
            ("Arial".to_owned(), CJK_BITMAP_ERA),
        ]);
        assert_eq!(table.get(&family_key("Arial")), Some(ONE_EM));
        // The built-in list itself is not changed, and other names keep their built-in box.
        assert_eq!(default_line_box("Arial"), Some(arial));
        assert_eq!(table.get(&family_key("SimSun")), Some(CJK_BITMAP_ERA));
    }

    #[test]
    fn box_without_height_or_em_is_left_out() {
        let table = LineBoxTable::new(&[
            ("Arial".to_owned(), LineBox::new(0, 0, 1000)),
            ("My Game Font".to_owned(), LineBox::new(800, 200, 0)),
        ]);
        assert_eq!(table.get(&family_key("Arial")), default_line_box("Arial"));
        assert_eq!(table.get(&family_key("My Game Font")), None);
    }

    #[test]
    fn scaling() {
        assert_eq!(CJK_BITMAP_ERA.pixels_per_em(22), 22.0);
        assert_eq!(CJK_BITMAP_ERA.baseline(22), 19);
        assert_eq!(ONE_EM.baseline(10), 8);
    }
}
