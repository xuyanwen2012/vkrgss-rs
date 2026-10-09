//! Rasterisation and the glyph cache.

mod common;

use common::{LIBERATION, SYMBOLS, engine, max_coverage, request};
use vkxp_text::TextRequest;

// Requirement: Rasterisation

#[test]
fn mask_size() {
    let mut engine = engine();
    for size in [12, 22, 52] {
        for text in ["", " ", "I", "Hello, World!", "fj", "  padded  "] {
            let request = request(LIBERATION, size);
            let layout = engine.layout(&request, text);
            let width = layout.width() + layout.right_overhang;
            let mask = engine.rasterize(&request, text);
            assert_eq!(
                (mask.width, mask.height),
                (width, layout.height),
                "{text:?}"
            );
            assert_eq!(mask.left_overhang, layout.left_overhang);
            assert_eq!(mask.coverage.len(), (width * layout.height) as usize);
        }
    }
}

#[test]
fn ink_where_expected() {
    let mut engine = engine();
    let mask = engine.rasterize(&request(LIBERATION, 40), "I");
    assert!(mask.width >= 3);

    let middle = mask.width / 3..mask.width - mask.width / 3;
    assert_eq!(max_coverage(&mask, middle), 255);

    let at = |x: u32, y: u32| mask.coverage[(y * mask.width + x) as usize];
    let (right, bottom) = (mask.width - 1, mask.height - 1);
    assert_eq!(
        [at(0, 0), at(right, 0), at(0, bottom), at(right, bottom)],
        [0; 4]
    );
}

#[test]
fn glyphs_are_placed_at_the_sum_of_the_advances() {
    let mut engine = engine();
    let request = request(LIBERATION, 40);
    let i = engine.rasterize(&request, "I");
    let three = engine.rasterize(&request, "III");
    assert_eq!(three.width, 3 * i.width);
    for y in 0..i.height {
        let single = &i.coverage[(y * i.width) as usize..((y + 1) * i.width) as usize];
        for copy in 0..3 {
            let start = (y * three.width + copy * i.width) as usize;
            assert_eq!(&three.coverage[start..start + i.width as usize], single);
        }
    }
}

#[test]
fn coverage_outside_the_mask_is_clipped() {
    // At the pixels per em of Liberation Sans, glyphs of Noto Sans Symbols 2 are taller than
    // the line box and many are wider than their rounded advance.
    let mut engine = engine();
    for size in [1, 2, 3, 12, 52] {
        for text in ["\u{2BC5}\u{2B1B}\u{1F0A1}", "jfjf", "Åg_|"] {
            let request = TextRequest::new([LIBERATION, SYMBOLS], size);
            let layout = engine.layout(&request, text);
            let width = layout.width() + layout.right_overhang;
            let mask = engine.rasterize(&request, text);
            assert_eq!((mask.width, mask.height), (width, layout.height));
            assert_eq!(mask.coverage.len(), (width * layout.height) as usize);
        }
    }
}

#[test]
fn space() {
    let mut engine = engine();
    for size in [12, 22, 52] {
        let mask = engine.rasterize(&request(LIBERATION, size), " ");
        assert!(mask.width > 0);
        assert_eq!(
            mask.height,
            engine.measure(&request(LIBERATION, size), " ").1
        );
        assert!(mask.coverage.iter().all(|&value| value == 0));
    }
}

#[test]
fn repeatable() {
    let mut engine = engine();
    let request = request(LIBERATION, 22);
    let first = engine.rasterize(&request, "Repeat me");
    assert!(first.coverage.iter().any(|&value| value != 0));

    engine.rasterize(&common::request(LIBERATION, 52), "Something else");
    engine.rasterize(&common::request(SYMBOLS, 22), "\u{2BC5}");
    engine.measure(&common::request("Arial", 12), "Repeat me");
    assert_eq!(engine.rasterize(&request, "Repeat me"), first);

    // Another engine on the same font files gives the same mask.
    assert_eq!(common::engine().rasterize(&request, "Repeat me"), first);
}

// The glyph cache

#[test]
fn glyph_cache_is_bounded_and_does_not_change_results() {
    const LIMIT: usize = 16 * 1024;
    let texts = [
        "ABCDEFGHIJKLM",
        "NOPQRSTUVWXYZ",
        "abcdefghijklm",
        "nopqrstuvwxyz",
        "0123456789",
    ];
    let mut bounded = engine();
    bounded.set_glyph_cache_limit(LIMIT);
    let mut unbounded = engine();
    let mut evicted = false;
    for _ in 0..2 {
        for size in [52, 40, 22] {
            for text in texts {
                let request = request(LIBERATION, size);
                let before = bounded.glyph_cache_bytes();
                assert_eq!(
                    bounded.rasterize(&request, text),
                    unbounded.rasterize(&request, text)
                );
                assert!(bounded.glyph_cache_bytes() <= LIMIT);
                evicted |= bounded.glyph_cache_bytes() < before;
            }
        }
    }
    assert!(evicted, "the limit was never reached");
    assert!(unbounded.glyph_cache_bytes() > LIMIT);

    // A cache that can hold nothing still draws.
    bounded.set_glyph_cache_limit(0);
    assert_eq!(bounded.glyph_cache_bytes(), 0);
    let request = request(LIBERATION, 22);
    assert_eq!(
        bounded.rasterize(&request, texts[0]),
        unbounded.rasterize(&request, texts[0])
    );
    assert_eq!(bounded.glyph_cache_bytes(), 0);
}
