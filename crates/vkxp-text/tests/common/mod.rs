//! Helpers shared by the test files: engines built from the committed test fonts.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use swash::FontRef;
use vkxp_text::{FontSources, TextEngine, TextMask, TextRequest};

pub const LIBERATION: &str = "Liberation Sans";
pub const LIBERATION_FILE: &str = "LiberationSans-Regular.ttf";
pub const SYMBOLS: &str = "Noto Sans Symbols 2";
pub const SYMBOLS_FILE: &str = "NotoSansSymbols2-Regular.ttf";

/// A character Noto Sans Symbols 2 has and Liberation Sans lacks.
pub const SYMBOL: char = '\u{2BC5}';
/// A character neither test font has.
pub const NOWHERE: char = '\u{4E2D}';

/// The directory of the committed test fonts.
pub fn fonts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts")
}

/// The sources of an engine that knows the two test fonts and nothing else.
pub fn test_sources() -> FontSources {
    FontSources::directories_only([fonts_dir()])
}

/// An engine that knows the two test fonts and nothing else.
pub fn engine() -> TextEngine {
    TextEngine::new(&test_sources())
}

/// Copies one of the test fonts into `directory` and returns the copy's path.
pub fn copy_font(directory: &Path, file: &str) -> PathBuf {
    let target = directory.join(file);
    std::fs::copy(fonts_dir().join(file), &target).expect("copy test font");
    target.canonicalize().expect("canonical path of the copy")
}

pub fn request(family: &str, size: u32) -> TextRequest {
    TextRequest::new([family], size)
}

/// The family a request for `families` resolves to.
pub fn resolved_family(engine: &mut TextEngine, families: &[&str]) -> String {
    engine
        .resolve(&TextRequest::new(families.iter().copied(), 22))
        .expect("the engine has fonts")
        .family
}

/// Values read straight from a test font file, independently of the engine.
pub struct RawFont {
    data: Vec<u8>,
}

impl RawFont {
    pub fn load(file: &str) -> Self {
        Self {
            data: std::fs::read(fonts_dir().join(file)).expect("read test font"),
        }
    }

    fn font(&self) -> FontRef<'_> {
        FontRef::from_index(&self.data, 0).expect("test font parses")
    }

    pub fn has(&self, ch: char) -> bool {
        self.font().charmap().map(ch) != 0
    }

    pub fn units_per_em(&self) -> f32 {
        f32::from(self.font().metrics(&[]).units_per_em)
    }

    /// The OS/2 `usWinAscent` and `usWinDescent`.
    pub fn win_metrics(&self) -> (f32, f32) {
        let table = self
            .font()
            .table(swash::tag_from_bytes(b"OS/2"))
            .expect("OS/2 table");
        let value = |at: usize| f32::from(u16::from_be_bytes([table[at], table[at + 1]]));
        (value(74), value(76))
    }

    /// Pixels per em at which this font's line box is `size` pixels high.
    pub fn pixels_per_em(&self, size: u32) -> f32 {
        let (ascent, descent) = self.win_metrics();
        size as f32 * self.units_per_em() / (ascent + descent)
    }

    /// The advance of `ch` (of the missing-glyph box when the font lacks it) at
    /// `pixels_per_em`, rounded to whole pixels.
    pub fn advance(&self, ch: char, pixels_per_em: f32) -> u32 {
        let font = self.font();
        let glyph = font.charmap().map(ch);
        let units = font.glyph_metrics(&[]).advance_width(glyph);
        (units * pixels_per_em / self.units_per_em()).round() as u32
    }
}

/// The largest coverage value in the columns `columns` of `mask`.
pub fn max_coverage(mask: &TextMask, columns: std::ops::Range<u32>) -> u8 {
    let mut max = 0;
    for y in 0..mask.height {
        for x in columns.clone() {
            max = max.max(mask.coverage[(y * mask.width + x) as usize]);
        }
    }
    max
}
