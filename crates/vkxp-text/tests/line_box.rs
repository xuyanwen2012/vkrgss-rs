//! Line boxes of replaced families.

mod common;

use common::{
    CHINESE_PRC, Derivation, LIBERATION, LIBERATION_FILE, NAME_FAMILY, RawFont, SYMBOL,
    derive_font, engine, request, test_sources,
};
use vkxp_text::{FontSources, LineBox, TextEngine, TextMask, default_line_box};

/// One em wide in Liberation Sans.
const EM_DASH: &str = "\u{2014}";

/// A line box exactly one em high.
const ONE_EM: LineBox = LineBox {
    ascent: 800,
    descent: 200,
    units_per_em: 1000,
};

/// An engine on the test fonts in which Liberation Sans replaces `family`.
fn engine_replacing(family: &str, line_boxes: &[(&str, LineBox)]) -> TextEngine {
    TextEngine::new(&FontSources {
        substitutions: vec![(family.to_owned(), vec![LIBERATION.to_owned()])],
        line_boxes: line_boxes
            .iter()
            .map(|(name, line_box)| ((*name).to_owned(), *line_box))
            .collect(),
        ..test_sources()
    })
}

/// The number of rows from the top of `mask` to its lowest row with ink.
fn ink_bottom(mask: &TextMask) -> u32 {
    (0..mask.height)
        .rev()
        .find(|&y| {
            let row = (y * mask.width) as usize;
            mask.coverage[row..row + mask.width as usize]
                .iter()
                .any(|&value| value != 0)
        })
        .expect("ink")
        + 1
}

/// The test character is what the tests assume it to be.
#[test]
fn em_dash_is_one_em_wide() {
    let raw = RawFont::load(LIBERATION_FILE);
    assert!(raw.has('\u{2014}'));
    assert_eq!(raw.advance('\u{2014}', 22), 22);
    assert_eq!(raw.advance('\u{2014}', raw.pixels_per_em(22)), 20);
}

// Requirement: Line box of a replaced family

#[test]
fn replacement_takes_the_size_of_the_family_it_replaces() {
    let mut engine = engine_replacing("SimHei", &[("SimHei", ONE_EM)]);
    assert!(!engine.families().iter().any(|family| family == "SimHei"));
    let replaced = request("SimHei", 22);
    assert_eq!(engine.resolve(&replaced).unwrap().family, LIBERATION);
    assert_eq!(engine.measure(&replaced, EM_DASH).0, 22);
    assert!(engine.measure(&request(LIBERATION, 22), EM_DASH).0 < 22);

    // The baseline lies the replaced family's share of ascent below the top.
    assert_eq!(ink_bottom(&engine.rasterize(&replaced, "H")), 18);
    // A character from a fallback font is as wide as the replacement's missing glyph at
    // the same pixels per em.
    let liberation = RawFont::load(LIBERATION_FILE);
    assert_eq!(
        engine.layout(&replaced, &SYMBOL.to_string()).advances,
        [liberation.advance(SYMBOL, 22)]
    );
}

#[test]
fn height_stays_the_size() {
    let mut engine = engine_replacing("SimHei", &[("SimHei", ONE_EM)]);
    let replaced = request("SimHei", 22);
    let mixed = format!("Ag{SYMBOL}");
    for text in [" ", "Ag", "Éj|", EM_DASH, mixed.as_str()] {
        let (width, height) = engine.measure(&replaced, text);
        assert_eq!(height, 22, "{text:?}");
        let width = width + engine.layout(&replaced, text).right_overhang;
        let mask = engine.rasterize(&replaced, text);
        assert_eq!((mask.width, mask.height), (width, 22), "{text:?}");
        assert_eq!(mask.coverage.len(), (width * 22) as usize, "{text:?}");
    }
}

/// The line box Liberation Sans is set in when it is requested by name.
fn liberation_line_box() -> LineBox {
    let raw = RawFont::load(LIBERATION_FILE);
    let (ascent, descent) = raw.win_metrics();
    LineBox {
        ascent: ascent as u16,
        descent: descent as u16,
        units_per_em: raw.units_per_em() as u16,
    }
}

// Requirement: Known line boxes

#[test]
fn built_in_chinese_family() {
    for (family, written) in [("黑体", "黑体"), ("SimHei", "simhei"), ("宋体", "宋体")] {
        let mut engine = engine_replacing(family, &[]);
        let replaced = request(written, 22);
        assert_eq!(engine.resolve(&replaced).unwrap().family, LIBERATION);
        assert_eq!(engine.measure(&replaced, EM_DASH).0, 22, "{written}");
        // 220 of 256 units above the baseline.
        assert_eq!(ink_bottom(&engine.rasterize(&replaced, "H")), 19);
    }
}

#[test]
fn arial_agrees_with_its_metric_compatible_replacement() {
    assert_eq!(default_line_box("Arial"), Some(liberation_line_box()));
    assert_eq!(default_line_box("ARIAL"), Some(liberation_line_box()));
}

#[test]
fn every_required_family_has_a_built_in_line_box() {
    let required = [
        "Arial",
        "SimSun",
        "宋体",
        "SimHei",
        "黑体",
        "MS Gothic",
        "MS Mincho",
    ];
    for family in required {
        assert!(default_line_box(family).is_some(), "{family}");
        assert!(
            default_line_box(&family.to_uppercase()).is_some(),
            "the entry for {family} depends on letter case"
        );
    }
    assert_eq!(default_line_box("Microsoft YaHei"), None);
}

#[test]
fn callers_line_box_takes_precedence() {
    // The built-in table replaces Arial by Liberation Sans.
    let mut engine = TextEngine::new(&FontSources {
        line_boxes: vec![("ARIAL".to_owned(), ONE_EM)],
        ..test_sources()
    });
    let replaced = request("Arial", 22);
    assert_eq!(engine.resolve(&replaced).unwrap().family, LIBERATION);
    assert_eq!(engine.measure(&replaced, EM_DASH).0, 22);

    // Without the caller's entry the built-in line box of Arial applies, which is Liberation
    // Sans's own.
    let mut engine = common::engine();
    let text = format!("Hello, World! {EM_DASH}");
    assert_eq!(
        engine.rasterize(&replaced, &text),
        engine.rasterize(&request(LIBERATION, 22), &text)
    );
}

// Requirement: Own line box otherwise

#[test]
fn unknown_family() {
    let text = format!("Hello, World! {EM_DASH}{SYMBOL}");
    // `Helvetica` has a built-in replacement list and `My Game Font` a caller's; neither has
    // a line box.
    for family in ["Helvetica", "My Game Font"] {
        assert_eq!(default_line_box(family), None);
        let mut engine = engine_replacing("My Game Font", &[]);
        for size in [12, 22, 52] {
            let replaced = request(family, size);
            let named = request(LIBERATION, size);
            assert_eq!(engine.resolve(&replaced).unwrap().family, LIBERATION);
            assert_eq!(
                engine.measure(&replaced, &text),
                engine.measure(&named, &text)
            );
            assert_eq!(
                engine.rasterize(&replaced, &text),
                engine.rasterize(&named, &text)
            );
        }
    }
}

#[test]
fn installed_family() {
    // Liberation Sans under a second name that has a built-in line box.
    let directory = tempfile::tempdir().unwrap();
    let path = derive_font(
        LIBERATION_FILE,
        &directory.path().join("named.ttf"),
        Derivation {
            names: &[(NAME_FAMILY, CHINESE_PRC, "黑体")],
            ..Derivation::default()
        },
    );
    let mut engine = TextEngine::new(&FontSources {
        // A line box of the caller does not apply to an installed family either.
        line_boxes: vec![("黑体".to_owned(), ONE_EM)],
        ..FontSources::directories_only([directory.path()])
    });
    assert!(default_line_box("黑体").is_some());
    let installed = request("黑体", 22);
    let face = engine.resolve(&installed).unwrap();
    assert_eq!(face.path.canonicalize().unwrap(), path);

    let raw = RawFont::load(LIBERATION_FILE);
    let expected = raw.advance('\u{2014}', raw.pixels_per_em(22));
    assert_eq!(engine.measure(&installed, EM_DASH).0, expected);
    let text = format!("Hello, World! {EM_DASH}");
    assert_eq!(
        engine.rasterize(&installed, &text),
        engine.rasterize(&request(LIBERATION, 22), &text)
    );
}

#[test]
fn fallback_font() {
    // Nothing replaces `黑体` here, so the fallback family draws the text, in its own line box.
    let mut engine = engine();
    assert!(!engine.is_available("黑体"));
    let text = format!("Hello, World! {EM_DASH}");
    assert_eq!(
        engine.resolve(&request("黑体", 22)).unwrap().family,
        LIBERATION
    );
    assert_eq!(
        engine.rasterize(&request("黑体", 22), &text),
        engine.rasterize(&request(LIBERATION, 22), &text)
    );
}

// Requirement: Own line box on request

#[test]
fn replacement_font_at_its_own_size() {
    let text = format!("Hello, World! {EM_DASH}{SYMBOL}");
    // A built-in line box and one of the caller: neither applies.
    for line_boxes in [&[][..], &[("SimHei", ONE_EM)]] {
        let mut engine = TextEngine::new(&FontSources {
            substitutions: vec![("SimHei".to_owned(), vec![LIBERATION.to_owned()])],
            line_boxes: line_boxes
                .iter()
                .map(|(name, line_box)| ((*name).to_owned(), *line_box))
                .collect(),
            own_metrics: true,
            ..test_sources()
        });
        let replaced = request("SimHei", 22);
        let named = request(LIBERATION, 22);
        assert_eq!(engine.resolve(&replaced).unwrap().family, LIBERATION);
        assert_eq!(engine.measure(&replaced, &text).1, 22);
        assert_eq!(
            engine.measure(&replaced, &text),
            engine.measure(&named, &text)
        );
        assert_eq!(engine.measure(&replaced, EM_DASH).0, 20);
        assert_eq!(
            engine.rasterize(&replaced, &text),
            engine.rasterize(&named, &text)
        );
    }
}

#[test]
fn default_unchanged() {
    assert!(!test_sources().own_metrics);
    // Without the configuration the replacement is still set in the line box of `SimHei`,
    // in which the em is as high as the size.
    let mut engine = engine_replacing("SimHei", &[]);
    let replaced = request("SimHei", 22);
    assert_eq!(engine.measure(&replaced, EM_DASH), (22, 22));
    assert_eq!(ink_bottom(&engine.rasterize(&replaced, "H")), 19);
}
