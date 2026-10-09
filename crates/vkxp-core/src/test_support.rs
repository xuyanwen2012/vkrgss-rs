//! Stand-ins for tests: of this crate, and of crates that build on it before the real
//! implementations exist. Nothing here is meant for a running game.

use crate::{Font, Mask, TextRasterizer};

/// A deterministic stand-in for a font rasteriser.
///
/// Every character is a cell: `size / 2` pixels wide (one more when bold) and `size` pixels
/// high, at least 1 by 1. A space has no coverage. Any other character covers its whole cell
/// with values from 56 to 255 that depend on the position within the text and on the
/// character, so tests can tell where a pixel of the mask ended up. Family names and the
/// italic flag are ignored.
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
}

impl TextRasterizer for CellRasterizer {
    fn measure(&self, font: &Font, text: &str) -> (u32, u32) {
        let characters = text.chars().count() as u32;
        (
            characters * CellRasterizer::cell_width(font),
            CellRasterizer::line_height(font),
        )
    }

    fn rasterize(&self, font: &Font, text: &str) -> Mask {
        let (width, height) = self.measure(font, text);
        let cell = CellRasterizer::cell_width(font);
        let characters: Vec<char> = text.chars().collect();
        let mut coverage = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height {
            for x in 0..width {
                let character = characters[(x / cell) as usize];
                coverage.push(CellRasterizer::coverage(character, x, y));
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
    fn rasterize_has_the_size_measure_reports() {
        let rasterizer = CellRasterizer;
        for font in fonts() {
            for text in ["", " ", "A", "Hello, world", "    ", "日本語 text", "a\tb"] {
                let (width, height) = rasterizer.measure(&font, text);
                let mask = rasterizer.rasterize(&font, text);
                assert_eq!((mask.width(), mask.height()), (width, height), "{text:?}");
                assert_eq!(mask.coverage().len() as u32, width * height, "{text:?}");
                assert_eq!(
                    width,
                    text.chars().count() as u32 * CellRasterizer::cell_width(&font)
                );
                assert_eq!(height, CellRasterizer::line_height(&font));
            }
        }
    }

    #[test]
    fn empty_string_has_no_width_and_one_line_of_height() {
        let defaults = FontDefaults::new(Vec::new());
        let font = Font::new(&defaults);
        assert_eq!(CellRasterizer.measure(&font, ""), (0, 22));
    }

    #[test]
    fn the_pattern_is_deterministic_and_recognisable() {
        let defaults = FontDefaults::new(Vec::new());
        let font = Font::with(&defaults, None, Some(10));
        let mask = CellRasterizer.rasterize(&font, "A B");
        assert_eq!(mask, CellRasterizer.rasterize(&font, "A B"));
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
        assert_eq!(CellRasterizer.measure(&bold, "A B"), (18, 10));
    }
}
