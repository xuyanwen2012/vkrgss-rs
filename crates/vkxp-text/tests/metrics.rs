//! Size, bold and italic selection, and measurement.

mod common;

use common::{LIBERATION, LIBERATION_FILE, RawFont, SYMBOL, engine, request};
use vkxp_text::TextRequest;

// Requirement: Size, bold and italic

#[test]
fn height_equals_size() {
    let mut engine = engine();
    let mixed = format!("Ag{SYMBOL}");
    for size in [12, 22, 52] {
        for text in ["", " ", "Ag", "Éj|", mixed.as_str()] {
            let request = request(LIBERATION, size);
            assert_eq!(engine.measure(&request, text).1, size, "{text:?} at {size}");
            assert_eq!(engine.rasterize(&request, text).height, size);
        }
    }
}

#[test]
fn baseline_follows_the_line_metrics() {
    // The line box is the font's Windows ascent plus descent scaled to the size, so the ink
    // of a letter without descender ends at the rounded ascent and that of `g` goes below.
    let raw = RawFont::load(LIBERATION_FILE);
    let (ascent, descent) = raw.win_metrics();
    let mut engine = engine();
    for size in [12, 22, 52] {
        let baseline = (size as f32 * ascent / (ascent + descent)).round() as u32;
        let last_ink_row = |text: &str, engine: &mut vkxp_text::TextEngine| {
            let mask = engine.rasterize(&request(LIBERATION, size), text);
            (0..mask.height)
                .rev()
                .find(|&y| {
                    let row = (y * mask.width) as usize;
                    mask.coverage[row..row + mask.width as usize]
                        .iter()
                        .any(|&value| value != 0)
                })
                .expect("ink")
        };
        assert_eq!(last_ink_row("H", &mut engine) + 1, baseline, "size {size}");
        assert!(last_ink_row("g", &mut engine) + 1 > baseline, "size {size}");
    }
}

#[test]
fn bold_face_absent() {
    // The test directory holds only the regular face of Liberation Sans.
    let mut engine = engine();
    let regular = request(LIBERATION, 22);
    for (bold, italic) in [(true, false), (false, true), (true, true)] {
        let styled = TextRequest {
            bold,
            italic,
            ..regular.clone()
        };
        let face = engine.resolve(&styled).unwrap();
        assert_eq!(face.family, LIBERATION);
        assert_eq!((face.weight, face.italic), (400, false));
        assert_eq!(engine.resolve(&regular).unwrap(), face);
        assert_eq!(
            engine.measure(&styled, "Bold text"),
            engine.measure(&regular, "Bold text")
        );
        assert_eq!(
            engine.rasterize(&styled, "Bold text"),
            engine.rasterize(&regular, "Bold text")
        );
    }
}

// Requirement: Measurement

#[test]
fn additive_widths() {
    let mut engine = engine();
    for size in [12, 22, 52] {
        let request = request(LIBERATION, size);
        let a = engine.measure(&request, "A").0;
        let b = engine.measure(&request, "B").0;
        assert!(a > 0 && b > 0);
        assert_eq!(engine.measure(&request, "AB").0, a + b);
        // No kerning: a pair that fonts kern measures as its parts.
        let v = engine.measure(&request, "V").0;
        assert_eq!(engine.measure(&request, "AVA").0, a + v + a);
    }
}

#[test]
fn widths_are_rounded_advances_of_the_font() {
    let raw = RawFont::load(LIBERATION_FILE);
    let mut engine = engine();
    for size in [12, 22, 52] {
        let pixels_per_em = raw.pixels_per_em(size);
        let text = "Hello, World! 0123";
        let expected: u32 = text.chars().map(|ch| raw.advance(ch, pixels_per_em)).sum();
        assert_eq!(engine.measure(&request(LIBERATION, size), text).0, expected);
    }
}

#[test]
fn empty_string() {
    let mut engine = engine();
    assert_eq!(engine.measure(&request(LIBERATION, 22), ""), (0, 22));
    let mask = engine.rasterize(&request(LIBERATION, 22), "");
    assert_eq!((mask.width, mask.height, mask.coverage.len()), (0, 22, 0));
}

#[test]
fn size_zero() {
    let mut engine = engine();
    assert_eq!(engine.measure(&request(LIBERATION, 0), "Text"), (0, 0));
    let mask = engine.rasterize(&request(LIBERATION, 0), "Text");
    assert_eq!((mask.width, mask.height, mask.coverage.len()), (0, 0, 0));
}

#[test]
fn control_characters_do_not_start_a_line() {
    let mut engine = engine();
    let request = request(LIBERATION, 22);
    let plain = engine.measure(&request, "AB");
    for text in ["A\nB", "A\r\nB", "A\tB", "\nAB\n", "A\u{0}B"] {
        assert_eq!(engine.measure(&request, text), plain, "{text:?}");
        assert_eq!(
            engine.rasterize(&request, text),
            engine.rasterize(&request, "AB"),
            "{text:?}"
        );
    }
}
