//! The metric rule of RGSS3: glyphs hinted as Wine's FreeType hints them.
//!
//! The rule is checked on the committed test fonts and on copies of them whose hinting tables
//! are changed. The `text_size` table of the fonts of the run-time package of VX Ace
//! (`tests/data/rgss3-text-size-samples.txt`) needs those fonts, which are not committed: the
//! test reads them from the directory `VKXP_RGSS3_FONTS` names and is skipped without it.

mod common;

use std::path::{Path, PathBuf};

use common::{LIBERATION, LIBERATION_FILE, SYMBOLS, SYMBOLS_FILE, copy_font, engine, fonts_dir};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::pen::NullPen;
use skrifa::outline::{DrawSettings, Engine, HintingInstance, HintingOptions, SmoothMode, Target};
use skrifa::{FontRef, MetadataProvider};
use vkxp_text::{
    FontSources, HintingEngine, MaskRules, MetricRules, TextEngine, TextLayout, TextRequest,
};

fn rgss3(mut engine: TextEngine) -> TextEngine {
    engine.set_metric_rules(MetricRules::Rgss3);
    engine
}

fn styled(family: &str, size: u32, bold: bool, italic: bool) -> TextRequest {
    TextRequest {
        bold,
        italic,
        ..TextRequest::new([family], size)
    }
}

/// An engine that knows only the fonts in `directory`, and no fallback family.
fn engine_on(directory: &Path) -> TextEngine {
    TextEngine::new(&FontSources {
        fallback_families: Some(Vec::new()),
        ..FontSources::directories_only([directory])
    })
}

/// How a copy of a test font changes its table directory.
enum Table {
    /// The table is renamed, so that the font no longer has it.
    Without(&'static [u8; 4]),
    /// The table's length in the directory is set to this many bytes.
    Length(&'static [u8; 4], u32),
}

/// Writes a copy of the test font `file` to `directory` with its table directory changed.
/// A renamed table keeps its place in the sorted directory: only its last letter changes.
fn copy_with_tables(directory: &Path, file: &str, changes: &[Table]) -> PathBuf {
    let mut font = std::fs::read(fonts_dir().join(file)).expect("read test font");
    let table_count = usize::from(u16::from_be_bytes([font[4], font[5]]));
    for position in 0..table_count {
        let record = 12 + 16 * position;
        for change in changes {
            match change {
                Table::Without(tag) if &font[record..record + 4] == *tag => {
                    font[record + 3] = b'~';
                }
                Table::Length(tag, length) if &font[record..record + 4] == *tag => {
                    font[record + 12..record + 16].copy_from_slice(&length.to_be_bytes());
                }
                _ => {}
            }
        }
    }
    let target = directory.join(file);
    std::fs::write(&target, font).expect("write the copy");
    target
}

// Requirement: Metric rules

#[test]
fn the_plain_rule_is_the_default() {
    assert_eq!(engine().metric_rules(), MetricRules::Plain);
}

#[test]
fn the_rule_of_rgss3_does_not_depend_on_the_mask_rules() {
    let results: Vec<_> = [MaskRules::Plain, MaskRules::Rgss3, MaskRules::Rgss3Early]
        .into_iter()
        .map(|rules| {
            let mut engine = rgss3(engine());
            engine.set_mask_rules(rules);
            let request = styled(LIBERATION, 24, true, true);
            let text = "Hello World";
            (
                engine.measure(&request, text),
                engine.layout(&request, text),
            )
        })
        .collect();
    assert_eq!(results[0], results[1]);
    assert_eq!(results[0], results[2]);
}

#[test]
fn switching_back_to_the_plain_rule() {
    let mut engine = engine();
    let request = styled(LIBERATION, 24, false, true);
    let text = "Hello World";
    let mut set = |rules| {
        engine.set_metric_rules(rules);
        (
            engine.layout(&request, text),
            engine.rasterize(&request, text),
        )
    };
    let first = set(MetricRules::Plain);
    let hinted = set(MetricRules::Rgss3);
    let third = set(MetricRules::Plain);
    assert_eq!(first, third);
    assert_ne!(first.1, hinted.1);
}

// Requirement: Hinted outlines of RGSS3

#[test]
fn the_engine_of_each_test_font() {
    let mut engine = engine();
    assert_eq!(
        engine.hinting_engine(&TextRequest::new([LIBERATION], 24)),
        Some(HintingEngine::Font)
    );
    assert_eq!(
        engine.hinting_engine(&TextRequest::new([SYMBOLS], 24)),
        Some(HintingEngine::Automatic)
    );
}

#[test]
fn the_size_of_the_prep_table_chooses_the_engine() {
    for (length, expected) in [
        (0, HintingEngine::Automatic),
        (7, HintingEngine::Automatic),
        (8, HintingEngine::Font),
    ] {
        let directory = tempfile::tempdir().unwrap();
        copy_with_tables(
            directory.path(),
            LIBERATION_FILE,
            &[Table::Without(b"fpgm"), Table::Length(b"prep", length)],
        );
        let mut engine = engine_on(directory.path());
        let found = engine.hinting_engine(&TextRequest::new([LIBERATION], 24));
        assert_eq!(found, Some(expected), "prep of {length} bytes");
    }
}

/// The hinted advance of `ch` at `pixels_per_em`, rounded up, as `skrifa` gives it with the
/// engine `engine`.
fn hinted_advance(font: &FontRef<'_>, engine: HintingEngine, ch: char, pixels_per_em: u32) -> u32 {
    let outlines = font.outline_glyphs();
    let options = HintingOptions {
        engine: match engine {
            HintingEngine::Automatic => Engine::Auto(None),
            HintingEngine::Font => Engine::Interpreter,
        },
        target: Target::Smooth {
            mode: SmoothMode::Normal,
            symmetric_rendering: true,
            preserve_linear_metrics: false,
        },
    };
    let size = Size::new(pixels_per_em as f32);
    let instance = HintingInstance::new(&outlines, size, LocationRef::default(), options)
        .expect("hinting instance");
    let glyph = font.charmap().map(ch).unwrap_or_default();
    let metrics = outlines
        .get(glyph)
        .expect("outline")
        .draw(DrawSettings::hinted(&instance, false), &mut NullPen)
        .expect("hinted draw");
    let advance = (metrics.advance_width.expect("hinted advance") * 64.0).round() as u32;
    advance.div_ceil(64)
}

#[test]
fn advances_are_hinted_advances_rounded_up() {
    for (family, file, hinting) in [
        (LIBERATION, LIBERATION_FILE, HintingEngine::Font),
        (SYMBOLS, SYMBOLS_FILE, HintingEngine::Automatic),
    ] {
        // Alone, so that a character the font lacks is its missing glyph.
        let directory = tempfile::tempdir().unwrap();
        copy_font(directory.path(), file);
        let mut engine = rgss3(engine_on(directory.path()));
        let data = std::fs::read(fonts_dir().join(file)).unwrap();
        let font = FontRef::new(&data).unwrap();
        let text = "Hello World";
        for size in 6..=96 {
            let layout = engine.layout(&TextRequest::new([family], size), text);
            let expected: Vec<u32> = text
                .chars()
                .map(|ch| hinted_advance(&font, hinting, ch, layout.pixels_per_em))
                .collect();
            assert_eq!(layout.advances, expected, "{family} at {size}");
        }
    }
}

// Requirement: Advances of RGSS3

#[test]
fn a_synthesised_bold_adds_a_pixel_to_every_hinted_advance() {
    let mut engine = rgss3(engine());
    for size in [6, 12, 24, 48, 96] {
        let regular = engine.layout(&styled(LIBERATION, size, false, false), "Hello");
        let bold = engine.layout(&styled(LIBERATION, size, true, false), "Hello");
        let wider: Vec<u32> = regular.advances.iter().map(|advance| advance + 1).collect();
        assert_eq!(bold.advances, wider, "at {size}");
    }
}

// Requirement: Glyph box of RGSS3

/// The drawn width `E` of the sweeps of RGSS3 for "Arial", which was Liberation Sans: the
/// advances, the first glyph's left overhang and the last glyph's right overhang.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, the sweeps without `lfWidth`.
#[test]
fn measured_drawn_widths() {
    let mut engine = rgss3(engine());
    let drawn = |layout: TextLayout| layout.width() + layout.right_overhang;
    let layout = engine.layout(&styled(LIBERATION, 20, false, false), "HHHHHHHHHH");
    assert_eq!(drawn(layout), 130);
    let layout = engine.layout(&styled(LIBERATION, 33, false, true), "Hello World");
    assert_eq!(drawn(layout), 153);
}

// Requirement: Glyph images of RGSS3

#[test]
fn a_hinted_glyph_is_not_the_plain_glyph() {
    let directory = tempfile::tempdir().unwrap();
    copy_with_tables(
        directory.path(),
        LIBERATION_FILE,
        &[Table::Without(b"fpgm"), Table::Without(b"prep")],
    );
    let mut engine = engine_on(directory.path());
    let request = TextRequest::new([LIBERATION], 24);
    assert_eq!(
        engine.hinting_engine(&request),
        Some(HintingEngine::Automatic)
    );
    let plain = engine.rasterize(&request, "H");
    engine.set_metric_rules(MetricRules::Rgss3);
    let hinted = engine.rasterize(&request, "H");
    assert_ne!(plain.coverage, hinted.coverage);
}

// The samples measured on the original runtime.

/// The lines of `kind` of the sample file `file` under `tests/data`, split into fields.
fn samples(file: &str, kind: &str) -> Vec<Vec<String>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(file);
    let text = std::fs::read_to_string(path).expect("read the samples");
    let lines: Vec<Vec<String>> = text
        .lines()
        .map(|line| line.split_whitespace().map(str::to_owned).collect())
        .filter(|fields: &Vec<String>| fields.first().is_some_and(|first| first == kind))
        .collect();
    assert!(!lines.is_empty(), "no {kind} samples");
    lines
}

fn number(field: &str) -> u32 {
    field.parse().expect("a number")
}

/// A string written as the hexadecimal digits of its UTF-8 bytes, `-` for the empty string.
fn string(field: &str) -> String {
    if field == "-" {
        return String::new();
    }
    let bytes = (0..field.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&field[at..at + 2], 16).expect("hexadecimal"))
        .collect();
    String::from_utf8(bytes).expect("UTF-8")
}

// Requirement: Average width of RGSS3

/// `M liberation size ppem ascent descent ave` of `text-samples.txt`.
#[test]
fn measured_sizes_of_liberation_sans() {
    let mut engine = rgss3(engine());
    let samples: Vec<_> = samples("text-samples.txt", "M")
        .into_iter()
        .filter(|sample| sample[1] == "liberation")
        .collect();
    assert_eq!(samples.len(), 91);
    for sample in samples {
        let size = number(&sample[2]);
        let layout = engine.layout(&TextRequest::new([LIBERATION], size), "H");
        let found = [
            layout.pixels_per_em,
            layout.ascent,
            layout.height - layout.ascent,
            layout.average_width,
        ];
        let expected = [3, 4, 5, 6].map(|field| number(&sample[field]));
        assert_eq!(found, expected, "Liberation Sans at {size}");
    }
}

// Requirement: Text size measured on RGSS3

/// The sizes `text_size` of RGSS3 gave for "Arial" at size 24, which was Liberation Sans
/// (Wine's replacement) in its own line box.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, the same on RGSS300 3.0.0.1.
#[test]
fn measured_sizes_of_rgss3_with_either_metric_rule() {
    for rules in [MetricRules::Plain, MetricRules::Rgss3] {
        let mut engine = engine();
        engine.set_metric_rules(rules);
        let request = TextRequest::new([LIBERATION], 24);
        assert_eq!(engine.measure(&request, "Hello"), (49, 23), "{rules:?}");
        assert_eq!(engine.measure(&request, "W"), (20, 23), "{rules:?}");
        assert_eq!(engine.measure(&request, "il"), (10, 23), "{rules:?}");
    }
}

#[test]
fn a_synthesised_bold_adds_a_pixel_to_every_character() {
    let mut engine = rgss3(engine());
    let (width, height) = engine.measure(&styled(LIBERATION, 24, false, false), "Hello");
    assert_eq!(
        engine.measure(&styled(LIBERATION, 24, true, false), "Hello"),
        (width + 5, height)
    );
}

/// The committed table: 39 strings, and 728 lines of 39 values, one for each font, size from
/// 6 to 96 and style.
#[test]
fn the_table_of_the_run_time_package_is_complete() {
    let file = "rgss3-text-size-samples.txt";
    let strings = &samples(file, "STR")[0][1..];
    assert_eq!(strings.len(), 39);
    assert!(
        strings
            .iter()
            .map(|field| string(field))
            .any(|text| text.is_empty())
    );
    let lines = samples(file, "ts");
    assert_eq!(lines.len(), 728);
    for line in &lines {
        assert_eq!(line.len(), 6 + strings.len(), "{}", line[1..5].join(" "));
    }
}

/// `ts font size bold italic | width,height ...` of `rgss3-text-size-samples.txt`, one value
/// for each string of its `STR` line, in VL Gothic and VL PGothic of the run-time package of
/// VX Ace.
#[test]
fn measured_text_sizes_of_the_fonts_of_the_run_time_package() {
    let Some(directory) = std::env::var_os("VKXP_RGSS3_FONTS") else {
        eprintln!(
            "skipped: VKXP_RGSS3_FONTS does not name a directory with VL-Gothic-Regular.ttf \
             and VL-PGothic-Regular.ttf"
        );
        return;
    };
    let directory = PathBuf::from(directory);
    for file in ["VL-Gothic-Regular.ttf", "VL-PGothic-Regular.ttf"] {
        assert!(directory.join(file).is_file(), "{file} in {directory:?}");
    }
    let file = "rgss3-text-size-samples.txt";
    let strings: Vec<String> = samples(file, "STR")[0][1..]
        .iter()
        .map(|field| string(field))
        .collect();
    let mut engine = rgss3(engine_on(&directory));
    let (mut checked, mut wrong) = (0, Vec::new());
    for sample in samples(file, "ts") {
        let family = sample[1].replace('_', " ");
        let request = styled(
            &family,
            number(&sample[2]),
            sample[3] == "true",
            sample[4] == "true",
        );
        assert_eq!(sample[5], "|");
        assert_eq!(sample.len() - 6, strings.len());
        for (text, value) in strings.iter().zip(&sample[6..]) {
            let (width, height) = value.split_once(',').expect("width,height");
            let expected = (number(width), number(height));
            let found = engine.measure(&request, text);
            if found != expected {
                wrong.push(format!(
                    "{} {:?}: {found:?}, measured {expected:?}",
                    sample[1..5].join(" "),
                    text
                ));
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 28392);
    assert!(
        wrong.is_empty(),
        "{} of {} values:\n{}",
        wrong.len(),
        checked,
        wrong.join("\n")
    );
}
