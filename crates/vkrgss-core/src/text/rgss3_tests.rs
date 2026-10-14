//! The text rules of RGSS3, against the numbers measured on the original runtime of VX Ace
//! (`tests/data/rgss3-text-samples.txt`).

use std::cell::RefCell;

use vkrgss_pixel::Rgba;

use super::*;
use crate::bitmap::testing::*;
use crate::test_support::CellRasterizer;
use crate::{Color, FontDefaults, MAX_BITMAP_PIXELS};

const SAMPLES: &str = include_str!("../../tests/data/rgss3-text-samples.txt");

/// The sample lines that begin with `kind`, as their fields, with the number of the line
/// after each.
fn samples(kind: &str) -> Vec<(Vec<&'static str>, usize)> {
    let lines: Vec<_> = SAMPLES
        .lines()
        .enumerate()
        .map(|(number, line)| (line.split_whitespace().collect::<Vec<_>>(), number + 1))
        .filter(|(fields, _)| fields.first() == Some(&kind))
        .collect();
    assert!(!lines.is_empty(), "no {kind} samples");
    lines
}

fn number<T: std::str::FromStr>(field: &str) -> T {
    field.parse().ok().expect("a number")
}

/// A pixel or a colour written `r,g,b,a`.
fn quad(field: &str) -> Rgba {
    let channels: Vec<u8> = field.split(',').map(number).collect();
    channels.try_into().expect("four channels")
}

/// What the edge probes fill their bitmaps with, and the colours they draw in.
const MARK: Rgba = [7, 8, 9, 0];
const PROBE_COLOR: Rgba = [250, 180, 60, 255];
const PROBE_OUT_COLOR: Rgba = [1, 2, 3, 255];

/// A text of one cell whatever the string: `size` pixels, with the coverage `coverage` in
/// the pixels `ink` (left, top, right, bottom, the last two exclusive) and none elsewhere.
/// Set with other advances the cell is as wide as their sum and the ink is cut by it.
#[derive(Debug, Clone, Copy)]
struct Block {
    size: (u32, u32),
    ink: (u32, u32, u32, u32),
    coverage: u8,
}

/// The text of the edge probes: 9 wide and 24 high, with its ink inside.
const PROBE_TEXT: Block = Block {
    size: (9, 24),
    ink: (1, 5, 8, 19),
    coverage: 255,
};

impl TextRasterizer for Block {
    fn metrics(&self, _font: &Font, text: &str) -> TextMetrics {
        TextMetrics {
            height: self.size.1,
            advances: if text.is_empty() {
                Vec::new()
            } else {
                vec![self.size.0]
            },
            average_width: self.size.0.max(1),
            ..TextMetrics::default()
        }
    }

    fn rasterize(&self, _: &Font, _: &str, advances: &[u32], _: Option<(u32, u32)>) -> Mask {
        let width = total(advances);
        let (left, top, right, bottom) = self.ink;
        let mut coverage = Vec::new();
        for y in 0..self.size.1 {
            for x in 0..width {
                let inked = (left..right).contains(&x) && (top..bottom).contains(&y);
                coverage.push(if inked { self.coverage } else { 0 });
            }
        }
        Mask::new(width, self.size.1, coverage)
    }
}

/// A text with the given advances, one for each character, that remembers how it was asked
/// to be set: the condensing of every call of `rasterize`. The cell is fully covered.
struct Recorder {
    advances: Vec<u32>,
    height: u32,
    average_width: u32,
    asked: RefCell<Vec<Option<(u32, u32)>>>,
}

impl Recorder {
    fn new(advances: &[u32], height: u32, average_width: u32) -> Recorder {
        Recorder {
            advances: advances.to_vec(),
            height,
            average_width,
            asked: RefCell::new(Vec::new()),
        }
    }

    /// The average character widths asked for since the last call, `None` for a text set
    /// at its natural width.
    fn take(&self) -> Vec<Option<u32>> {
        let asked = self.asked.take();
        asked
            .into_iter()
            .map(|condense| condense.map(|(width, _)| width))
            .collect()
    }
}

impl TextRasterizer for Recorder {
    fn metrics(&self, _font: &Font, text: &str) -> TextMetrics {
        TextMetrics {
            height: self.height,
            advances: self.advances[..text.chars().count()].to_vec(),
            average_width: self.average_width,
            ..TextMetrics::default()
        }
    }

    fn rasterize(&self, _: &Font, _: &str, advances: &[u32], condense: Option<(u32, u32)>) -> Mask {
        self.asked.borrow_mut().push(condense);
        let width = total(advances);
        Mask::new(
            width,
            self.height,
            vec![255; (width * self.height) as usize],
        )
    }
}

/// The defaults of a font of RGSS3 in the colours of the edge probes.
fn probe_font(outline: bool) -> FontDefaults {
    FontDefaults {
        outline,
        color: color(PROBE_COLOR),
        out_color: color(PROBE_OUT_COLOR),
        ..FontDefaults::rgss3()
    }
}

/// A bitmap of the given size filled with `fill`.
fn filled(
    bitmaps: &mut Bitmaps,
    (width, height): (u32, u32),
    fill: Rgba,
    defaults: &FontDefaults,
) -> BitmapId {
    let rgba = fill.repeat((width * height) as usize);
    bitmaps
        .create_from_rgba(width, height, rgba, defaults)
        .unwrap()
}

/// The box of the pixels that are no longer `fill`, as (left, top, right, bottom) with the
/// last two exclusive, and how many such pixels there are.
fn changed(rgba: &[u8], width: u32, fill: Rgba) -> Option<((u32, u32, u32, u32), usize)> {
    let mut found: Option<(u32, u32, u32, u32)> = None;
    let mut count = 0;
    for (index, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
        if *pixel == fill {
            continue;
        }
        count += 1;
        let (x, y) = (index as u32 % width, index as u32 / width);
        found = Some(match found {
            None => (x, y, x + 1, y + 1),
            Some((left, top, right, bottom)) => {
                (left.min(x), top.min(y), right.max(x + 1), bottom.max(y + 1))
            }
        });
    }
    found.map(|area| (area, count))
}

/// The box [`changed`] finds after the probe's text is drawn by the rules of RGSS3 into
/// `rect` of a bitmap of `size` filled with [`MARK`].
fn probe_box(
    size: (u32, u32),
    rect: Rect,
    align: Align,
    outline: bool,
) -> Option<((u32, u32, u32, u32), usize)> {
    let mut bitmaps = Bitmaps::new();
    let id = filled(&mut bitmaps, size, MARK, &probe_font(outline));
    bitmaps
        .draw_text_as(id, rect, "H", align, TextRules::Rgss3, &PROBE_TEXT)
        .unwrap();
    changed(bitmaps.rgba(id).unwrap(), size.0, MARK)
}

/// The left and top of the box [`probe_box`] finds: where the work rectangle was put. With
/// the outline the first passes reach the pixel left of and above the first pixel blended,
/// which is one in from that place, so this is the place itself in both cases.
fn probe_place(size: (u32, u32), rect: Rect, align: Align, outline: bool) -> (u32, u32) {
    let ((left, top, _, _), _) = probe_box(size, rect, align, outline).expect("something drawn");
    (left, top)
}

fn align(number: u32) -> Align {
    match number {
        1 => Align::Center,
        2 => Align::Right,
        _ => Align::Left,
    }
}

// Requirement: Text rules by runtime

/// Scenario "The same draw by three rules".
#[test]
fn rgss1_and_rgss2_draw_alike_what_fits() {
    let draw = |rules: TextRules| {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps
            .create_from_rgba(40, 20, pattern(40, 20, 3), &defaults())
            .unwrap();
        bitmaps.font_mut(id).unwrap().size = 10;
        let rect = Rect::new(3, 2, 30, 14);
        bitmaps
            .draw_text_as(id, rect, "Hello", Align::Center, rules, &CellRasterizer)
            .unwrap();
        bitmaps.rgba(id).unwrap().to_vec()
    };
    assert_eq!(draw(TextRules::Rgss1), draw(TextRules::Rgss2));
    assert_ne!(draw(TextRules::Rgss1), pattern(40, 20, 3));
}

/// The entry points without a value of the runtime draw what the value of their squeeze
/// draws.
#[test]
fn the_older_entry_points_keep_their_results() {
    let draw = |call: &dyn Fn(&mut Bitmaps, BitmapId, Rect)| {
        let mut bitmaps = Bitmaps::new();
        let mut shadowed = defaults();
        shadowed.shadow = true;
        shadowed.size = 20;
        let id = bitmaps
            .create_from_rgba(60, 30, pattern(60, 30, 5), &shadowed)
            .unwrap();
        // Too narrow for the ten characters, so the squeeze of the runtime shows.
        call(&mut bitmaps, id, Rect::new(2, 2, 52, 24));
        bitmaps.rgba(id).unwrap().to_vec()
    };
    let text = "ABCDEFGHIJ";
    let as_rules = |rules: TextRules| {
        draw(&|bitmaps, id, rect| {
            bitmaps
                .draw_text_as(id, rect, text, Align::Right, rules, &CellRasterizer)
                .unwrap();
        })
    };
    let plain = draw(&|bitmaps, id, rect| {
        bitmaps
            .draw_text(id, rect, text, Align::Right, &CellRasterizer)
            .unwrap();
    });
    let squeezed = draw(&|bitmaps, id, rect| {
        bitmaps
            .draw_text_squeezed(
                id,
                rect,
                text,
                Align::Right,
                Squeeze::Rgss2,
                &CellRasterizer,
            )
            .unwrap();
    });
    assert_eq!(plain, as_rules(TextRules::Rgss1));
    assert_eq!(squeezed, as_rules(TextRules::Rgss2));
    assert_ne!(plain, squeezed);
}

/// Scenario "RGSS3 touches more".
#[test]
fn rgss3_blends_the_rectangle_where_rgss1_blends_the_cell() {
    let rect = Rect::new(10, 10, 200, 60);
    let draw = |rules: TextRules| {
        let mut bitmaps = Bitmaps::new();
        let id = filled(&mut bitmaps, (300, 100), MARK, &probe_font(false));
        bitmaps
            .draw_text_as(id, rect, "H", Align::Left, rules, &PROBE_TEXT)
            .unwrap();
        changed(bitmaps.rgba(id).unwrap(), 300, MARK).unwrap()
    };
    assert_eq!(draw(TextRules::Rgss1), ((10, 28, 19, 52), 9 * 24));
    assert_eq!(draw(TextRules::Rgss3), ((10, 28, 210, 88), 200 * 60));
}

// Requirement: Text placement of RGSS3

/// Scenario "Without outline, measured".
#[test]
fn placement_without_the_outline() {
    let rect = Rect::new(0, 0, 800, 600);
    let lefts = [Align::Left, Align::Center, Align::Right]
        .map(|align| probe_place((800, 600), rect, align, false));
    assert_eq!(lefts, [(0, 288), (395, 288), (791, 288)]);
}

/// Scenario "Outline moves the text up, measured".
#[test]
fn the_outline_moves_the_text_up() {
    let rect = Rect::new(10, 10, 200, 60);
    assert_eq!(probe_place((300, 100), rect, Align::Left, false), (10, 28));
    assert_eq!(probe_place((300, 100), rect, Align::Left, true), (10, 27));
}

/// Scenario "Outline when aligned".
#[test]
fn the_outline_counts_when_the_text_is_aligned() {
    let left = |width: i32, align: Align, outline: bool| {
        probe_place((300, 100), Rect::new(0, 0, width, 60), align, outline).0
    };
    assert_eq!(left(100, Align::Right, false), 91);
    assert_eq!(left(101, Align::Right, false), 92);
    assert_eq!(left(100, Align::Right, true), 89);
    assert_eq!(left(101, Align::Right, true), 90);
    assert_eq!(left(100, Align::Center, false), 45);
    assert_eq!(left(101, Align::Center, false), 46);
    assert_eq!(left(100, Align::Center, true), 45);
    assert_eq!(left(101, Align::Center, true), 45);
}

/// Scenario "Rectangle lower than the text".
#[test]
fn a_rectangle_lower_than_the_text_shows_its_top() {
    let rect = Rect::new(10, 10, 100, 25);
    assert_eq!(probe_place((300, 100), rect, Align::Left, true), (10, 10));
}

/// Scenarios "Text wider than the rectangle" and "One step only".
#[test]
fn a_text_that_stays_too_wide_starts_at_the_left_edge() {
    // Ten characters of 11 pixels at size 22 in a rectangle of half their width: `r` is
    // 0.6, the width asked for trunc(22 * 0.6 * 7 / 16) = 5, and with an average
    // character width of 6 every advance becomes ceil(11 * 5 / 6) = 10. The text is 100
    // wide and the rectangle 55.
    let text = Recorder::new(&[11; 10], 22, 6);
    let mut defaults = FontDefaults::rgss3();
    defaults.size = 22;
    defaults.outline = false;
    let mut boxes = Vec::new();
    for align in [Align::Left, Align::Center, Align::Right] {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(200, 40, &defaults).unwrap();
        let rect = Rect::new(20, 5, 55, 30);
        bitmaps
            .draw_text_as(id, rect, "ABCDEFGHIJ", align, TextRules::Rgss3, &text)
            .unwrap();
        // Set once, at the width asked for.
        assert_eq!(text.take(), [Some(5)]);
        let rgba = bitmaps.rgba(id).unwrap();
        // The text has ink up to the last column of the work rectangle and none beyond.
        assert_eq!(at(rgba, 200, 74, 20), [255, 255, 255, 255]);
        assert_eq!(at(rgba, 200, 75, 20), [0, 0, 0, 0]);
        boxes.push(changed(rgba, 200, [0, 0, 0, 0]).unwrap().0);
    }
    // The text is 22 high, so the work rectangle is at row 5 + (30 - 22) / 2.
    assert_eq!(boxes, [(20, 9, 75, 39); 3]);
}

// Requirement: Work rectangle of RGSS3

/// Scenarios "Measured boxes", "Below and right of the rectangle, measured", "Limits of 640
/// and 480, measured" and "Nothing to draw, measured"; of "Outline of RGSS3" the scenarios
/// "Narrow and low rectangles, measured" and "Outline at the limit of 640, measured".
#[test]
fn measured_boxes_of_the_edge_probes() {
    let lines = samples("edge");
    assert_eq!(lines.len(), 31);
    for (fields, line) in lines {
        let size = (number(fields[1]), number(fields[2]));
        let rect = Rect::new(
            number(fields[3]),
            number(fields[4]),
            number(fields[5]),
            number(fields[6]),
        );
        let found = probe_box(size, rect, align(number(fields[7])), fields[8] == "1");
        assert_eq!(fields[9], "|");
        if fields[10] == "none" {
            assert_eq!(found, None, "line {line}");
            continue;
        }
        let ((left, top, right, bottom), count) = found.unwrap_or_else(|| panic!("line {line}"));
        // Without the outline the changed pixels are a whole rectangle. With it the
        // passes leave pixels of the border out.
        if fields[8] == "0" {
            assert_eq!(count as u32, (right - left) * (bottom - top), "line {line}");
        }
        // A side written "-" was not measured.
        for (field, side) in fields[10..14].iter().zip([left, top, right, bottom]) {
            if *field != "-" {
                assert_eq!(side, number::<u32>(field), "line {line}");
            }
        }
    }
}

/// Scenario "Pixels without ink".
#[test]
fn pixels_without_ink_are_blended_too() {
    let mut bitmaps = Bitmaps::new();
    let mut rgba = MARK.repeat(60 * 40);
    // An opaque pixel inside the work rectangle, where the text has no ink.
    let opaque = [90, 80, 70, 255];
    rgba[(20 * 60 + 40) * 4..][..4].copy_from_slice(&opaque);
    let id = bitmaps
        .create_from_rgba(60, 40, rgba, &probe_font(false))
        .unwrap();
    let rect = Rect::new(5, 5, 50, 30);
    bitmaps
        .draw_text_as(id, rect, "H", Align::Left, TextRules::Rgss3, &PROBE_TEXT)
        .unwrap();
    let rgba = bitmaps.rgba(id).unwrap();
    assert_eq!(at(rgba, 60, 41, 20), [250, 180, 60, 0]);
    assert_eq!(at(rgba, 60, 40, 20), opaque);
    // Outside the work rectangle nothing is touched.
    assert_eq!(at(rgba, 60, 55, 20), MARK);
}

/// Scenario "No state between draws".
#[test]
fn a_large_draw_leaves_nothing_for_the_next() {
    let mut bitmaps = Bitmaps::new();
    let small = |bitmaps: &mut Bitmaps| {
        let id = filled(bitmaps, (300, 100), MARK, &probe_font(true));
        let rect = Rect::new(10, 10, 200, 60);
        bitmaps
            .draw_text_as(id, rect, "H", Align::Center, TextRules::Rgss3, &PROBE_TEXT)
            .unwrap();
        bitmaps.rgba(id).unwrap().to_vec()
    };
    let before = small(&mut bitmaps);
    let large = filled(&mut bitmaps, (800, 600), MARK, &probe_font(true));
    let rect = Rect::new(0, 0, 800, 600);
    bitmaps
        .draw_text_as(
            large,
            rect,
            "H",
            Align::Right,
            TextRules::Rgss3,
            &PROBE_TEXT,
        )
        .unwrap();
    assert_eq!(small(&mut bitmaps), before);
}

/// Scenario "Nothing to draw, measured": the empty string.
#[test]
fn an_empty_string_draws_nothing() {
    for outline in [false, true] {
        let mut bitmaps = Bitmaps::new();
        let id = filled(&mut bitmaps, (300, 100), MARK, &probe_font(outline));
        for rect in [Rect::new(10, 10, 200, 60), Rect::new(0, 0, 300, 100)] {
            bitmaps
                .draw_text_as(id, rect, "", Align::Center, TextRules::Rgss3, &PROBE_TEXT)
                .unwrap();
        }
        assert_eq!(changed(bitmaps.rgba(id).unwrap(), 300, MARK), None);
    }
}

// Requirement: Text passes of RGSS3

/// The coverage of a measured glyph, taken from its draw without outline and shadow on a
/// transparent bitmap, where the work rectangle lies at (10, 18).
struct Measured {
    size: (u32, u32),
    coverage: Vec<u8>,
}

impl TextRasterizer for Measured {
    fn metrics(&self, _font: &Font, _text: &str) -> TextMetrics {
        TextMetrics {
            height: self.size.1,
            advances: vec![self.size.0],
            average_width: self.size.0,
            ..TextMetrics::default()
        }
    }

    fn rasterize(&self, _: &Font, _: &str, _: &[u32], condense: Option<(u32, u32)>) -> Mask {
        assert_eq!(condense, None, "the measured glyphs are not squeezed");
        Mask::new(self.size.0, self.size.1, self.coverage.clone())
    }
}

/// One measured draw: its header fields and the pixels of its box, in row order.
struct Solid {
    line: usize,
    case: u32,
    text: (u32, u32),
    outline: bool,
    shadow: bool,
    color: Rgba,
    out_color: Rgba,
    background: Rgba,
    /// Left, top, width and height of the box.
    area: (u32, u32, u32, u32),
    pixels: Vec<Rgba>,
}

fn solids() -> Vec<Solid> {
    let lines: Vec<&str> = SAMPLES.lines().collect();
    samples("solid")
        .into_iter()
        .map(|(fields, line)| {
            let area: (u32, u32, u32, u32) = (
                number(fields[9]),
                number(fields[10]),
                number(fields[11]),
                number(fields[12]),
            );
            let pixels: Vec<Rgba> = lines[line..line + area.3 as usize]
                .iter()
                .flat_map(|row| row.split_whitespace().map(quad))
                .collect();
            assert_eq!(pixels.len() as u32, area.2 * area.3, "line {line}");
            Solid {
                line,
                case: number(fields[1]),
                text: (number(fields[2]), number(fields[3])),
                outline: fields[4] == "1",
                shadow: fields[5] == "1",
                color: quad(fields[6]),
                out_color: quad(fields[7]),
                background: quad(fields[8]),
                area,
                pixels,
            }
        })
        .collect()
}

/// Scenario "Measured draws of the solid glyphs".
#[test]
fn measured_draws_of_the_solid_glyphs() {
    let solids = solids();
    assert_eq!(solids.len(), 16);
    let mut checked = 0;
    for reference in solids.iter().filter(|solid| [0, 16].contains(&solid.case)) {
        assert!(!reference.outline && !reference.shadow);
        assert_eq!(reference.color[3], 255);
        assert_eq!(reference.background[3], 0);
        // The alpha of the reference is the coverage; its work rectangle is at (10, 18).
        let (width, height) = reference.text;
        let mut coverage = vec![0; (width * height) as usize];
        let (left, top, columns, _) = reference.area;
        for (index, pixel) in reference.pixels.iter().enumerate() {
            let (x, y) = (left + index as u32 % columns, top + index as u32 / columns);
            let (u, v) = (x - 10, y - 18);
            if pixel[3] != 0 {
                coverage[(v * width + u) as usize] = pixel[3];
            }
        }
        let glyph = Measured {
            size: reference.text,
            coverage,
        };
        // The eight draws of this glyph, the reference among them.
        for solid in solids
            .iter()
            .filter(|solid| solid.case / 16 == reference.case / 16)
        {
            assert_eq!(solid.text, reference.text);
            let defaults = FontDefaults {
                outline: solid.outline,
                shadow: solid.shadow,
                color: color(solid.color),
                out_color: color(solid.out_color),
                ..FontDefaults::rgss3()
            };
            let mut bitmaps = Bitmaps::new();
            let id = filled(&mut bitmaps, (120, 60), solid.background, &defaults);
            let rect = Rect::new(10, 10, 100, 40);
            bitmaps
                .draw_text_as(id, rect, "x", Align::Left, TextRules::Rgss3, &glyph)
                .unwrap();
            let rgba = bitmaps.rgba(id).unwrap();
            let (left, top, columns, _) = solid.area;
            for (index, expected) in solid.pixels.iter().enumerate() {
                let (x, y) = (left + index as u32 % columns, top + index as u32 / columns);
                assert_eq!(
                    at(rgba, 120, x, y),
                    *expected,
                    "case {} of line {} at ({x}, {y})",
                    solid.case,
                    solid.line
                );
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 16);
}

/// Scenario "Shadow without outline".
#[test]
fn the_shadow_without_outline_is_that_of_rgss2() {
    let draw = |rules: TextRules| {
        let mut bitmaps = Bitmaps::new();
        let mut shadowed = defaults();
        shadowed.shadow = true;
        shadowed.size = 10;
        shadowed.color = Color::new(200.0, 100.0, 50.0, 180.0);
        let id = bitmaps
            .create_from_rgba(40, 20, pattern(40, 20, 9), &shadowed)
            .unwrap();
        // Five cells of 5 by 10 pixels: the rectangle is the cell of the text.
        let rect = Rect::new(4, 3, 25, 10);
        bitmaps
            .draw_text_as(id, rect, "Hello", Align::Left, rules, &CellRasterizer)
            .unwrap();
        bitmaps.rgba(id).unwrap().to_vec()
    };
    assert_eq!(draw(TextRules::Rgss2), draw(TextRules::Rgss3));
    assert_ne!(draw(TextRules::Rgss3), pattern(40, 20, 9));
}

/// A block of full coverage in the pixels (3, 5) to (5, 9) of a text 9 wide and 24 high.
const SQUARE: Block = Block {
    size: (9, 24),
    ink: (3, 5, 6, 10),
    coverage: 255,
};

/// The bitmap after `block` is drawn left aligned by the rules of RGSS3 into the rectangle
/// (10, 10, 50, 26) of a transparent bitmap of 80 by 50, with a font changed by `change`.
/// The work rectangle is at (10, 10) with the outline and at (10, 11) without.
fn drawn_block(block: Block, change: impl Fn(&mut FontDefaults)) -> Vec<u8> {
    let mut defaults = FontDefaults::rgss3();
    change(&mut defaults);
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(80, 50, &defaults).unwrap();
    let rect = Rect::new(10, 10, 50, 26);
    bitmaps
        .draw_text_as(id, rect, "x", Align::Left, TextRules::Rgss3, &block)
        .unwrap();
    bitmaps.rgba(id).unwrap().to_vec()
}

/// Scenario "Shadow outside the work rectangle".
#[test]
fn the_shadow_leaves_the_work_rectangle() {
    // Ink in the last column and row of a work rectangle of 9 by 24.
    let block = Block {
        size: (9, 24),
        ink: (6, 20, 9, 24),
        coverage: 255,
    };
    let mut defaults = FontDefaults::rgss3();
    defaults.outline = false;
    defaults.shadow = true;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(40, 40, &defaults).unwrap();
    let rect = Rect::new(5, 5, 9, 24);
    bitmaps
        .draw_text_as(id, rect, "x", Align::Left, TextRules::Rgss3, &block)
        .unwrap();
    let rgba = bitmaps.rgba(id).unwrap();
    assert_eq!(at(rgba, 40, 13, 28), [255, 255, 255, 255]);
    // One pixel right of and below the rectangle, which ends at column 13 and row 28.
    assert_eq!(at(rgba, 40, 14, 28), [0, 0, 0, 255]);
    assert_eq!(at(rgba, 40, 13, 29), [0, 0, 0, 255]);
    assert_eq!(at(rgba, 40, 14, 29), [0, 0, 0, 255]);
    assert_eq!(at(rgba, 40, 15, 29), [0, 0, 0, 0]);
}

/// Scenario "Colour alpha 0".
#[test]
fn a_colour_of_alpha_0_draws_nothing() {
    let mut bitmaps = Bitmaps::new();
    let mut defaults = probe_font(true);
    defaults.shadow = true;
    defaults.color = Color::new(250.0, 180.0, 60.0, 0.0);
    let id = filled(&mut bitmaps, (300, 100), MARK, &defaults);
    let rect = Rect::new(10, 10, 200, 60);
    bitmaps
        .draw_text_as(id, rect, "H", Align::Left, TextRules::Rgss3, &PROBE_TEXT)
        .unwrap();
    assert_eq!(changed(bitmaps.rgba(id).unwrap(), 300, MARK), None);
}

// Requirement: Outline of RGSS3

/// Scenario "One pass and two, measured". The block is at the columns 13 to 15 and the rows
/// 15 to 19 of the bitmap.
#[test]
fn outline_passes_compose_where_they_overlap() {
    let rgba = drawn_block(SQUARE, |_| {});
    // Diagonally off a corner: one pass.
    for (x, y) in [(12, 14), (16, 14), (16, 20), (12, 20)] {
        assert_eq!(at(&rgba, 80, x, y), [0, 0, 0, 128], "({x}, {y})");
    }
    // Beside a straight edge: two passes.
    for (x, y) in [(12, 17), (16, 17), (14, 14), (14, 20)] {
        assert_eq!(at(&rgba, 80, x, y), [0, 0, 0, 191], "({x}, {y})");
    }
    assert_eq!(at(&rgba, 80, 14, 17), [255, 255, 255, 255]);
    assert_eq!(at(&rgba, 80, 11, 17)[3], 0);
}

/// Scenario "No straight passes".
#[test]
fn the_outline_has_no_straight_passes() {
    // A vertical line one pixel wide: column 14, rows 15 to 19 of the bitmap.
    let line = Block {
        ink: (4, 5, 5, 10),
        ..SQUARE
    };
    let rgba = drawn_block(line, |_| {});
    assert_eq!(at(&rgba, 80, 14, 15), [255, 255, 255, 255]);
    assert_eq!(at(&rgba, 80, 14, 14)[3], 0, "straight above the top end");
    assert_eq!(at(&rgba, 80, 14, 20)[3], 0, "straight below the bottom end");
    assert_eq!(at(&rgba, 80, 13, 14), [0, 0, 0, 128]);
    assert_eq!(at(&rgba, 80, 15, 14), [0, 0, 0, 128]);
}

/// Scenario "Alpha of the outline, measured".
#[test]
fn the_outline_takes_the_alpha_of_the_colour() {
    let rgba = drawn_block(SQUARE, |defaults| {
        defaults.out_color = Color::new(255.0, 0.0, 0.0, 255.0);
        defaults.color = Color::new(255.0, 255.0, 255.0, 128.0);
    });
    assert_eq!(at(&rgba, 80, 12, 14), [255, 0, 0, 128]);
}

/// Scenario "Out colour of alpha 0".
#[test]
fn an_out_colour_of_alpha_0_draws_the_text_alone() {
    let inked = |rgba: &[u8]| -> Vec<(usize, Rgba)> {
        rgba.as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter(|(_, pixel)| pixel[3] != 0)
            .map(|(index, pixel)| (index, *pixel))
            .collect()
    };
    let outlined = drawn_block(SQUARE, |defaults| {
        defaults.out_color = Color::new(255.0, 0.0, 0.0, 0.0);
    });
    // The text pass alone, at the place the outline gives: one row higher than without.
    let mut defaults = FontDefaults::rgss3();
    defaults.outline = false;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(80, 50, &defaults).unwrap();
    let rect = Rect::new(10, 9, 50, 26);
    bitmaps
        .draw_text_as(id, rect, "x", Align::Left, TextRules::Rgss3, &SQUARE)
        .unwrap();
    let alone = inked(bitmaps.rgba(id).unwrap());
    assert_eq!(alone.len(), 3 * 5);
    assert_eq!(inked(&outlined), alone);
}

/// Scenario "Border columns are dropped".
#[test]
fn the_outline_drops_the_border_of_the_work_rectangle() {
    // Ink in the columns 0 to 2 of the cell: columns 10 to 12 of the bitmap.
    let block = Block {
        ink: (0, 5, 3, 10),
        ..SQUARE
    };
    let rgba = drawn_block(block, |defaults| {
        defaults.out_color = Color::new(255.0, 0.0, 0.0, 255.0);
    });
    // Column 0 of the ink is not drawn: the pixel has only the outline of its neighbour.
    assert_eq!(at(&rgba, 80, 10, 17), [255, 0, 0, 255]);
    assert_eq!(at(&rgba, 80, 11, 17), [255, 255, 255, 255]);
    let ((left, ..), _) = changed(&rgba, 80, [0, 0, 0, 0]).unwrap();
    assert_eq!(left, 10, "nothing left of the rectangle");
}

/// Scenario "Shadow between outline and text".
#[test]
fn the_shadow_lies_between_the_outline_and_the_text() {
    let draw = |shadow: bool| {
        drawn_block(SQUARE, |defaults| {
            defaults.out_color = Color::new(255.0, 0.0, 0.0, 255.0);
            defaults.shadow = shadow;
        })
    };
    let (with, without) = (draw(true), draw(false));
    // Diagonally below and right of the block's lower right corner, (15, 19).
    assert_eq!(at(&with, 80, 16, 20), [0, 0, 0, 255]);
    assert_eq!(at(&without, 80, 16, 20), [255, 0, 0, 255]);
    for rgba in [&with, &without] {
        for y in 15..20 {
            for x in 13..16 {
                assert_eq!(at(rgba, 80, x, y), [255, 255, 255, 255], "({x}, {y})");
            }
        }
    }
}

// Requirement: Text squeezing of RGSS3

/// Scenario "Measured widths asked for".
#[test]
fn measured_widths_asked_for() {
    let sweeps = samples("sweep");
    assert_eq!(sweeps.len(), 24);
    let mut pairs = 0;
    for (fields, line) in sweeps {
        let (size, drawn): (u32, u32) = (number(fields[1]), number(fields[2]));
        assert_eq!(fields[3], "|");
        for pair in &fields[4..] {
            let (width, asked) = pair.split_once(':').expect("a pair");
            let (width, asked): (u32, u32) = (number(width), number(asked));
            // The samples write 0 for a text that was not squeezed.
            assert_eq!(
                squeeze_width_rgss3(size, width, drawn).unwrap_or(0),
                asked,
                "line {line}, width {width}"
            );
            assert_eq!(asked == 0, width >= drawn, "line {line}, width {width}");
            pairs += 1;
        }
    }
    assert_eq!(pairs, 2330);
}

/// The average character widths asked for when `text` is drawn at the font size `size`
/// into rectangles of the given widths.
fn asked(text: &Recorder, size: i32, outline: bool, widths: &[i32]) -> Vec<Option<u32>> {
    let mut defaults = FontDefaults::rgss3();
    defaults.size = size;
    defaults.outline = outline;
    let characters = "x".repeat(text.advances.len());
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(200, 40, &defaults).unwrap();
    let mut all = Vec::new();
    for &width in widths {
        let rect = Rect::new(0, 0, width, 40);
        bitmaps
            .draw_text_as(id, rect, &characters, Align::Left, TextRules::Rgss3, text)
            .unwrap();
        let calls = text.take();
        assert_eq!(calls.len(), 1, "the text is set once");
        all.push(calls[0]);
    }
    all
}

/// Scenario "Size 20, measured".
#[test]
fn squeeze_at_size_20() {
    let text = Recorder::new(&[13; 10], 20, 10);
    assert_eq!(
        asked(&text, 20, false, &[130, 129, 124, 123, 109, 108, 12]),
        [None, Some(7), Some(7), Some(6), Some(6), Some(5), Some(5)]
    );
}

/// Scenario "Outline squeezes earlier, measured".
#[test]
fn the_outline_squeezes_earlier() {
    let text = Recorder::new(&[9; 12], 24, 9);
    let widths = [109, 101, 100, 89];
    assert_eq!(
        asked(&text, 24, false, &widths),
        [None, Some(8), Some(8), Some(7)]
    );
    assert_eq!(
        asked(&text, 24, true, &widths),
        [Some(8), Some(8), Some(7), Some(6)]
    );
    // With the outline the text fits from two pixels more.
    assert_eq!(
        asked(&text, 24, true, &[110, 111]),
        [None, None],
        "the drawn width is 110 with the outline"
    );
}

/// Scenario "Squeezed text is placed by its own width".
#[test]
fn a_squeezed_text_is_placed_by_its_own_width() {
    // Ten cells of 11 pixels at size 22, 112 with the outline, in a rectangle of 100: the
    // width asked for is trunc(22 * (100 / 112 - 0.15) * 7 / 16) = 7, so the text is 70
    // wide and its drawn width 72.
    let mut defaults = FontDefaults::rgss3();
    defaults.size = 22;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(200, 40, &defaults).unwrap();
    let rect = Rect::new(20, 5, 100, 30);
    bitmaps
        .draw_text_as(
            id,
            rect,
            "ABCDEFGHIJ",
            Align::Right,
            TextRules::Rgss3,
            &CellRasterizer,
        )
        .unwrap();
    let rgba = bitmaps.rgba(id).unwrap();
    let ((left, ..), _) = changed(rgba, 200, [0, 0, 0, 0]).unwrap();
    assert_eq!(left, 20 + 100 - 72);
    // The cells are 7 wide: the eleventh column of the text is the second character's.
    let mask = CellRasterizer.rasterize(
        bitmaps.font(id).unwrap(),
        "ABCDEFGHIJ",
        &[7; 10],
        Some((7, 11)),
    );
    assert_eq!(mask.width(), 70);
    assert_ne!(at(rgba, 200, left + 10, 20)[3], 0);
    // Right of the 70 columns of the text there is no ink.
    assert_eq!(at(rgba, 200, left + 72, 20)[3], 0);
}

/// A text whose squeezed width is 0 draws nothing, also where the natural one would.
#[test]
fn a_squeezed_width_of_0_draws_nothing() {
    // Size 2 asks for trunc(2 * 0.6 * 7 / 16) = 0.
    let text = Recorder::new(&[4; 10], 2, 2);
    let mut defaults = FontDefaults::rgss3();
    defaults.size = 2;
    defaults.outline = false;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(60, 20, &defaults).unwrap();
    let rect = Rect::new(0, 0, 10, 20);
    bitmaps
        .draw_text_as(id, rect, "ABCDEFGHIJ", Align::Left, TextRules::Rgss3, &text)
        .unwrap();
    assert_eq!(text.take(), [Some(0)]);
    assert_eq!(changed(bitmaps.rgba(id).unwrap(), 60, [0, 0, 0, 0]), None);
}

// Requirement: Text measurement of RGSS3

/// Scenarios "Outline and shadow" and "Empty string".
#[test]
fn the_measured_size_counts_neither_outline_nor_shadow() {
    let mut bitmaps = Bitmaps::new();
    let mut plain = FontDefaults::rgss3();
    plain.outline = false;
    let mut both = FontDefaults::rgss3();
    both.shadow = true;
    let without = bitmaps.create(8, 8, &plain).unwrap();
    let with = bitmaps.create(8, 8, &both).unwrap();
    let overhanging = Recorder::new(&[9; 5], 24, 9);
    for text in ["Hello", "H", " "] {
        let size = bitmaps.text_size(without, text, &overhanging).unwrap();
        assert_eq!(bitmaps.text_size(with, text, &overhanging).unwrap(), size);
        assert_eq!(size.height, 24);
        assert_eq!(size.width, 9 * text.len() as i32);
    }
    assert_eq!(
        bitmaps.text_size(with, "", &overhanging),
        Ok(Rect::new(0, 0, 0, 0))
    );
}

// Requirement: Largest bitmap of RGSS3

/// Scenarios "Above the limit of RGSS2" and "The limit".
#[test]
fn the_largest_bitmap_is_that_of_the_arena() {
    assert_eq!(MAX_BITMAP_PIXELS, 268_435_456);
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(2049, 2048, &FontDefaults::rgss3()).unwrap();
    assert_eq!(bitmaps.width(id), Ok(2049));
    assert!(matches!(
        bitmaps.create(32768, 32768, &FontDefaults::rgss3()),
        Err(BitmapError::CreateFailed(
            crate::CreateFailure::TooLarge { .. }
        ))
    ));
}
