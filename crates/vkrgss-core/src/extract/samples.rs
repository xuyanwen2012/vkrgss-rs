//! The cases of `sprite-geometry-samples.txt`: sprites measured on the original runtime,
//! each set up as its line says and compared with the pixels the file lists.

use vkrgss_pixel::Rgba;

use super::testing::*;
use crate::Rect;

const SAMPLES: &str = include_str!("../../tests/data/sprite-geometry-samples.txt");

/// One measured sprite.
#[derive(Debug)]
struct Case {
    name: &'static str,
    /// The number of the case inside its topic.
    number: u32,
    x: i32,
    y: i32,
    ox: i32,
    oy: i32,
    zoom: (f64, f64),
    angle: f64,
    mirror: bool,
    src_rect: Option<Rect>,
    bush_depth: i32,
    /// Rectangle and origin of the viewport.
    viewport: Option<(Rect, i32, i32)>,
    /// The number of pixels drawn.
    drawn: usize,
    /// Left, top, right and bottom of the pixels drawn.
    bbox: Option<[u32; 4]>,
    /// The rows listed, from the left to the right edge of `bbox`.
    rows: Vec<(u32, Vec<Rgba>)>,
}

/// The numbers of a list such as `[20, 15, 110, 80]` or `2,2`.
fn numbers(text: &str) -> Vec<f64> {
    text.trim_matches(['[', ']', ')'])
        .split(',')
        .map(|number| number.trim().parse().unwrap())
        .collect()
}

/// The pixel a token of a row stands for, over black.
fn pixel(token: &str) -> Rgba {
    if token == "." {
        return BLACK;
    }
    let (texel, faint) = match token.strip_suffix('*') {
        Some(texel) => (texel, true),
        None => (token, false),
    };
    let at = numbers(texel);
    let full = [8 + 10 * at[0] as u32, 8 + 15 * at[1] as u32, 200];
    let [r, g, b] = full.map(|value| if faint { (value * 128) >> 8 } else { value } as u8);
    [r, g, b, 255]
}

/// The cases of `topic`.
fn cases(topic: &str) -> Vec<Case> {
    let mut cases: Vec<Case> = Vec::new();
    let prefix = format!("case {topic}/");
    let mut inside = false;
    for line in SAMPLES.lines() {
        if let Some(rest) = line.strip_prefix("case ") {
            inside = line.starts_with(&prefix);
            if !inside {
                continue;
            }
            let (name, setup) = rest.split_once(" frame ").unwrap();
            // What follows `key` up to the next key.
            let value = |key: &str| -> &'static str {
                let rest = &setup[setup.find(key).unwrap() + key.len()..];
                let end = rest.find(['=']).map_or(rest.len(), |at| {
                    rest[..at].rfind(' ').expect("a key follows a space")
                });
                rest[..end].trim()
            };
            let whole = |key: &str| value(key).parse::<i32>().unwrap();
            let zoom = numbers(value(" zoom="));
            let src_rect = match value(" src_rect=") {
                "whole" => None,
                list => {
                    let r = numbers(list).iter().map(|&n| n as i32).collect::<Vec<_>>();
                    Some(Rect::new(r[0], r[1], r[2], r[3]))
                }
            };
            let viewport = match value(" viewport=") {
                "none" => None,
                list => {
                    let v = numbers(list).iter().map(|&n| n as i32).collect::<Vec<_>>();
                    Some((Rect::new(v[0], v[1], v[2], v[3]), v[4], v[5]))
                }
            };
            cases.push(Case {
                name,
                number: name.split_once('/').unwrap().1.parse().unwrap(),
                x: whole(" x="),
                y: whole(" y="),
                ox: whole(" ox="),
                oy: whole(" oy="),
                zoom: (zoom[0], zoom[1]),
                angle: value(" angle=").parse().unwrap(),
                mirror: whole(" mirror=") == 1,
                src_rect,
                bush_depth: whole(" bush_depth="),
                viewport,
                drawn: 0,
                bbox: None,
                rows: Vec::new(),
            });
        } else if !inside {
            continue;
        } else if let Some(rest) = line.trim_start().strip_prefix("drawn ") {
            let case = cases.last_mut().unwrap();
            assert!(rest.ends_with("model wrong 0"), "{}", case.name);
            case.drawn = rest.split(' ').next().unwrap().parse().unwrap();
            if let Some(at) = rest.find("bbox [") {
                let list = rest[at + 6..].split(')').next().unwrap();
                let bbox = numbers(list).iter().map(|&n| n as u32).collect::<Vec<_>>();
                case.bbox = Some([bbox[0], bbox[1], bbox[2], bbox[3]]);
            }
        } else if let Some(rest) = line.trim_start().strip_prefix("row ") {
            let (y, tokens) = rest.split_once(':').unwrap();
            let pixels = tokens.split_whitespace().map(pixel).collect();
            let case = cases.last_mut().unwrap();
            case.rows.push((y.parse().unwrap(), pixels));
        }
    }
    cases
}

/// The frame of one case, drawn alone on a screen of 640 by 480.
fn draw(case: &Case) -> vkrgss_compose_ref::Frame {
    let mut stage = Stage::new();
    let pattern: Vec<Rgba> = (0..16u8)
        .flat_map(|y| (0..24u8).map(move |x| [8 + 10 * x, 8 + 15 * y, 200, 255]))
        .collect();
    let bitmap = stage.pixels(24, 16, &pattern);
    let viewport = case.viewport.map(|(rect, ox, oy)| {
        let id = stage.scene.create_viewport(rect).unwrap();
        let viewport = stage.scene.viewport_mut(id).unwrap();
        (viewport.ox, viewport.oy) = (ox, oy);
        id
    });
    let id = stage.scene.create_sprite(viewport).unwrap();
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
    (sprite.x, sprite.y) = (case.x, case.y);
    (sprite.ox, sprite.oy) = (case.ox, case.oy);
    (sprite.zoom_x, sprite.zoom_y) = case.zoom;
    sprite.angle = case.angle;
    sprite.mirror = case.mirror;
    sprite.bush_depth = case.bush_depth;
    if let Some(rect) = case.src_rect {
        sprite.src_rect = rect;
    }
    stage.frame()
}

/// The case `name`, drawn.
fn drawn_case(name: &str) -> vkrgss_compose_ref::Frame {
    let topic = name.split_once('/').unwrap().0;
    let found = cases(topic).into_iter().find(|case| case.name == name);
    draw(&found.unwrap())
}

/// Draws every case of `topic` that `chosen` picks and compares it with what was measured:
/// the number of pixels drawn inside the cell of the case, their bounding box and the rows
/// listed. Returns the number of cases compared.
fn compare(topic: &str, chosen: impl Fn(&Case) -> bool) -> usize {
    let mut compared = 0;
    for case in cases(topic).iter().filter(|case| chosen(case)) {
        let name = case.name;
        let frame = draw(case);

        let (left, top) = (160 * (case.number % 4), 120 * (case.number / 4 % 4));
        let mut drawn = Vec::new();
        for y in top..top + 120 {
            for x in left..left + 160 {
                if frame.pixel(x, y) != BLACK {
                    drawn.push((x, y));
                }
            }
        }
        assert_eq!(drawn.len(), case.drawn, "pixels drawn of {name}");
        if let Some(bbox) = case.bbox {
            let xs = drawn.iter().map(|at| at.0);
            let ys = drawn.iter().map(|at| at.1);
            let found = [
                xs.clone().min().unwrap(),
                ys.clone().min().unwrap(),
                xs.max().unwrap() + 1,
                ys.max().unwrap() + 1,
            ];
            assert_eq!(found, bbox, "bounding box of {name}");
            assert!(!case.rows.is_empty(), "{name}");
        }
        for (y, pixels) in &case.rows {
            let bbox = case.bbox.unwrap();
            assert_eq!(pixels.len() as u32, bbox[2] - bbox[0], "{name}");
            for (x, expected) in (bbox[0]..).zip(pixels) {
                assert_eq!(frame.pixel(x, *y), *expected, "({x}, {y}) of {name}");
            }
        }
        compared += 1;
    }
    compared
}

/// Turned sprites inside viewports with the origins (0, 0), (13, 7), (-9, -5) and
/// (100, -50), cut by each edge of the viewport.
#[test]
fn turned_sprites_in_viewports_show_the_measured_pixels() {
    assert_eq!(compare("turnedvp", |_| true), 32);
    // With the origin (100, -50) the picture leaves its box.
    let far: Vec<Case> = cases("turnedvp")
        .into_iter()
        .filter(|case| case.viewport.is_some_and(|viewport| viewport.1 == 100))
        .collect();
    assert_eq!(far.len(), 8);
    assert!(far.iter().all(|case| case.drawn == 0));
}

/// Viewports partly off screen, a mirrored source rectangle, one origin only, a full turn
/// and boxes with a bush depth cut by the viewport and by the screen.
#[test]
fn turned_sprites_in_cut_viewports_show_the_measured_pixels() {
    let turned = |case: &Case| (32..48).contains(&case.number) && case.angle != 0.0;
    assert_eq!(compare("geom2", turned), 15);
}

/// The bush depth of turned sprites: the lowest rows of the box are the faint ones.
#[test]
fn bush_depths_of_turned_sprites_show_the_measured_rows() {
    assert_eq!(compare("bush", |case| case.angle != 0.0), 44);
}

/// Turned sprites whose source rectangle leaves the bitmap draw nothing.
#[test]
fn turned_sprites_with_source_rectangles_show_the_measured_pixels() {
    let turned = |case: &Case| case.angle != 0.0;
    assert_eq!(compare("srcrect", turned), 16);
    assert_eq!(compare("geom2", |case| case.number < 16), 16);
}

/// The worked example of a viewport with an origin: the box is that of a sprite at
/// (55, 40) in the viewport, the picture that of one at (68, 47).
#[test]
fn a_viewport_origin_displaces_the_picture_inside_its_box() {
    let frame = drawn_case("turnedvp/1");
    assert_eq!(frame.pixel(240, 60), pixel("9,5"));
    assert_eq!(frame.pixel(250, 70), pixel("10,11"));
    assert_eq!(frame.pixel(230, 45), BLACK);
    assert_eq!(frame.pixel(220, 58), BLACK);
    let frame = drawn_case("turnedvp/2");
    assert_eq!(frame.pixel(380, 45), pixel("10,4"));
    assert_eq!(frame.pixel(400, 60), pixel("15,15"));
    assert_eq!(frame.pixel(414, 75), BLACK);
}

/// A sprite of 24 by 16 at (240, 60) with the origin (12, 8), zoom 2, angle 30 and bush
/// depth 8: its box spans the rows 34 to 85, and the last 8 of them are faint.
#[test]
fn the_lowest_rows_of_a_turned_box_are_faint() {
    let frame = drawn_case("bush/1");
    let faint: Vec<Rgba> = (0..16)
        .flat_map(|v| (0..24).map(move |u| pixel(&format!("{u},{v}*"))))
        .collect();
    let row = |y: u32| -> Vec<Rgba> {
        let pixels = (0..640).map(|x| frame.pixel(x, y));
        pixels.filter(|pixel| *pixel != BLACK).collect()
    };
    assert!(row(34).is_empty());
    for y in 35..78 {
        assert!(!row(y).is_empty(), "row {y}");
        assert!(row(y).iter().all(|pixel| !faint.contains(pixel)), "row {y}");
    }
    for y in 78..86 {
        assert!(!row(y).is_empty(), "row {y}");
        assert!(row(y).iter().all(|pixel| faint.contains(pixel)), "row {y}");
    }
    assert_eq!((row(77).len(), row(78).len(), row(85).len()), (20, 18, 2));
    assert!(row(86).is_empty());
}

/// A turned sprite whose source rectangle leaves its bitmap by one texel is not drawn.
#[test]
fn a_turned_sprite_with_a_source_rectangle_outside_its_bitmap_draws_nothing() {
    let rects = cases("geom2").into_iter().filter(|case| case.number < 6);
    for case in rects {
        assert_eq!(case.angle, 30.0);
        assert!(all(&draw(&case), BLACK), "{}", case.name);
    }
    let first = &cases("geom2")[0];
    assert_eq!(first.src_rect, Some(Rect::new(-1, 0, 24, 16)));
    assert!(!all(&drawn_case("geom2/6"), BLACK));
}

/// Sprites that are not turned, with a bush depth and a vertical zoom: the faint rows are
/// a stretch of their own.
#[test]
fn bush_depths_of_zoomed_sprites_show_the_measured_rows() {
    assert_eq!(compare("bushzoom", |_| true), 16);
    assert_eq!(compare("geom2", |case| (16..32).contains(&case.number)), 16);
    assert_eq!(compare("bush", |case| case.angle == 0.0), 4);
    assert_eq!(compare("geom2", |case| case.number == 42), 1);
}

/// Sprites that are not turned, with source rectangles that leave the bitmap on every
/// side, at several zooms, mirrored and not.
#[test]
fn cut_source_rectangles_show_the_measured_pixels() {
    assert_eq!(compare("srcrect", |case| case.angle == 0.0), 96);
    assert_eq!(compare("srcrect2", |_| true), 16);
}

/// A sprite of the 24 by 16 pattern at (10, 10), or (20, 20) with a bush depth, that is
/// not turned.
fn upright(src_rect: Option<Rect>, zoom: (f64, f64), mirror: bool, bush_depth: i32) -> Case {
    let at = if bush_depth > 0 { 20 } else { 10 };
    Case {
        name: "",
        number: 0,
        x: at,
        y: at,
        ox: 0,
        oy: 0,
        zoom,
        angle: 0.0,
        mirror,
        src_rect,
        bush_depth,
        viewport: None,
        drawn: 0,
        bbox: None,
        rows: Vec::new(),
    }
}

/// The texel column of every pixel of `row` that is drawn, with the first of them.
fn columns(frame: &vkrgss_compose_ref::Frame, row: u32) -> (u32, Vec<u32>) {
    let shown = |x: u32| {
        let pixel = frame.pixel(x, row);
        (pixel != BLACK).then(|| (u32::from(pixel[0]) - 8) / 10)
    };
    let first = (0..640).find(|&x| shown(x).is_some()).unwrap();
    let texels: Vec<u32> = (first..640).map_while(shown).collect();
    assert!((first + texels.len() as u32..640).all(|x| shown(x).is_none()));
    (first, texels)
}

/// The texel row of every pixel of `column` that is drawn, with the first of them and
/// whether each is faint.
fn texel_rows(frame: &vkrgss_compose_ref::Frame, column: u32) -> (u32, Vec<(u32, bool)>) {
    let shown = |y: u32| {
        let pixel = frame.pixel(column, y);
        // The blue of every texel is 200, and 100 where it is faint.
        let faint = pixel[2] == 100;
        let green = u32::from(pixel[1]);
        let row = (0..16).find(|v| {
            let full = 8 + 15 * v;
            green == if faint { (full * 128) >> 8 } else { full }
        });
        (pixel != BLACK).then(|| (row.unwrap(), faint))
    };
    let first = (0..480).find(|&y| shown(y).is_some()).unwrap();
    let rows: Vec<(u32, bool)> = (first..480).map_while(shown).collect();
    assert!((first + rows.len() as u32..480).all(|y| shown(y).is_none()));
    (first, rows)
}

/// Every texel of `range` as many times as `times` says, in order.
fn repeated(range: impl Iterator<Item = u32>, times: usize) -> Vec<u32> {
    range.flat_map(|texel| [texel].repeat(times)).collect()
}

/// The table of measured columns of the design: source rectangles that leave a bitmap of
/// 24 by 16 on each side, with the columns and rows drawn and the texels they show.
#[test]
fn cut_source_rectangles_show_the_columns_of_the_table() {
    // The first column drawn and the texel of every column, in a row that is drawn.
    let across = |rect: [i32; 4], zoom, mirror, x: i32, row| {
        let mut case = upright(
            Some(Rect::new(rect[0], rect[1], rect[2], rect[3])),
            zoom,
            mirror,
            0,
        );
        case.x = x;
        columns(&draw(&case), row)
    };
    // The first row drawn and the texel of every row, in a column that is drawn.
    let down = |rect: [i32; 4], zoom, column| {
        let case = upright(
            Some(Rect::new(rect[0], rect[1], rect[2], rect[3])),
            zoom,
            false,
            0,
        );
        let (first, rows) = texel_rows(&draw(&case), column);
        (first, rows.into_iter().map(|row| row.0).collect::<Vec<_>>())
    };
    let z2 = (2.0, 2.0);
    let all_but_last = |texels: Vec<u32>| texels[..texels.len() - 1].to_vec();

    assert_eq!(
        across([-6, 0, 24, 16], z2, false, 10, 20),
        (22, repeated(0..18, 2))
    );
    let (first, texels) = across([6, 0, 24, 16], z2, false, 10, 20);
    assert_eq!((first, texels.len()), (10, 35), "the last column is lost");
    assert_eq!(texels, all_but_last(repeated(6..24, 2)));
    let (first, texels) = across([-6, -4, 36, 24], z2, false, 10, 30);
    assert_eq!((first, first + texels.len() as u32 - 1), (22, 68));
    assert_eq!(texels, all_but_last(repeated(0..24, 2)));
    let (first, rows) = down([-6, -4, 36, 24], z2, 30);
    assert_eq!((first, first + rows.len() as u32 - 1), (18, 48));
    assert_eq!(rows, all_but_last(repeated(0..16, 2)));
    assert_eq!(
        across([20, 0, 12, 8], z2, false, 10, 12),
        (10, vec![20, 20, 21, 21, 22, 22, 23])
    );
    assert_eq!(
        across([-6, 0, 12, 16], z2, false, 10, 20),
        (22, repeated(0..6, 2))
    );
    let (first, rows) = down([0, 4, 24, 16], z2, 20);
    assert_eq!((first, first + rows.len() as u32 - 1), (10, 32));
    assert_eq!(rows, all_but_last(repeated(4..16, 2)));
    // One axis at zoom 1 is cut as the stretched one is.
    assert_eq!(
        across([6, 0, 24, 16], (1.0, 2.0), false, 10, 20),
        (10, (6..23).collect())
    );
    assert_eq!(
        down([0, 4, 24, 16], (2.0, 1.0), 20),
        (10, (4..15).collect())
    );
    // Not stretched at all, nothing is lost.
    assert_eq!(
        across([6, 0, 24, 16], (1.0, 1.0), false, 10, 20),
        (10, (6..24).collect())
    );

    // Mirrored, the rectangle is mirrored before it is cut.
    let (first, texels) = across([-6, 0, 24, 16], z2, true, 30, 20);
    assert_eq!((first, texels.len()), (30, 35));
    assert_eq!(texels, all_but_last(repeated((0..18).rev(), 2)));
    assert_eq!(
        across([6, 0, 24, 16], z2, true, 30, 20),
        (42, repeated((6..24).rev(), 2))
    );
    let (first, texels) = across([-6, -4, 36, 24], z2, true, 30, 30);
    assert_eq!((first, first + texels.len() as u32 - 1), (42, 88));
    assert_eq!(texels, all_but_last(repeated((0..24).rev(), 2)));
    assert_eq!(
        across([-6, 0, 12, 16], (2.5, 3.0), true, 27, 20),
        (27, vec![5, 5, 5, 4, 4, 4, 3, 3, 2, 2, 2, 1, 1, 0])
    );
}

/// The table of item 3b of the design: the texel row of every screen row of a sprite of
/// 16 rows, top to bottom, the faint ones marked.
#[test]
fn bush_depths_of_zoomed_sprites_show_the_rows_of_the_table() {
    let measured = |zoom_y: f64, depth: i32| {
        let frame = draw(&upright(None, (2.0, zoom_y), false, depth));
        let (first, rows) = texel_rows(&frame, 30);
        assert_eq!(first, 20);
        rows
    };
    // The rows as the table writes them: a texel row, faint with a star.
    let table = |text: &str| -> Vec<(u32, bool)> {
        let row = |token: &str| {
            (
                token.trim_end_matches('*').parse().unwrap(),
                token.ends_with('*'),
            )
        };
        text.split_whitespace().map(row).collect()
    };
    let full = |range: std::ops::Range<u32>, times: usize| -> Vec<(u32, bool)> {
        repeated(range, times)
            .into_iter()
            .map(|row| (row, false))
            .collect()
    };

    // 16 source rows into 31, and no faint row.
    let rows = measured(2.0, 1);
    assert_eq!(rows.len(), 31);
    assert_eq!(rows, [full(0..15, 2), table("15")].concat());
    let rows = measured(2.0, 3);
    assert_eq!(rows.len(), 32);
    assert_eq!(rows, [full(0..14, 2), table("14 15* 15* 15*")].concat());
    let rows = measured(2.0, 5);
    assert_eq!(rows.len(), 32);
    assert_eq!(
        rows,
        [full(0..13, 2), table("13 14* 14* 14* 15* 15*")].concat()
    );
    let rows = measured(3.0, 2);
    assert_eq!(rows.len(), 46);
    assert_eq!(
        rows,
        [full(0..8, 3), table("8 8"), full(9..15, 3), table("15 15")].concat()
    );
    let rows = measured(3.0, 4);
    assert_eq!(rows.len(), 48);
    assert_eq!(
        rows,
        [full(0..14, 3), table("14 14 15* 15* 15* 15*")].concat()
    );
    assert_eq!(
        measured(1.5, 3),
        table("0 0 1 1 2 3 3 4 5 5 6 7 7 8 9 9 10 11 11 12 13 14* 14* 15*")
    );
    assert_eq!(measured(0.5, 3), table("0 2 4 6 8 10* 12* 14*"));
    assert_eq!(
        measured(2.5, 6),
        table(
            "0 0 0 1 1 2 2 2 3 3 4 4 4 5 5 6 6 6 7 7 8 8 9 9 9 10 10 11 11 11 12 12 13 13 \
             14* 14* 14* 14* 15* 15*"
        )
    );
}
