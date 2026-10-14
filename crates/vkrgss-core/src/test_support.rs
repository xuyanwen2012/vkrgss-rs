//! Stand-ins for tests: of this crate, and of crates that build on it before the real
//! implementations exist. Nothing here is meant for a running game.

use crate::{Font, Mask, TextMetrics, TextRasterizer};

pub mod sprite_cases;
pub mod window_cases;

/// A deterministic stand-in for a font rasteriser.
///
/// Every character is a cell: `size / 2` pixels wide (one more when bold) and `size` pixels
/// high, at least 1 by 1, without overhangs. The average character width is the cell width,
/// and bold counts as synthesised. A space has no coverage. Any other character covers the
/// whole of its advance with values from 56 to 255 that depend on the position within the
/// text and on the character, so tests can tell where a pixel of the mask ended up; with
/// other advances than the natural ones the cells are that wide, and condensing changes
/// nothing else. Family names and the italic flag are ignored.
#[derive(Debug, Clone, Copy, Default)]
pub struct CellRasterizer;

impl CellRasterizer {
    /// Width of one character in `font`.
    pub fn cell_width(font: &Font) -> u32 {
        (font.size / 2).max(1).unsigned_abs() + u32::from(font.bold)
    }

    /// Height of a line in `font`.
    pub fn line_height(font: &Font) -> u32 {
        font.size.max(1).unsigned_abs()
    }

    /// The coverage of character `character` at (`x`, `y`) of the mask of its text.
    pub fn coverage(character: char, x: u32, y: u32) -> u8 {
        if character == ' ' {
            return 0;
        }
        let step = x + 3 * y + 11 * (u32::from(character) % 5);
        255 - (step % 200) as u8
    }

    /// The mask of `text` at its natural advances.
    pub fn natural(font: &Font, text: &str) -> Mask {
        let advances = CellRasterizer.metrics(font, text).advances;
        CellRasterizer.rasterize(font, text, &advances, None)
    }
}

impl TextRasterizer for CellRasterizer {
    fn metrics(&self, font: &Font, text: &str) -> TextMetrics {
        let cell = CellRasterizer::cell_width(font);
        TextMetrics {
            height: CellRasterizer::line_height(font),
            advances: vec![cell; text.chars().count()],
            left_overhang: 0,
            right_overhang: 0,
            average_width: cell,
            emboldened: u32::from(font.bold),
        }
    }

    fn rasterize(
        &self,
        font: &Font,
        text: &str,
        advances: &[u32],
        _condense: Option<(u32, u32)>,
    ) -> Mask {
        let height = CellRasterizer::line_height(font);
        // The character of each column.
        let columns: Vec<char> = text
            .chars()
            .zip(advances)
            .flat_map(|(character, &advance)| (0..advance).map(move |_| character))
            .collect();
        let width = columns.len() as u32;
        let mut coverage = Vec::with_capacity(columns.len() * height as usize);
        for y in 0..height {
            for (x, &character) in columns.iter().enumerate() {
                coverage.push(CellRasterizer::coverage(character, x as u32, y));
            }
        }
        Mask::new(width, height, coverage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FontDefaults;

    fn fonts() -> Vec<Font> {
        let defaults = FontDefaults::new(vec!["Test".to_owned()]);
        let mut fonts = Vec::new();
        for size in [-4, 0, 1, 2, 9, 10, 22, 48] {
            for bold in [false, true] {
                for italic in [false, true] {
                    let mut font = Font::with(&defaults, None, Some(size));
                    font.bold = bold;
                    font.italic = italic;
                    fonts.push(font);
                }
            }
        }
        fonts
    }

    #[test]
    fn the_natural_mask_has_the_size_the_metrics_report() {
        for font in fonts() {
            for text in ["", " ", "A", "Hello, world", "    ", "日本語 text", "a\tb"] {
                let metrics = CellRasterizer.metrics(&font, text);
                let mask = CellRasterizer::natural(&font, text);
                let characters = text.chars().count() as u32;
                assert_eq!(metrics.advances.len() as u32, characters);
                assert_eq!(
                    metrics.width(),
                    characters * CellRasterizer::cell_width(&font)
                );
                assert_eq!(metrics.height, CellRasterizer::line_height(&font));
                assert_eq!(mask.width(), metrics.width(), "{text:?}");
                assert_eq!(mask.height(), metrics.height, "{text:?}");
                assert_eq!(metrics.average_width, CellRasterizer::cell_width(&font));
                assert_eq!(metrics.emboldened, u32::from(font.bold));
            }
        }
    }

    #[test]
    fn other_advances_give_cells_of_that_width() {
        let defaults = FontDefaults::new(Vec::new());
        let font = Font::with(&defaults, None, Some(10));
        let mask = CellRasterizer.rasterize(&font, "A B", &[3, 2, 4], Some((3, 5)));
        assert_eq!((mask.width(), mask.height()), (9, 10));
        for x in 0..9 {
            assert_eq!(mask.get(x, 0) == 0, (3..5).contains(&x), "column {x}");
        }
    }

    #[test]
    fn the_pattern_is_deterministic_and_recognisable() {
        let defaults = FontDefaults::new(Vec::new());
        let font = Font::with(&defaults, None, Some(10));
        let mask = CellRasterizer::natural(&font, "A B");
        assert_eq!(mask, CellRasterizer::natural(&font, "A B"));
        assert_eq!((mask.width(), mask.height()), (15, 10));
        for y in 0..10 {
            for x in 0..15 {
                let value = mask.get(x, y);
                if (5..10).contains(&x) {
                    assert_eq!(value, 0, "a space has no coverage");
                } else {
                    assert!(value >= 56, "({x}, {y})");
                }
            }
            // Neighbouring columns of one character differ, so a column can be told apart.
            assert_ne!(mask.get(0, y), mask.get(1, y));
        }
        assert_ne!(
            CellRasterizer::coverage('A', 0, 0),
            CellRasterizer::coverage('B', 0, 0)
        );

        let mut bold = font.clone();
        bold.bold = true;
        assert_eq!(CellRasterizer.metrics(&bold, "A B").width(), 18);
    }
}
