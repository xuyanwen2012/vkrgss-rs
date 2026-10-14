//! The text backend of a running game: the binding's interface over the font engine.
//!
//! It lives outside the binding so that the binding does not depend on the font engine.
//! For a game of RGSS3 it also takes the fonts of the run-time packages, the glyph rule of
//! the DLL the game ships and the metric rule of RGSS3.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use vkrgss_binding::TextBackend;
use vkrgss_core::{Font, Mask, TextMetrics, TextRasterizer};
use vkrgss_text::{FontSources, MaskRules, MetricRules, TextEngine, TextRequest};
use vkrgss_vfs::{LibraryVersion, Runtime};

/// The directories a game of `runtime` takes fonts from before the system's, in order: those
/// of `font_dirs`, the game's `Fonts` directory, and for RGSS3 the `Fonts` directory of each
/// run-time package in the order of `rtp_dirs`, each of the last two when it exists. The
/// game comes before the packages, as in the search of the game's files.
///
/// Provenance of the fonts of RGSS3: oracle measurement 2026-10-13, `ace-v0-REPORT.md`
/// "Fonts" and `ace-v1-bitmap-REPORT.md` section 7: the original loads the fonts of
/// `<RTP>/Fonts` and `<game>/Fonts`. The packages of RGSS1 and RGSS2 are not read for fonts.
pub(crate) fn font_directories(
    runtime: Runtime,
    game_dir: &Path,
    rtp_dirs: &[PathBuf],
    font_dirs: &[PathBuf],
) -> Vec<PathBuf> {
    let mut directories = font_dirs.to_vec();
    let packages: &[PathBuf] = if runtime == Runtime::Rgss3 {
        rtp_dirs
    } else {
        &[]
    };
    let fonts = std::iter::once(game_dir)
        .chain(packages.iter().map(PathBuf::as_path))
        .map(|dir| dir.join("Fonts"))
        .filter(|fonts| fonts.is_dir());
    directories.extend(fonts);
    directories
}

/// The rule masks of glyphs are made by in a game of `runtime` whose DLL has `version`:
/// those of RGSS1 and RGSS2, and in RGSS3 those of the DLL 3.0.0.1 for a version whose
/// first three numbers are 3.0.0 and those of 3.0.1.1 for any other version and for none.
/// This is the table of `docs/decisions.md`, "Text and bitmaps of RGSS3".
pub(crate) fn mask_rules(runtime: Runtime, version: Option<LibraryVersion>) -> MaskRules {
    match (runtime, version) {
        (Runtime::Rgss3, Some(version)) if version.is_3_0_0() => MaskRules::Rgss3Early,
        (Runtime::Rgss3, _) => MaskRules::Rgss3,
        (Runtime::Rgss1 | Runtime::Rgss2, _) => MaskRules::Plain,
    }
}

/// The rule text is measured and set by in a game of `runtime`: in RGSS3 that of the glyphs
/// Wine's FreeType hints, whatever version the DLL has (both versions measured the same
/// table of `text_size`), and the plain rule in RGSS1 and RGSS2. See `docs/decisions.md`,
/// "Text and bitmaps of RGSS3".
pub(crate) fn metric_rules(runtime: Runtime) -> MetricRules {
    match runtime {
        Runtime::Rgss3 => MetricRules::Rgss3,
        Runtime::Rgss1 | Runtime::Rgss2 => MetricRules::Plain,
    }
}

/// A [`TextBackend`] that draws with a [`TextEngine`].
///
/// The binding's interface takes `&self` and the engine fills caches through `&mut self`;
/// everything runs on the interpreter thread, so a `RefCell` joins the two.
pub(crate) struct FontBackend {
    engine: RefCell<TextEngine>,
}

impl FontBackend {
    /// A backend over the fonts of `sources`.
    pub(crate) fn new(sources: &FontSources) -> FontBackend {
        FontBackend {
            engine: RefCell::new(TextEngine::new(sources)),
        }
    }

    /// The backend of a game of `runtime`: the fonts of the directories of
    /// [`font_directories`], then the fonts installed on the system, with the glyph rule of
    /// [`mask_rules`] for the version of the game's DLL and the metric rule of
    /// [`metric_rules`]. With `own_metrics` a font that replaces another is sized by its own
    /// line box.
    pub(crate) fn for_game(
        runtime: Runtime,
        game_dir: &Path,
        rtp_dirs: &[PathBuf],
        font_dirs: &[PathBuf],
        own_metrics: bool,
        version: Option<LibraryVersion>,
    ) -> FontBackend {
        let backend = FontBackend::new(&FontSources {
            directories: font_directories(runtime, game_dir, rtp_dirs, font_dirs),
            own_metrics,
            ..FontSources::system()
        });
        {
            let mut engine = backend.engine.borrow_mut();
            engine.set_mask_rules(mask_rules(runtime, version));
            engine.set_metric_rules(metric_rules(runtime));
        }
        backend
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

    fn font_exists_exactly(&self, name: &str) -> bool {
        self.engine.borrow().has_family_exactly(name)
    }
}

#[cfg(test)]
mod tests {
    use vkrgss_core::{Align, Bitmaps, FontDefaults, Rect};

    use super::*;

    /// The directory of the test fonts and measured samples of `vkrgss-text`.
    fn text_tests() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../vkrgss-text/tests")
    }

    /// A backend whose only source is the test fonts of `vkrgss-text`.
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

    /// Scenarios "The order of the directories" and "VX" of "Fonts of a game of RGSS3".
    #[test]
    fn a_game_of_rgss3_takes_the_fonts_of_its_packages_after_its_own() {
        let root = tempfile::tempdir().unwrap();
        let dir = |name: &str| root.path().join(name);
        let (named, game, first, second) = (dir("named"), dir("game"), dir("rtp1"), dir("rtp2"));
        for fonts in [named.clone(), game.join("Fonts"), second.join("Fonts")] {
            std::fs::create_dir_all(fonts).unwrap();
        }
        std::fs::create_dir_all(&first).unwrap();
        let rtp_dirs = [first, second.clone()];
        let font_dirs = [named.clone()];
        assert_eq!(
            font_directories(Runtime::Rgss3, &game, &rtp_dirs, &font_dirs),
            [named.clone(), game.join("Fonts"), second.join("Fonts")]
        );
        for runtime in [Runtime::Rgss1, Runtime::Rgss2] {
            assert_eq!(
                font_directories(runtime, &game, &rtp_dirs, &font_dirs),
                [named.clone(), game.join("Fonts")]
            );
        }
    }

    /// Scenario "A font of the run-time package": the backend over the directories of a game
    /// of RGSS3, without the system's fonts, which may have the same family.
    #[test]
    fn a_font_of_the_run_time_package_exists() {
        let root = tempfile::tempdir().unwrap();
        let (game, rtp) = (root.path().join("game"), root.path().join("rtp"));
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(rtp.join("Fonts")).unwrap();
        std::fs::copy(
            text_tests().join("fonts/LiberationSans-Regular.ttf"),
            rtp.join("Fonts/LiberationSans-Regular.ttf"),
        )
        .unwrap();
        let directories = font_directories(Runtime::Rgss3, &game, &[rtp], &[]);
        let backend = FontBackend::new(&FontSources::directories_only(directories));
        assert!(backend.font_exists_exactly("Liberation Sans"));
        let without = font_directories(Runtime::Rgss2, &game, &[root.path().join("rtp")], &[]);
        let backend = FontBackend::new(&FontSources::directories_only(without));
        assert!(!backend.font_exists_exactly("Liberation Sans"));
    }

    /// Scenario "The table of the decision" of "Glyph rule by the library of a game of
    /// RGSS3", and the rule a backend of a game is made with.
    #[test]
    fn the_glyph_rule_follows_the_version_of_the_dll() {
        let version = |numbers| Some(LibraryVersion(numbers));
        let rules: Vec<MaskRules> = [
            version([3, 0, 0, 1]),
            version([3, 0, 0, 7]),
            version([3, 0, 1, 1]),
            version([2, 0, 2, 1]),
            None,
        ]
        .into_iter()
        .map(|version| mask_rules(Runtime::Rgss3, version))
        .collect();
        assert_eq!(
            rules,
            [
                MaskRules::Rgss3Early,
                MaskRules::Rgss3Early,
                MaskRules::Rgss3,
                MaskRules::Rgss3,
                MaskRules::Rgss3,
            ]
        );
        assert_eq!(
            mask_rules(Runtime::Rgss2, version([3, 0, 0, 1])),
            MaskRules::Plain
        );
        let game = tempfile::tempdir().unwrap();
        for (runtime, expected) in [
            (Runtime::Rgss3, MaskRules::Rgss3Early),
            (Runtime::Rgss2, MaskRules::Plain),
        ] {
            let backend =
                FontBackend::for_game(runtime, game.path(), &[], &[], false, version([3, 0, 0, 1]));
            assert_eq!(backend.engine.borrow().mask_rules(), expected);
        }
    }

    /// Scenarios "The table" and "The backend of a game" of "Metric rule of a game of RGSS3".
    #[test]
    fn the_metric_rule_follows_the_runtime() {
        assert_eq!(metric_rules(Runtime::Rgss3), MetricRules::Rgss3);
        assert_eq!(metric_rules(Runtime::Rgss2), MetricRules::Plain);
        assert_eq!(metric_rules(Runtime::Rgss1), MetricRules::Plain);
        let version = |numbers| Some(LibraryVersion(numbers));
        let game = tempfile::tempdir().unwrap();
        let fonts = [text_tests().join("fonts")];
        for (runtime, version, expected) in [
            (Runtime::Rgss3, version([3, 0, 0, 1]), MetricRules::Rgss3),
            (Runtime::Rgss3, version([3, 0, 1, 1]), MetricRules::Rgss3),
            (Runtime::Rgss3, None, MetricRules::Rgss3),
            (Runtime::Rgss2, version([2, 0, 2, 1]), MetricRules::Plain),
            (Runtime::Rgss1, None, MetricRules::Plain),
        ] {
            let backend = FontBackend::for_game(runtime, game.path(), &[], &fonts, false, version);
            let engine = backend.engine.borrow();
            assert_eq!(engine.metric_rules(), expected, "{runtime:?} {version:?}");
            assert_eq!(engine.mask_rules(), mask_rules(runtime, version));
        }
    }

    /// `Font.exist?` of RGSS3 asks exactly, letter case included.
    #[test]
    fn font_exists_exactly_answers_from_the_engine() {
        let backend = backend();
        assert!(backend.font_exists_exactly("Liberation Sans"));
        assert!(backend.font_exists("liberation sans"));
        assert!(!backend.font_exists_exactly("liberation sans"));
        assert!(!backend.font_exists_exactly("NoSuchFamilyAnywhere"));
        // A name of the substitution table, as the table spells it, exists when a font that
        // replaces it is there: the known difference of `docs/compatibility.md`.
        assert!(backend.font_exists_exactly("Arial"));
        assert!(!backend.font_exists_exactly("arial"));
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
            for step in vkrgss_core::SQUEEZE_STEPS {
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
