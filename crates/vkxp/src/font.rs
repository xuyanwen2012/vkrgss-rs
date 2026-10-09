//! The text backend of a running game: the binding's interface over the font engine.
//!
//! It lives in the binary so that the binding does not depend on the font engine.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use vkxp_binding::TextBackend;
use vkxp_core::{Font, Mask, TextMetrics, TextRasterizer};
use vkxp_text::{FontSources, TextEngine, TextRequest};

/// A [`TextBackend`] that draws with a [`TextEngine`].
///
/// The binding's interface takes `&self` and the engine fills caches through `&mut self`;
/// everything runs on the interpreter thread, so a `RefCell` joins the two.
pub struct FontBackend {
    engine: RefCell<TextEngine>,
}

impl FontBackend {
    /// A backend over the fonts of `sources`.
    pub fn new(sources: &FontSources) -> FontBackend {
        FontBackend {
            engine: RefCell::new(TextEngine::new(sources)),
        }
    }

    /// The backend of a game: the fonts of `font_dirs` first, then those of the game's
    /// `Fonts` directory if it has one, then the fonts installed on the system. With
    /// `own_metrics` a font that replaces another is sized by its own line box.
    pub fn for_game(game_dir: &Path, font_dirs: &[PathBuf], own_metrics: bool) -> FontBackend {
        let mut directories = font_dirs.to_vec();
        let game_fonts = game_dir.join("Fonts");
        if game_fonts.is_dir() {
            directories.push(game_fonts);
        }
        FontBackend::new(&FontSources {
            directories,
            own_metrics,
            ..FontSources::system()
        })
    }
}

/// What the engine is asked for a core font: its names in order, its size (a size below 1 is
/// drawn as 1), and its weight and slant.
fn request(font: &Font) -> TextRequest {
    TextRequest {
        families: font.names.clone(),
        size: font.size.max(1).unsigned_abs(),
        bold: font.bold,
        italic: font.italic,
    }
}

impl TextRasterizer for FontBackend {
    fn metrics(&self, font: &Font, text: &str) -> TextMetrics {
        let layout = self.engine.borrow_mut().layout(&request(font), text);
        TextMetrics {
            height: layout.height,
            advances: layout.advances,
            left_overhang: layout.left_overhang,
            right_overhang: layout.right_overhang,
            average_width: layout.average_width,
            emboldened: layout.emboldened,
        }
    }

    fn rasterize(
        &self,
        font: &Font,
        text: &str,
        advances: &[u32],
        condense: Option<(u32, u32)>,
    ) -> Mask {
        let mut engine = self.engine.borrow_mut();
        let mask = engine.rasterize_run(&request(font), text, advances, condense);
        Mask::new(mask.width, mask.height, mask.coverage)
    }
}

impl TextBackend for FontBackend {
    fn font_exists(&self, name: &str) -> bool {
        self.engine.borrow().is_available(name)
    }
}

#[cfg(test)]
mod tests {
    use vkxp_core::{Align, Bitmaps, FontDefaults, Rect};

    use super::*;

    /// The directory of the test fonts and measured samples of `vkxp-text`.
    fn text_tests() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../vkxp-text/tests")
    }

    /// A backend whose only source is the test fonts of `vkxp-text`.
    fn backend() -> FontBackend {
        FontBackend::new(&FontSources::directories_only([text_tests().join("fonts")]))
    }

    /// A backend on the font the original runtime was measured with, which is not
    /// committed: `None`, after a note, when `local/fonts` does not hold it.
    fn measured_backend() -> Option<FontBackend> {
        let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/fonts");
        if !fonts.join("NotoSansCJKsc-Regular.otf").is_file() {
            eprintln!("skipped: local/fonts/NotoSansCJKsc-Regular.otf is absent");
            return None;
        }
        Some(FontBackend::new(&FontSources::directories_only([fonts])))
    }

    /// The measured sample lines of one kind, split into fields.
    fn samples(kind: &str) -> Vec<Vec<String>> {
        let text = std::fs::read_to_string(text_tests().join("data/text-samples.txt")).unwrap();
        let lines: Vec<Vec<String>> = text
            .lines()
            .map(|line| line.split_whitespace().map(str::to_owned).collect())
            .filter(|fields: &Vec<String>| fields.first().is_some_and(|first| first == kind))
            .collect();
        assert!(!lines.is_empty(), "no {kind} samples");
        lines
    }

    fn number(field: &str) -> i32 {
        field.parse().expect("a number")
    }

    /// A string written as the hexadecimal digits of its UTF-8 bytes.
    fn string(field: &str) -> String {
        let bytes = (0..field.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&field[at..at + 2], 16).expect("hexadecimal"))
            .collect();
        String::from_utf8(bytes).expect("UTF-8")
    }

    fn font(name: &str, size: i32) -> Font {
        let defaults = FontDefaults::new(vec![name.to_owned()]);
        Font::with(&defaults, None, Some(size))
    }

    fn natural(backend: &FontBackend, font: &Font, text: &str) -> Mask {
        let advances = backend.metrics(font, text).advances;
        backend.rasterize(font, text, &advances, None)
    }

    /// `text` drawn into a transparent bitmap of `size` with an opaque font: the alpha of
    /// every pixel, which is the coverage, in rows.
    fn drawn(
        backend: &FontBackend,
        font: &Font,
        size: (u32, u32),
        rect: Rect,
        text: &str,
        align: Align,
    ) -> Vec<Vec<u8>> {
        let mut bitmaps = Bitmaps::new();
        let defaults = FontDefaults::new(font.names.clone());
        let id = bitmaps.create(size.0, size.1, &defaults).unwrap();
        bitmaps.set_font(id, font.clone()).unwrap();
        bitmaps.draw_text(id, rect, text, align, backend).unwrap();
        let rgba = bitmaps.rgba(id).unwrap();
        rgba.chunks(size.0 as usize * 4)
            .map(|row| row.chunks(4).map(|pixel| pixel[3]).collect())
            .collect()
    }

    /// The first and the last position of `lines` that has ink.
    fn ink(lines: impl Iterator<Item = bool>) -> Option<(i32, i32)> {
        let inked: Vec<i32> = lines
            .enumerate()
            .filter(|(_, ink)| *ink)
            .map(|(position, _)| position as i32)
            .collect();
        Some((*inked.first()?, *inked.last()?))
    }

    #[test]
    fn the_natural_mask_is_the_cell_the_metrics_describe() {
        let backend = backend();
        let font = font("Liberation Sans", 22);
        let metrics = backend.metrics(&font, "Hello");
        let mask = natural(&backend, &font, "Hello");
        assert!(metrics.width() > 0);
        assert_eq!(metrics.advances.len(), 5);
        assert_eq!(metrics.height, 22);
        assert_eq!(mask.width(), metrics.width() + metrics.right_overhang);
        assert_eq!(mask.height(), metrics.height);
    }

    #[test]
    fn a_rasterised_latin_string_has_ink() {
        let backend = backend();
        let mask = natural(&backend, &font("Liberation Sans", 24), "Hello");
        let ink = (0..mask.height())
            .flat_map(|y| (0..mask.width()).map(move |x| (x, y)))
            .any(|(x, y)| mask.get(x, y) > 0);
        assert!(ink);
    }

    #[test]
    fn a_size_below_one_is_drawn_as_one() {
        let backend = backend();
        for size in [0, -5] {
            let font = font("Liberation Sans", size);
            assert_eq!(backend.metrics(&font, "Hello").height, 1);
            assert_eq!(natural(&backend, &font, "Hello").height(), 1);
        }
    }

    #[test]
    fn squeezed_text_is_set_with_the_advances_asked_for() {
        let backend = backend();
        let font = font("Liberation Sans", 40);
        let metrics = backend.metrics(&font, "HHHH");
        let narrow: Vec<u32> = metrics.advances.iter().map(|a| a * 3 / 5).collect();
        let mask = backend.rasterize(&font, "HHHH", &narrow, Some((3, 5)));
        assert_eq!(mask.width(), narrow.iter().sum::<u32>());
        assert_eq!(mask.height(), metrics.height);
    }

    #[test]
    fn a_bold_font_without_a_bold_face_is_a_pixel_wider_per_character() {
        let backend = backend();
        let regular = font("Liberation Sans", 22);
        let mut bold = regular.clone();
        bold.bold = true;
        let plain = backend.metrics(&regular, "Hello");
        let metrics = backend.metrics(&bold, "Hello");
        assert_eq!(metrics.emboldened, 1);
        assert_eq!(metrics.average_width, plain.average_width + 1);
        let wider: Vec<u32> = plain.advances.iter().map(|a| a + 1).collect();
        assert_eq!(metrics.advances, wider);
    }

    #[test]
    fn font_exists_answers_from_the_engine() {
        let backend = backend();
        assert!(backend.font_exists("Liberation Sans"));
        assert!(!backend.font_exists("NoSuchFamilyAnywhere"));
    }

    const NOTO: &str = "Noto Sans CJK SC";

    /// `V size style rect_h | text height, first and last ink row of H`: `H国g` drawn left
    /// aligned in a rectangle of that height.
    #[test]
    fn measured_vertical_placement() {
        let Some(backend) = measured_backend() else {
            return;
        };
        let mut wrong = Vec::new();
        let samples = samples("V");
        for sample in &samples {
            let (size, style, height) =
                (number(&sample[1]), number(&sample[2]), number(&sample[3]));
            let mut font = font(NOTO, size);
            font.bold = style & 1 != 0;
            font.italic = style & 2 != 0;
            let text = "H国g";
            let metrics = backend.metrics(&font, text);
            let rect = Rect::new(8, 5, 400, height);
            let rows = drawn(&backend, &font, (420, 240), rect, text, Align::Left);
            // The columns of the H: its advance from the pen.
            let start = 8 + metrics.left_overhang as usize;
            let h = start..start + metrics.advances[0] as usize;
            let (first, last) = ink(rows.iter().map(|row| row[h.clone()].iter().any(|&a| a > 0)))
                .map(|(first, last)| (first - 5, last - 5))
                .expect("ink");
            // The last row is the one above the baseline and is exact. The first depends on
            // how the top of the letter is rasterised, which is not the original's: the
            // unhinted outline may put faint ink one row higher, and the original's
            // synthesised bold grows upwards too.
            let expected = (number(&sample[6]), number(&sample[7]));
            let same = metrics.height as i32 == number(&sample[5])
                && last == expected.1
                && (first - expected.0).abs() <= 1;
            if !same {
                let line = sample.join(" ");
                wrong.push(format!("{line}: {} {first} {last}", metrics.height));
            }
        }
        let count = (wrong.len(), samples.len());
        assert!(wrong.is_empty(), "{count:?}:\n{}", wrong.join("\n"));
    }

    /// `A size style string text_size_width rect_w align | first..last ink column`: the
    /// string drawn in a rectangle as high as the text.
    #[test]
    fn measured_alignment() {
        let Some(backend) = measured_backend() else {
            return;
        };
        let mut wrong = Vec::new();
        let samples = samples("A");
        for sample in &samples {
            let mut font = font(NOTO, number(&sample[1]));
            let style = number(&sample[2]);
            font.bold = style & 1 != 0;
            font.italic = style & 2 != 0;
            let text = string(&sample[3]);
            let metrics = backend.metrics(&font, &text);
            let align = match number(&sample[6]) {
                1 => Align::Center,
                2 => Align::Right,
                _ => Align::Left,
            };
            let width = number(&sample[5]);
            let rect = Rect::new(40, 0, width, metrics.height as i32);
            let size = (width as u32 + 80, metrics.height);
            let rows = drawn(&backend, &font, size, rect, &text, align);
            let columns = (0..size.0 as usize).map(|x| rows.iter().any(|row| row[x] > 0));
            let (first, last) = ink(columns)
                .map(|(first, last)| (first - 40, last - 40))
                .expect("ink");
            // The first column is exact. The last depends on how the last glyph is
            // rasterised, which is not the original's: the unhinted outline may put faint
            // ink one column further.
            let expected = (number(&sample[8]), number(&sample[9]));
            let same = metrics.width() as i32 == number(&sample[4])
                && first == expected.0
                && (last == expected.1 || last == expected.1 + 1);
            if !same {
                let line = sample.join(" ");
                wrong.push(format!("{line}: {} {first} {last}", metrics.width()));
            }
        }
        let count = (wrong.len(), samples.len());
        assert!(wrong.is_empty(), "{count:?}:\n{}", wrong.join("\n"));
    }

    /// The backend and family of the font a sample names; `None` for the font that is not
    /// committed when it is absent.
    fn sample_font<'a>(
        name: &str,
        committed: &'a FontBackend,
        measured: Option<&'a FontBackend>,
    ) -> Option<(&'a FontBackend, &'static str)> {
        match name {
            "liberation" => Some((committed, "Liberation Sans")),
            "noto" => measured.map(|backend| (backend, NOTO)),
            other => panic!("a sample of the unknown font {other}"),
        }
    }

    /// `Q font size string natural_width | widths`: the widths the text is drawn with, the
    /// natural one and then each narrower one a squeeze step gives, overhangs included.
    #[test]
    fn measured_squeeze_levels() {
        let committed = backend();
        let measured = measured_backend();
        let mut wrong = Vec::new();
        let mut checked = 0;
        for sample in samples("Q") {
            let Some((backend, family)) = sample_font(&sample[1], &committed, measured.as_ref())
            else {
                continue;
            };
            let size = number(&sample[2]);
            let font = font(family, size);
            let text = string(&sample[3]);
            let metrics = backend.metrics(&font, &text);
            let expected: Vec<u32> = sample[6..].iter().map(|w| number(w) as u32).collect();
            let mut levels = vec![natural(backend, &font, &text).width()];
            for step in vkxp_core::SQUEEZE_STEPS {
                let (advances, condense) = metrics.squeezed(size as u32, step);
                let width = backend
                    .rasterize(&font, &text, &advances, Some(condense))
                    .width();
                if width < *levels.last().unwrap() {
                    levels.push(width);
                }
            }
            if metrics.width() as i32 != number(&sample[4]) || levels != expected {
                wrong.push(format!(
                    "{}: {} {levels:?}",
                    sample.join(" "),
                    metrics.width()
                ));
            }
            checked += 1;
        }
        assert!(checked > 0);
        assert!(
            wrong.is_empty(),
            "{} of {checked}:\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    /// `P font size string text_size_width rect_w align | first..last ink column`: a sweep of
    /// rectangle widths from 1 to past the text width, in which the text is squeezed step
    /// by step and at last cut.
    #[test]
    fn measured_squeeze_sweep() {
        let committed = backend();
        let measured = measured_backend();
        let mut wrong = Vec::new();
        let mut checked = 0;
        for sample in samples("P") {
            let Some((backend, family)) = sample_font(&sample[1], &committed, measured.as_ref())
            else {
                continue;
            };
            let font = font(family, number(&sample[2]));
            let text = string(&sample[3]);
            let metrics = backend.metrics(&font, &text);
            let align = match number(&sample[6]) {
                2 => Align::Right,
                _ => Align::Left,
            };
            let width = number(&sample[5]);
            let rect = Rect::new(20, 0, width, metrics.height as i32);
            let size = (metrics.width() + 44, metrics.height);
            let rows = drawn(backend, &font, size, rect, &text, align);
            let columns = (0..size.0 as usize).map(|x| rows.iter().any(|row| row[x] > 0));
            let found = ink(columns).map(|(first, last)| (first - 20, last - 20));
            let expected = (number(&sample[8]), number(&sample[9]));
            if metrics.width() as i32 != number(&sample[4]) || found != Some(expected) {
                wrong.push(format!(
                    "{}: {} {found:?}",
                    sample.join(" "),
                    metrics.width()
                ));
            }
            checked += 1;
        }
        assert!(checked > 0);
        assert!(
            wrong.is_empty(),
            "{} of {checked}:\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }
}
