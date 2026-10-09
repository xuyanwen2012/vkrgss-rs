//! Size, bold and italic selection, and measurement.

mod common;

use common::{
    LIBERATION, LIBERATION_FILE, RawFont, SYMBOL, engine, max_coverage, request, test_sources,
};
use vkxp_text::{FontSources, TextEngine, TextRequest};

// Requirement: Size, bold and italic

#[test]
fn height_equals_size() {
    // Liberation Sans in the line box of `SimHei`, which is exactly one em high.
    let mut engine = TextEngine::new(&FontSources {
        substitutions: vec![("SimHei".to_owned(), vec![LIBERATION.to_owned()])],
        ..test_sources()
    });
    let mixed = format!("Ag{SYMBOL}");
    for size in [12, 22, 52] {
        for text in [" ", "Ag", "Éj|", mixed.as_str()] {
            let request = request("SimHei", size);
            assert_eq!(engine.measure(&request, text).1, size, "{text:?} at {size}");
            assert_eq!(engine.rasterize(&request, text).height, size);
            assert_eq!(engine.layout(&request, text).pixels_per_em, size);
        }
    }
}

#[test]
fn height_is_the_rounded_ascent_plus_descent() {
    let raw = RawFont::load(LIBERATION_FILE);
    let mut engine = engine();
    let mut differs = 0;
    for size in 6..=96 {
        let (ascent, descent) = raw.cell(size);
        let layout = engine.layout(&request(LIBERATION, size), "Ag");
        assert_eq!(layout.pixels_per_em, raw.pixels_per_em(size), "size {size}");
        assert_eq!((layout.ascent, layout.height), (ascent, ascent + descent));
        assert_eq!(
            engine.measure(&request(LIBERATION, size), "Ag").1,
            ascent + descent
        );
        differs += u32::from(ascent + descent != size);
    }
    assert!(differs > 0, "the height is not always the size");
}

#[test]
fn baseline_follows_the_line_metrics() {
    // The ink of a letter without descender ends at the rounded ascent and that of `g` goes
    // below.
    let raw = RawFont::load(LIBERATION_FILE);
    let mut engine = engine();
    for size in [12, 22, 52] {
        let (baseline, _) = raw.cell(size);
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
    let text = "Bold text";
    let characters = text.chars().count() as u32;
    for size in [12, 22, 52] {
        let regular = request(LIBERATION, size);
        let plain = engine.layout(&regular, text);
        assert_eq!(plain.emboldened, 0);
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

            // Bold makes every character one pixel wider; italic keeps the advances.
            let layout = engine.layout(&styled, text);
            let extra = u32::from(bold);
            let expected: Vec<u32> = plain.advances.iter().map(|a| a + extra).collect();
            assert_eq!(layout.advances, expected, "bold {bold}, italic {italic}");
            assert_eq!(layout.emboldened, extra);
            assert_eq!(layout.average_width, plain.average_width + extra);
            assert_eq!((layout.height, layout.ascent), (plain.height, plain.ascent));
            let width = engine.measure(&styled, text).0;
            assert_eq!(width, layout.width());
            assert!(width >= engine.measure(&regular, text).0 + extra * characters);
            // The style is drawn, not only measured.
            assert_ne!(
                engine.rasterize(&styled, text),
                engine.rasterize(&regular, text)
            );
        }
    }
}

#[test]
fn synthesised_styles_change_the_ink() {
    let mut engine = engine();
    let regular = request(LIBERATION, 40);
    let ink = |mask: &vkxp_text::TextMask| -> u64 {
        mask.coverage.iter().map(|&value| u64::from(value)).sum()
    };
    let plain = engine.rasterize(&regular, "l");
    let bold = engine.rasterize(
        &TextRequest {
            bold: true,
            ..regular.clone()
        },
        "l",
    );
    assert!(ink(&bold) > ink(&plain));

    // A slanted stem reaches right of its advance at the top: an overhang the upright
    // letter does not have.
    let italic = TextRequest {
        italic: true,
        ..regular.clone()
    };
    assert_eq!(engine.layout(&regular, "l").right_overhang, 0);
    let layout = engine.layout(&italic, "l");
    assert!(layout.right_overhang > 0);
    let mask = engine.rasterize(&italic, "l");
    assert_eq!(mask.width, layout.width() + layout.right_overhang);
    assert!(max_coverage(&mask, layout.width()..mask.width) > 0);
}

#[test]
fn condensed_run_has_the_requested_advances() {
    let mut engine = engine();
    let request = request(LIBERATION, 40);
    let text = "HHH";
    let natural = engine.layout(&request, text);
    let advance = natural.advances[0];
    assert_eq!(natural.advances, [advance; 3]);
    assert_eq!((natural.left_overhang, natural.right_overhang), (0, 0));

    // Each glyph at 60 percent of its width, the characters 60 percent apart.
    let narrow = advance * 3 / 5;
    let mask = engine.rasterize_run(&request, text, &[narrow; 3], Some((3, 5)));
    assert_eq!((mask.width, mask.height), (3 * narrow, natural.height));
    assert_eq!(mask.left_overhang, 0);
    let cell = |copy: u32| -> Vec<u8> {
        (0..mask.height)
            .flat_map(|y| {
                let start = (y * mask.width + copy * narrow) as usize;
                mask.coverage[start..start + narrow as usize].to_vec()
            })
            .collect()
    };
    assert!(cell(0).iter().any(|&value| value != 0));
    assert_eq!(cell(0), cell(1));
    assert_eq!(cell(0), cell(2));

    // The condensed letter is narrower than the natural one.
    let columns = |mask: &vkxp_text::TextMask, width: u32| {
        (0..width)
            .filter(|&x| max_coverage(mask, x..x + 1) > 0)
            .count()
    };
    let one = engine.rasterize(&request, "H");
    assert!(columns(&mask, narrow) < columns(&one, advance));

    // Natural advances without condensing are the natural mask.
    assert_eq!(
        engine.rasterize_run(&request, text, &natural.advances, None),
        engine.rasterize(&request, text)
    );
    // Wider advances move the glyphs apart without changing them.
    let wide = engine.rasterize_run(&request, text, &[advance + 7; 3], None);
    assert_eq!(wide.width, 3 * (advance + 7));
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
        let expected: Vec<u32> = text
            .chars()
            .map(|ch| raw.advance(ch, pixels_per_em))
            .collect();
        let layout = engine.layout(&request(LIBERATION, size), text);
        assert_eq!(layout.advances, expected);
        assert_eq!(layout.left_overhang, 0);
        assert_eq!(
            engine.measure(&request(LIBERATION, size), text).0,
            expected.iter().sum::<u32>()
        );
    }
}

#[test]
fn left_overhang() {
    // The hook of `j` starts left of the pen.
    let raw = RawFont::load(LIBERATION_FILE);
    let mut engine = engine();
    let request = request(LIBERATION, 52);
    let pixels_per_em = raw.pixels_per_em(52);
    let advances = raw.advance('j', pixels_per_em) + raw.advance('H', pixels_per_em);
    let layout = engine.layout(&request, "jH");
    assert!(layout.left_overhang > 0);
    assert_eq!(
        engine.measure(&request, "jH").0,
        advances + layout.left_overhang
    );
    // Only the first glyph counts, and its ink is in the mask.
    assert_eq!(engine.measure(&request, "Hj").0, advances);
    let mask = engine.rasterize(&request, "jH");
    assert_eq!(mask.left_overhang, layout.left_overhang);
    assert!(max_coverage(&mask, 0..mask.left_overhang) > 0);
}

#[test]
fn empty_string() {
    let mut engine = engine();
    assert_eq!(engine.measure(&request(LIBERATION, 22), ""), (0, 0));
    assert!(
        engine
            .layout(&request(LIBERATION, 22), "")
            .advances
            .is_empty()
    );
    let mask = engine.rasterize(&request(LIBERATION, 22), "");
    assert_eq!((mask.width, mask.coverage.len()), (0, 0));
}

#[test]
fn size_zero() {
    let mut engine = engine();
    assert_eq!(engine.measure(&request(LIBERATION, 0), "Text"), (0, 0));
    let mask = engine.rasterize(&request(LIBERATION, 0), "Text");
    assert_eq!((mask.width, mask.height, mask.coverage.len()), (0, 0, 0));
}

#[test]
fn control_characters_measure_as_glyphs_and_do_not_start_a_line() {
    let raw = RawFont::load(LIBERATION_FILE);
    let mut engine = engine();
    let request = request(LIBERATION, 22);
    let pixels_per_em = raw.pixels_per_em(22);
    let (a, b) = (
        raw.advance('A', pixels_per_em),
        raw.advance('B', pixels_per_em),
    );
    let height = engine.measure(&request, "AB").1;
    // The font has no glyph for these, so each is as wide as its missing glyph.
    for control in ['\n', '\t', '\u{1}'] {
        assert!(!raw.has(control));
        let missing = raw.advance(control, pixels_per_em);
        assert!(missing > 0);
        let text = format!("A{control}B");
        assert_eq!(engine.layout(&request, &text).advances, [a, missing, b]);
        assert_eq!(engine.measure(&request, &text), (a + missing + b, height));
        assert_eq!(engine.rasterize(&request, &text).height, height);
    }
}
