use vkxp_pixel::Rgba;

use super::*;
use crate::bitmap::testing::*;
use crate::test_support::CellRasterizer;
use crate::{Color, FontDefaults};

/// The samples measured on the original runtime, shared with the text engine's tests.
const SAMPLES: &str = include_str!("../../../vkxp-text/tests/data/text-samples.txt");

/// The sample lines of one kind as numbers; fields that are not numbers are left out.
fn samples(kind: &str) -> Vec<(Vec<String>, Vec<u32>)> {
    let lines: Vec<_> = SAMPLES
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|fields| fields.first() == Some(&kind))
        .map(|fields| {
            let words = fields.iter().map(|field| (*field).to_owned()).collect();
            let numbers = fields
                .iter()
                .filter_map(|field| field.parse().ok())
                .collect();
            (words, numbers)
        })
        .collect();
    assert!(!lines.is_empty(), "no {kind} samples");
    lines
}

/// Defaults whose font the stand-in draws with cells of 5 by 10 pixels.
fn small() -> FontDefaults {
    let mut defaults = defaults();
    defaults.size = 10;
    defaults
}

/// Defaults whose font the stand-in draws with cells of 11 by 22 pixels.
fn normal() -> FontDefaults {
    let mut defaults = defaults();
    defaults.size = 22;
    defaults
}

/// Ten characters of natural advance 11 at the font of [`normal`].
const TEN: &str = "ABCDEFGHIJ";

/// The stand-in with overhangs: the first glyph reaches `left` pixels left of the pen
/// and the last `right` pixels right of its advance, with coverage 40 there. Text set
/// with other advances than the natural ones has no overhang.
struct Overhanging {
    left: u32,
    right: u32,
}

impl TextRasterizer for Overhanging {
    fn metrics(&self, font: &Font, text: &str) -> TextMetrics {
        TextMetrics {
            left_overhang: self.left,
            right_overhang: self.right,
            ..CellRasterizer.metrics(font, text)
        }
    }

    fn rasterize(
        &self,
        font: &Font,
        text: &str,
        advances: &[u32],
        condense: Option<(u32, u32)>,
    ) -> Mask {
        let inner = CellRasterizer.rasterize(font, text, advances, condense);
        if condense.is_some() {
            return inner;
        }
        let width = self.left + inner.width() + self.right;
        let mut coverage = Vec::new();
        for y in 0..inner.height() {
            coverage.extend(std::iter::repeat_n(40, self.left as usize));
            coverage.extend((0..inner.width()).map(|x| inner.get(x, y)));
            coverage.extend(std::iter::repeat_n(40, self.right as usize));
        }
        Mask::new(width, inner.height(), coverage)
    }
}

/// One character whose cell is a single pixel of the given coverage.
struct Dot(u8);

impl TextRasterizer for Dot {
    fn metrics(&self, _font: &Font, text: &str) -> TextMetrics {
        TextMetrics {
            height: 1,
            advances: vec![1; text.chars().count()],
            average_width: 1,
            ..TextMetrics::default()
        }
    }

    fn rasterize(&self, _: &Font, _: &str, advances: &[u32], _: Option<(u32, u32)>) -> Mask {
        Mask::new(advances.len() as u32, 1, vec![self.0; advances.len()])
    }
}

/// What `draw_text` must leave in a bitmap that held `before` when the text cell is
/// `mask`, `offset` pixels in from the rectangle's left edge: the model of the placement
/// and blending rules, written with signed arithmetic and no clipping shortcuts.
fn model(
    before: &[u8],
    (width, height): (u32, u32),
    rect: Rect,
    mask: &Mask,
    offset: u32,
    color: Rgba,
) -> Vec<u8> {
    let mut after = before.to_vec();
    let left = rect.x + offset as i32;
    let top = rect.y + ((rect.height - mask.height() as i32) / 2).max(0);
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let in_rect = (rect.x..rect.x + rect.width).contains(&x)
                && (rect.y..rect.y + rect.height).contains(&y);
            let in_text = (left..left + mask.width() as i32).contains(&x)
                && (top..top + mask.height() as i32).contains(&y);
            if !(in_rect && in_text) {
                continue;
            }
            let coverage = mask.get((x - left) as u32, (y - top) as u32);
            let source = [
                color[0],
                color[1],
                color[2],
                effective_alpha(coverage, color[3]),
            ];
            let offset = ((y as u32 * width + x as u32) * 4) as usize;
            let target: Rgba = before[offset..offset + 4].try_into().unwrap();
            after[offset..offset + 4].copy_from_slice(&blend_normal(target, source, 255));
        }
    }
    after
}

/// The columns of row `y` that are not transparent.
fn drawn_columns(rgba: &[u8], width: u32, y: u32) -> Vec<u32> {
    (0..width)
        .filter(|&x| at(rgba, width, x, y)[3] != 0)
        .collect()
}

/// The rows of column `x` that are not transparent.
fn drawn_rows(rgba: &[u8], width: u32, height: u32, x: u32) -> Vec<u32> {
    (0..height)
        .filter(|&y| at(rgba, width, x, y)[3] != 0)
        .collect()
}

fn metrics(advances: &[u32], height: u32, average_width: u32, bold: u32) -> TextMetrics {
    TextMetrics {
        height,
        advances: advances.to_vec(),
        average_width,
        emboldened: bold,
        ..TextMetrics::default()
    }
}

// Requirement: Text drawing

#[test]
fn align_offset_rules() {
    assert_eq!(align_offset(10, 30, Align::Left), 0);
    assert_eq!(align_offset(10, 30, Align::Center), 10);
    assert_eq!(align_offset(10, 30, Align::Right), 20);
    assert_eq!(align_offset(10, 31, Align::Center), 10);
    assert_eq!(align_offset(10, 10, Align::Right), 0);
}

#[test]
fn alignment_places_the_left_edge() {
    let rasterizer = CellRasterizer;
    for (align, left) in [(Align::Left, 3), (Align::Center, 13), (Align::Right, 23)] {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(40, 14, &small()).unwrap();
        // "AB" is 10 wide; the rectangle leaves 20 pixels free.
        bitmaps
            .draw_text(id, Rect::new(3, 2, 30, 10), "AB", align, &rasterizer)
            .unwrap();
        let rgba = bitmaps.rgba(id).unwrap();
        for y in 2..12 {
            let expected: Vec<u32> = (left..left + 10).collect();
            assert_eq!(drawn_columns(rgba, 40, y), expected, "{align:?}, row {y}");
        }
        assert!(drawn_columns(rgba, 40, 1).is_empty());
        assert!(drawn_columns(rgba, 40, 12).is_empty());
    }
}

#[test]
fn alignment_is_by_the_drawn_width() {
    // "AB" measures 10 + 2 and is drawn 3 wider: 15 pixels in a rectangle of 30.
    let rasterizer = Overhanging { left: 2, right: 3 };
    for (align, left) in [(Align::Left, 3), (Align::Center, 10), (Align::Right, 18)] {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(40, 10, &small()).unwrap();
        bitmaps
            .draw_text(id, Rect::new(3, 0, 30, 10), "AB", align, &rasterizer)
            .unwrap();
        let rgba = bitmaps.rgba(id).unwrap();
        let expected: Vec<u32> = (left..left + 15).collect();
        assert_eq!(drawn_columns(rgba, 40, 4), expected, "{align:?}");
        // The first glyph's overhang is drawn, and the pen starts right of it.
        assert_eq!(at(rgba, 40, left, 4)[3], 40);
        assert_eq!(at(rgba, 40, left + 1, 4)[3], 40);
        assert_eq!(
            at(rgba, 40, left + 2, 4)[3],
            CellRasterizer::coverage('A', 0, 4)
        );
        assert_eq!(at(rgba, 40, left + 14, 4)[3], 40);
    }
}

#[test]
fn right_overhang_is_cut_at_the_rectangle() {
    // A rectangle as wide as the text measures is too narrow for the drawn width, so
    // the text is squeezed; one as wide as the drawn width holds it unsqueezed.
    let rasterizer = Overhanging { left: 0, right: 3 };
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(40, 10, &small()).unwrap();
    let size = bitmaps.text_size(id, "ABCD", &rasterizer).unwrap();
    assert_eq!(size.width, 20);
    bitmaps
        .draw_text(
            id,
            Rect::new(0, 0, 20, 10),
            "ABCD",
            Align::Left,
            &rasterizer,
        )
        .unwrap();
    // Step 9 of a size 10 font with average width 5: every advance 4.
    assert_eq!(drawn_columns(bitmaps.rgba(id).unwrap(), 40, 4).len(), 16);

    let id = bitmaps.create(40, 10, &small()).unwrap();
    bitmaps
        .draw_text(
            id,
            Rect::new(0, 0, 23, 10),
            "ABCD",
            Align::Left,
            &rasterizer,
        )
        .unwrap();
    assert_eq!(drawn_columns(bitmaps.rgba(id).unwrap(), 40, 4).len(), 23);
}

#[test]
fn vertical_centring() {
    let rasterizer = CellRasterizer;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(12, 40, &small()).unwrap();
    // Text of height 10 in a rectangle of height 30: rows 10 to 19 of the rectangle,
    // which starts at row 4.
    bitmaps
        .draw_text(id, Rect::new(0, 4, 12, 30), "A", Align::Left, &rasterizer)
        .unwrap();
    let rows: Vec<u32> = (14..24).collect();
    assert_eq!(drawn_rows(bitmaps.rgba(id).unwrap(), 12, 40, 0), rows);

    // An odd free height is halved rounding down.
    let odd = bitmaps.create(12, 40, &small()).unwrap();
    bitmaps
        .draw_text(odd, Rect::new(0, 0, 12, 13), "A", Align::Left, &rasterizer)
        .unwrap();
    let rows: Vec<u32> = (1..11).collect();
    assert_eq!(drawn_rows(bitmaps.rgba(odd).unwrap(), 12, 40, 0), rows);
}

#[test]
fn rectangle_lower_than_the_text_shows_its_top() {
    let rasterizer = CellRasterizer;
    let mut defaults = defaults();
    defaults.size = 21;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(12, 40, &defaults).unwrap();
    // Text of height 21 in a rectangle of height 10.
    let rect = Rect::new(1, 5, 10, 10);
    bitmaps
        .draw_text(id, rect, "A", Align::Left, &rasterizer)
        .unwrap();
    let font = bitmaps.font(id).unwrap().clone();
    let mask = CellRasterizer::natural(&font, "A");
    assert_eq!(mask.height(), 21);
    let rgba = bitmaps.rgba(id).unwrap();
    assert_eq!(drawn_rows(rgba, 12, 40, 1), (5..15).collect::<Vec<u32>>());
    for row in 0..10 {
        assert_eq!(at(rgba, 12, 1, 5 + row)[3], mask.get(0, row), "row {row}");
    }
}

#[test]
fn squeezing_sets_the_text_at_a_step() {
    let rasterizer = CellRasterizer;
    // Ten characters of advance 11 in a rectangle of 100: nine pixels each at the 90
    // percent step, 90 wide, placed by that width.
    for (align, offset) in [(Align::Left, 0), (Align::Center, 5), (Align::Right, 10)] {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(120, 24, &normal()).unwrap();
        let font = bitmaps.font(id).unwrap().clone();
        assert_eq!(rasterizer.metrics(&font, TEN).advances, [11; 10]);
        bitmaps
            .draw_text(id, Rect::new(10, 1, 100, 22), TEN, align, &rasterizer)
            .unwrap();
        let rgba = bitmaps.rgba(id).unwrap();
        let expected: Vec<u32> = (10 + offset..100 + offset).collect();
        assert_eq!(drawn_columns(rgba, 120, 5), expected, "{align:?}");
        let mask = rasterizer.rasterize(&font, TEN, &[9; 10], None);
        for i in 0..90 {
            // The font colour is opaque, so on a transparent bitmap alpha is coverage.
            assert_eq!(
                at(rgba, 120, 10 + offset + i, 5)[3],
                mask.get(i, 4),
                "column {i}"
            );
        }
    }
}

#[test]
fn squeezing_takes_the_first_step_that_fits() {
    let rasterizer = CellRasterizer;
    // Size 22, average width 11: the steps are 90, 80, 70 and 60 pixels wide.
    for (width, drawn) in [
        (110, 110),
        (109, 90),
        (90, 90),
        (89, 80),
        (79, 70),
        (69, 60),
    ] {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(120, 22, &normal()).unwrap();
        bitmaps
            .draw_text(
                id,
                Rect::new(0, 0, width, 22),
                TEN,
                Align::Left,
                &rasterizer,
            )
            .unwrap();
        let columns = drawn_columns(bitmaps.rgba(id).unwrap(), 120, 5);
        assert_eq!(columns, (0..drawn).collect::<Vec<u32>>(), "width {width}");
    }
}

#[test]
fn squeeze_limit_clips_what_does_not_fit() {
    let rasterizer = CellRasterizer;
    for align in [Align::Left, Align::Center, Align::Right] {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(120, 24, &normal()).unwrap();
        let font = bitmaps.font(id).unwrap().clone();
        bitmaps
            .draw_text(id, Rect::new(10, 1, 40, 22), TEN, align, &rasterizer)
            .unwrap();
        let rgba = bitmaps.rgba(id).unwrap();
        // Set 60 wide at the last step from the rectangle's left edge; only its first 40
        // columns appear.
        let expected: Vec<u32> = (10..50).collect();
        assert_eq!(drawn_columns(rgba, 120, 5), expected, "{align:?}");
        let mask = rasterizer.rasterize(&font, TEN, &[6; 10], None);
        for i in 0..40 {
            assert_eq!(at(rgba, 120, 10 + i, 5)[3], mask.get(i, 4), "column {i}");
        }
    }
}

#[test]
fn zero_coverage_is_blended_like_the_rest_of_the_cell() {
    let rasterizer = CellRasterizer;
    let mut bitmaps = Bitmaps::new();
    // Rows 0 to 4 transparent black, rows 5 to 9 a translucent colour.
    let mut before = vec![0; 15 * 10 * 4];
    for pixel in before.chunks_mut(4).skip(15 * 5) {
        pixel.copy_from_slice(&[9, 8, 7, 100]);
    }
    let id = bitmaps
        .create_from_rgba(15, 10, before.clone(), &small())
        .unwrap();
    bitmaps.font_mut(id).unwrap().color = color([200, 100, 50, 255]);
    // The stand-in gives a space no coverage at all.
    bitmaps
        .draw_text(id, Rect::new(0, 0, 15, 10), " A ", Align::Left, &rasterizer)
        .unwrap();
    let rgba = bitmaps.rgba(id).unwrap();
    for y in 0..10 {
        for x in 0..15 {
            let pixel = at(rgba, 15, x, y);
            if (5..10).contains(&x) {
                assert_ne!(pixel, at(&before, 15, x, y), "({x}, {y})");
            } else if y < 5 {
                // A transparent pixel takes the font's colour and stays transparent.
                assert_eq!(pixel, [200, 100, 50, 0], "({x}, {y})");
            } else {
                assert_eq!(pixel, [9, 8, 7, 100], "({x}, {y})");
            }
        }
    }
}

#[test]
fn zero_colour_alpha_leaves_the_destination() {
    let rasterizer = CellRasterizer;
    let mut bitmaps = Bitmaps::new();
    let before = vec![0; 15 * 10 * 4];
    let id = bitmaps
        .create_from_rgba(15, 10, before.clone(), &small())
        .unwrap();
    bitmaps.font_mut(id).unwrap().color = Color::new(255.0, 255.0, 255.0, 0.0);
    bitmaps
        .draw_text(id, Rect::new(0, 0, 15, 10), "ABC", Align::Left, &rasterizer)
        .unwrap();
    assert_eq!(bitmaps.rgba(id).unwrap(), before);
}

#[test]
fn coverage_scales_the_colour_alpha_and_blends_source_over() {
    let rasterizer = CellRasterizer;
    let color: Rgba = [200, 100, 50, 128];
    // Rectangle, text, alignment, and the advances and offset the rules give.
    let cases = [
        (Rect::new(2, 1, 30, 14), "ABC", Align::Center, 5, 7),
        // 20 wide in a rectangle of 12: step 7 of size 10, three pixels a character.
        (Rect::new(-3, -2, 12, 9), "ABCD", Align::Right, 3, 0),
        (Rect::new(20, 4, 100, 12), "AB", Align::Left, 5, 0),
    ];
    for (rect, text, align, advance, offset) in cases {
        let mut bitmaps = Bitmaps::new();
        let before = pattern(36, 18, 8);
        let id = bitmaps
            .create_from_rgba(36, 18, before.clone(), &small())
            .unwrap();
        bitmaps.font_mut(id).unwrap().color = crate::bitmap::testing::color(color);
        let font = bitmaps.font(id).unwrap().clone();
        let advances = vec![advance; text.chars().count()];
        let mask = rasterizer.rasterize(&font, text, &advances, None);
        bitmaps
            .draw_text(id, rect, text, align, &rasterizer)
            .unwrap();
        let expected = model(&before, (36, 18), rect, &mask, offset, color);
        assert!(bitmaps.rgba(id).unwrap() == expected, "{rect:?}");
        assert!(expected != before, "{rect:?} draws something");
    }
}

/// `C coverage colour_alpha | destination | result`: pixels of the text cell.
#[test]
fn measured_colour_and_alpha() {
    for (words, numbers) in samples("C") {
        let [coverage, alpha, dr, dg, db, da, r, g, b, a] = numbers[..] else {
            panic!("{words:?}");
        };
        let mut bitmaps = Bitmaps::new();
        let before = [dr, dg, db, da].map(|value| value as u8).to_vec();
        let id = bitmaps.create_from_rgba(1, 1, before, &small()).unwrap();
        bitmaps.font_mut(id).unwrap().color = color([200, 100, 50, alpha as u8]);
        bitmaps
            .draw_text(
                id,
                Rect::new(0, 0, 1, 1),
                "A",
                Align::Left,
                &Dot(coverage as u8),
            )
            .unwrap();
        let expected = [r, g, b, a].map(|value| value as u8);
        assert_eq!(bitmaps.rgba(id).unwrap(), expected, "{words:?}");
    }
}

#[test]
fn text_uses_the_bitmaps_own_font() {
    let rasterizer = CellRasterizer;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(40, 40, &small()).unwrap();
    bitmaps.font_mut(id).unwrap().size = 20;
    bitmaps.font_mut(id).unwrap().color = Color::rgb(1.0, 2.0, 3.0);
    bitmaps
        .draw_text(id, Rect::new(0, 0, 40, 20), "A", Align::Left, &rasterizer)
        .unwrap();
    let rgba = bitmaps.rgba(id).unwrap();
    // A size of 20 gives the stand-in cells of 10 by 20.
    assert_eq!(drawn_columns(rgba, 40, 0), (0..10).collect::<Vec<u32>>());
    assert_eq!(drawn_rows(rgba, 40, 40, 0), (0..20).collect::<Vec<u32>>());
    assert_eq!(at(rgba, 40, 0, 0)[..3], [1, 2, 3]);
}

#[test]
fn empty_rectangles_and_strings_draw_nothing() {
    let rasterizer = CellRasterizer;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(20, 20, &small()).unwrap();
    for rect in [
        Rect::new(0, 0, 0, 10),
        Rect::new(0, 0, 10, 0),
        Rect::new(5, 5, -5, -5),
        Rect::new(20, 0, 10, 10),
        Rect::new(0, -10, 10, 10),
    ] {
        bitmaps
            .draw_text(id, rect, "AB", Align::Left, &rasterizer)
            .unwrap();
    }
    bitmaps
        .draw_text(id, Rect::new(0, 0, 20, 20), "", Align::Center, &rasterizer)
        .unwrap();
    assert_eq!(bitmaps.rgba(id).unwrap(), [0; 1600]);
}

#[test]
fn text_on_a_disposed_bitmap_fails() {
    let rasterizer = CellRasterizer;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(4, 4, &small()).unwrap();
    bitmaps.dispose(id);
    assert_eq!(
        bitmaps.draw_text(id, Rect::new(0, 0, 4, 4), "A", Align::Left, &rasterizer),
        Err(BitmapError::Disposed)
    );
    assert_eq!(
        bitmaps.text_size(id, "A", &rasterizer),
        Err(BitmapError::Disposed)
    );
}

// Requirement: Text squeezing

#[test]
fn squeeze_rule() {
    // Size 22, text height 21, average width 15, as measured: k = 14, 12, 11, 9.
    let wide = metrics(&[15; 5], 21, 15, 0);
    let widths: Vec<u32> = SQUEEZE_STEPS
        .iter()
        .map(|&step| total(&wide.squeezed(22, step).0))
        .collect();
    assert_eq!(widths, [70, 60, 55, 45]);
    assert_eq!(wide.squeezed(22, 9).1, (14, 15));
    // Each advance is rounded up on its own.
    let mixed = metrics(&[11, 4, 15], 21, 15, 0);
    assert_eq!(mixed.squeezed(22, 9).0, [11, 4, 14]);
    // The pixel of a synthesised bold is not scaled: 16 = 15 + 1, k = 22 * 9 * 16 / 210.
    let bold = metrics(&[16, 12], 21, 16, 1);
    assert_eq!(bold.squeezed(22, 9), (vec![16, 12], (15, 15)));
    assert_eq!(bold.squeezed(22, 6), (vec![11, 9], (10, 15)));
    // Nothing to divide by does not divide by zero.
    assert_eq!(metrics(&[3], 0, 0, 0).squeezed(0, 9).0, [0]);
}

/// `Q font size string natural_width | widths`, with the text height and the average
/// width of the `M` line of the font and size. Each string is one character twice.
///
/// The widths listed are drawn widths: the first is the natural one, which is the
/// measured width plus the last glyph's right overhang, and the same overhang is in the
/// others. Here the advances are checked; the widths the font engine draws with are
/// checked against the same samples where the engine is at hand.
#[test]
fn measured_squeeze_levels() {
    let number = |word: &String| word.parse::<u32>().expect("a number");
    let sizes = samples("M");
    for (words, _) in samples("Q") {
        let (size, natural) = (number(&words[2]), number(&words[4]));
        let expected: Vec<u32> = words[6..].iter().map(number).collect();
        let (line, _) = sizes
            .iter()
            .find(|(line, _)| line[1] == words[1] && line[2] == words[2])
            .expect("the size sample of the font");
        let height = number(&line[4]) + number(&line[5]);
        let average = number(&line[6]);
        let text = metrics(&[natural / 2; 2], height, average, 0);
        let overhang = expected[0] - natural;
        // The natural width, then every width a step gives that is narrower than the
        // one before.
        let mut levels = vec![natural + overhang];
        for step in SQUEEZE_STEPS {
            let width = total(&text.squeezed(size, step).0) + overhang;
            if width < *levels.last().unwrap() {
                levels.push(width);
            }
        }
        assert_eq!(levels, expected, "{words:?}");
    }
}

// Requirement: Text measurement

#[test]
fn measurement_matches_drawing() {
    let rasterizer = CellRasterizer;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(40, 20, &small()).unwrap();
    let size = bitmaps.text_size(id, "Hello", &rasterizer).unwrap();
    assert_eq!(size, Rect::new(0, 0, 25, 10));
    // Measuring draws nothing.
    assert_eq!(bitmaps.rgba(id).unwrap(), [0; 3200]);

    let rect = Rect::new(7, 4, size.width, size.height);
    bitmaps
        .draw_text(id, rect, "Hello", Align::Left, &rasterizer)
        .unwrap();
    let font = bitmaps.font(id).unwrap().clone();
    let mask = CellRasterizer::natural(&font, "Hello");
    let rgba = bitmaps.rgba(id).unwrap();
    for y in 0..20 {
        for x in 0..40 {
            // Unsqueezed, the mask appears pixel for pixel inside the rectangle, and no
            // glyph pixel falls outside it.
            let inside = (7..32).contains(&x) && (4..14).contains(&y);
            let expected = if inside { mask.get(x - 7, y - 4) } else { 0 };
            assert_eq!(at(rgba, 40, x, y)[3], expected, "({x}, {y})");
        }
    }
}

#[test]
fn measurement_includes_the_left_overhang_only() {
    let rasterizer = Overhanging { left: 2, right: 3 };
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(4, 4, &small()).unwrap();
    assert_eq!(
        bitmaps.text_size(id, "Hello", &rasterizer),
        Ok(Rect::new(0, 0, 27, 10))
    );
}

#[test]
fn empty_string_measures_nothing() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(4, 4, &small()).unwrap();
    for size in [10, 24] {
        bitmaps.font_mut(id).unwrap().size = size;
        assert_eq!(
            bitmaps.text_size(id, "", &CellRasterizer),
            Ok(Rect::new(0, 0, 0, 0))
        );
        // Also with a rasteriser that reports overhangs for it.
        assert_eq!(
            bitmaps.text_size(id, "", &Overhanging { left: 2, right: 3 }),
            Ok(Rect::new(0, 0, 0, 0))
        );
    }
}
