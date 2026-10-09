//! Per-character fallback.

mod common;

use common::{
    LIBERATION, LIBERATION_FILE, NOWHERE, RawFont, SYMBOL, SYMBOLS, SYMBOLS_FILE, copy_font,
    engine, max_coverage, request, test_sources,
};
use vkxp_text::{FontSources, TextEngine};

/// The test characters are what the tests assume them to be.
#[test]
fn test_characters() {
    let liberation = RawFont::load(LIBERATION_FILE);
    let symbols = RawFont::load(SYMBOLS_FILE);
    assert!(!liberation.has(SYMBOL) && symbols.has(SYMBOL));
    assert!(!liberation.has(NOWHERE) && !symbols.has(NOWHERE));
    assert!(liberation.has('A') && liberation.has('B'));
}

// Requirement: Per-character fallback

#[test]
fn mixed_string() {
    let liberation = RawFont::load(LIBERATION_FILE);
    let symbols = RawFont::load(SYMBOLS_FILE);
    let text = format!("A{SYMBOL}B");

    for sources in [
        // The fallback family is named by the caller.
        FontSources {
            fallback_families: Some(vec![SYMBOLS.to_owned()]),
            ..test_sources()
        },
        // It is not among the default fallback families and is found among all fonts.
        test_sources(),
    ] {
        let mut engine = TextEngine::new(&sources);
        for size in [12, 22, 52] {
            let request = request(LIBERATION, size);
            // The symbol is drawn from the other font and set at the advance of the
            // selected font's missing glyph.
            let pixels_per_em = liberation.pixels_per_em(size);
            let a = liberation.advance('A', pixels_per_em);
            let symbol = liberation.advance(SYMBOL, pixels_per_em);
            let b = liberation.advance('B', pixels_per_em);
            assert_ne!(symbol, symbols.advance(SYMBOL, pixels_per_em));
            let height = engine.measure(&request, "A").1;

            assert_eq!(engine.measure(&request, "A").0, a);
            assert_eq!(engine.measure(&request, &SYMBOL.to_string()).0, symbol);
            assert_eq!(engine.measure(&request, "B").0, b);
            assert_eq!(engine.layout(&request, &text).advances, [a, symbol, b]);
            assert_eq!(engine.measure(&request, &text), (a + symbol + b, height));

            let mask = engine.rasterize(&request, &text);
            assert_eq!(mask.height, height);
            assert!(mask.width >= a + symbol + b);
            assert!(max_coverage(&mask, 0..a) > 0);
            assert!(max_coverage(&mask, a..a + symbol) > 0);
            assert!(max_coverage(&mask, a + symbol..a + symbol + b) > 0);

            // The rest of the string stays in the selected font.
            assert_eq!(engine.resolve(&request).unwrap().family, LIBERATION);
            let alone = engine.rasterize(&request, "A");
            for y in 0..height {
                let row = (y * mask.width) as usize;
                let alone_row = (y * alone.width) as usize;
                // The last column may also hold the symbol's left edge.
                let columns = a as usize - 1;
                assert_eq!(
                    mask.coverage[row..row + columns],
                    alone.coverage[alone_row..alone_row + columns]
                );
            }
        }
    }
}

#[test]
fn fallback_glyph_is_the_fallback_fonts_drawing() {
    // The symbol drawn through fallback differs from the selected font's missing-glyph box.
    let only_liberation = tempfile::tempdir().unwrap();
    copy_font(only_liberation.path(), LIBERATION_FILE);
    let mut without = TextEngine::new(&FontSources::directories_only([only_liberation.path()]));
    let mut with = engine();
    let request = request(LIBERATION, 40);
    let text = SYMBOL.to_string();
    assert_ne!(
        with.rasterize(&request, &text),
        without.rasterize(&request, &text)
    );
}

#[test]
fn no_font_has_the_character() {
    let liberation = RawFont::load(LIBERATION_FILE);
    let mut engine = engine();
    for size in [12, 22, 52] {
        let request = request(LIBERATION, size);
        let pixels_per_em = liberation.pixels_per_em(size);
        // `RawFont::advance` gives the missing-glyph box for a character the font lacks.
        let missing = liberation.advance(NOWHERE, pixels_per_em);
        let a = liberation.advance('A', pixels_per_em);
        assert!(missing > 0);

        let height = engine.measure(&request, "A").1;
        let text = format!("A{NOWHERE}{NOWHERE}A");
        assert_eq!(
            engine.measure(&request, &text),
            (2 * a + 2 * missing, height)
        );
        let overhang = engine.layout(&request, &text).right_overhang;
        let mask = engine.rasterize(&request, &text);
        assert_eq!(mask.width, 2 * a + 2 * missing + overhang);
        assert_eq!(mask.height, height);
        assert!(max_coverage(&mask, 0..a) > 0);

        let text = NOWHERE.to_string();
        assert_eq!(engine.measure(&request, &text), (missing, height));
        assert_eq!(engine.rasterize(&request, &text).width, missing);
    }
}
