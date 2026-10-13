use vkxp_compose_ref::Compositor;
use vkxp_pixel::Rgba;
use vkxp_proto::{Op, Sampling};

use crate::extract::testing::*;
use crate::test_support::sprite_cases::{cases, digest, frames};
use crate::{Rect, Rules, Scene, Sprite, SpriteId};

/// A sprite of `stage` showing a bitmap of `width` by `height` whose red is the column and
/// whose green is the row, set up by `set`.
fn coded(stage: &mut Stage, width: u32, height: u32, set: impl FnOnce(&mut Sprite)) -> SpriteId {
    let pixels: Vec<Rgba> = (0..height)
        .flat_map(|y| (0..width).map(move |x| [x as u8, y as u8, 200, 255]))
        .collect();
    let bitmap = stage.pixels(width, height, &pixels);
    let id = stage.scene.create_sprite(None).unwrap();
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
    set(sprite);
    id
}

/// The rectangles the stretch quads of the next frame are drawn into.
fn dests(stage: &mut Stage) -> Vec<vkxp_proto::Rect> {
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let quads = packet.ops.iter().filter_map(|op| match op {
        Op::Quad(quad) => Some(quad.sampling),
        _ => None,
    });
    quads
        .filter_map(|sampling| match sampling {
            Sampling::Stretch { dest, .. } => Some(dest),
            Sampling::Affine(_) => None,
        })
        .collect()
}

/// Scenario "Truncation, measured on the original".
#[test]
fn the_left_edge_is_truncated_in_rgss2_and_rounded_down_in_rgss1() {
    let place = |sprite: &mut Sprite| {
        sprite.x = 0;
        sprite.ox = 5;
        sprite.zoom_x = 2.5;
    };
    let mut stage = Stage::rgss2(64, 32);
    coded(&mut stage, 24, 16, place);
    assert_eq!(dests(&mut stage), [vkxp_proto::Rect::new(-12, 0, 60, 16)]);
    let mut stage = Stage::sized(64, 32);
    coded(&mut stage, 24, 16, place);
    assert_eq!(dests(&mut stage), [vkxp_proto::Rect::new(-13, 0, 60, 16)]);
}

/// Scenario "Width depends on the place, measured on the original".
#[test]
fn the_width_depends_on_the_place() {
    let mut stage = Stage::rgss2(544, 16);
    for x in [0, 272] {
        coded(&mut stage, 90, 8, |sprite| {
            sprite.x = x;
            sprite.zoom_x = 0.7;
        });
    }
    let widths: Vec<u32> = dests(&mut stage).iter().map(|dest| dest.width).collect();
    assert_eq!(widths, [62, 63]);
    // In RGSS1 both are 63 wide.
    let mut stage = Stage::sized(544, 16);
    for x in [0, 272] {
        coded(&mut stage, 90, 8, |sprite| {
            sprite.x = x;
            sprite.zoom_x = 0.7;
        });
    }
    let widths: Vec<u32> = dests(&mut stage).iter().map(|dest| dest.width).collect();
    assert_eq!(widths, [63, 63]);
}

/// In a viewport the place and the size are found in the viewport's coordinates, and its
/// offset is added afterwards.
#[test]
fn the_place_is_found_in_the_coordinates_of_the_viewport() {
    let mut stage = Stage::rgss2(544, 32);
    let viewport = Rect::new(272, 3, 200, 20);
    let viewport = stage.scene.create_viewport(viewport).unwrap();
    stage.scene.viewport_mut(viewport).unwrap().ox = 7;
    let id = coded(&mut stage, 90, 8, |sprite| {
        sprite.ox = 5;
        sprite.zoom_x = 0.7;
    });
    stage.scene.set_sprite_viewport(id, Some(viewport)).unwrap();
    // trunc(0 - 3.5) is -3 and floor(-3 + 62.99...) + 3 is 62; at 272 - 7 - 3.5 the width
    // would be 63.
    assert_eq!(dests(&mut stage), [vkxp_proto::Rect::new(262, 3, 62, 8)]);
}

/// A zoom or a side of 0 or less draws nothing.
#[test]
fn a_sprite_without_a_size_draws_nothing() {
    for (zoom_x, zoom_y, width) in [(0.0, 1.0, 8), (1.0, -2.0, 8), (0.01, 1.0, 8), (1.0, 1.0, 0)] {
        let mut stage = Stage::rgss2(32, 32);
        coded(&mut stage, 8, 8, |sprite| {
            sprite.zoom_x = zoom_x;
            sprite.zoom_y = zoom_y;
            sprite.src_rect.width = width;
        });
        assert!(all(&stage.frame(), BLACK), "{zoom_x} {zoom_y} {width}");
    }
}

/// The red of every pixel of row `y` that is not black, with its column.
fn texels(stage: &mut Stage, y: u32) -> Vec<(u32, u8)> {
    let frame = stage.frame();
    (0..frame.size.width)
        .filter(|x| frame.pixel(*x, y) != BLACK)
        .map(|x| (x, frame.pixel(x, y)[0]))
        .collect()
}

/// Scenario "Mirrored cut, measured on the original".
#[test]
fn an_unstretched_mirrored_sprite_is_cut_first_and_mirrored_then() {
    let cut = |sprite: &mut Sprite| {
        sprite.src_rect = Rect::new(-5, 6, 18, 17);
        sprite.mirror = true;
    };
    let mut stage = Stage::rgss2(32, 32);
    coded(&mut stage, 24, 16, cut);
    let expected: Vec<(u32, u8)> = (5..18).zip((0..13).rev()).collect();
    assert_eq!(texels(&mut stage, 0), expected);
    // The rows 6 to 15 of the bitmap are the first ten of the sprite.
    assert_eq!(stage.frame().pixel(5, 9), [12, 15, 200, 255]);
    assert_eq!(stage.frame().pixel(5, 10), BLACK);

    // RGSS1 mirrors first: the texels stand in the columns 0 to 12.
    let mut stage = Stage::sized(32, 32);
    coded(&mut stage, 24, 16, cut);
    let expected: Vec<(u32, u8)> = (0..13).zip((0..13).rev()).collect();
    assert_eq!(texels(&mut stage, 0), expected);
}

/// A stretched sprite whose source rectangle leaves its bitmap is mirrored first and cut
/// then, as in RGSS1, and drawn from a picture of its own.
#[test]
fn a_stretched_cut_sprite_is_drawn_as_in_rgss1() {
    let cut = |sprite: &mut Sprite| {
        sprite.src_rect = Rect::new(-5, 6, 18, 17);
        sprite.mirror = true;
        sprite.zoom_x = 2.0;
        sprite.zoom_y = 1.5;
    };
    let mut rgss2 = Stage::rgss2(48, 32);
    let id = coded(&mut rgss2, 24, 16, cut);
    let mut rgss1 = Stage::sized(48, 32);
    coded(&mut rgss1, 24, 16, cut);
    assert_eq!(rgss2.frame().rgba, rgss1.frame().rgba);
    assert!(!all(&rgss2.frame(), BLACK));
    // The picture is given back when the sprite needs none.
    let picture = rgss2.scene.sprite(id).unwrap().turned.bitmap.unwrap();
    rgss2.scene.sprite_mut(id).unwrap().src_rect = Rect::new(0, 0, 24, 16);
    rgss2.frame();
    assert_eq!(rgss2.scene.sprite(id).unwrap().turned.bitmap, None);
    assert!(rgss2.bitmaps.is_disposed(picture));
}

/// Rule 4: `bitmap=` with the bitmap the sprite has keeps the source rectangle in RGSS2.
#[test]
fn the_same_bitmap_keeps_the_source_rectangle_in_rgss2() {
    for (rules, kept) in [(Rules::Rgss2, true), (Rules::Rgss1, false)] {
        let mut stage = Stage::sized(32, 32);
        stage.scene = Scene::with_rules(rules, stage.scene.logical_size());
        let first = stage.solid(8, 8, RED);
        let second = stage.solid(4, 6, GREEN);
        let id = stage.scene.create_sprite(None).unwrap();
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.set_bitmap(Some(first), &stage.bitmaps);
        assert_eq!(sprite.src_rect, Rect::new(0, 0, 8, 8));
        sprite.src_rect = Rect::new(1, 2, 3, 4);
        sprite.set_bitmap(Some(first), &stage.bitmaps);
        let expected = if kept {
            Rect::new(1, 2, 3, 4)
        } else {
            Rect::new(0, 0, 8, 8)
        };
        assert_eq!(sprite.src_rect, expected, "{rules:?}");
        // Another bitmap sets it, and no bitmap keeps it.
        sprite.set_bitmap(Some(second), &stage.bitmaps);
        assert_eq!(sprite.src_rect, Rect::new(0, 0, 4, 6));
        sprite.set_bitmap(None, &stage.bitmaps);
        assert_eq!(sprite.src_rect, Rect::new(0, 0, 4, 6));
        assert_eq!(sprite.bitmap(), None);
    }
}

/// Composes every frame of every scene of `text` with the reference compositor and asserts
/// that it is the picture its digest names.
fn compare(text: &str, scenes: usize) {
    let cases = cases(text);
    assert_eq!(cases.len(), scenes);
    let mut wrong = Vec::new();
    for case in cases {
        let mut compositor = Compositor::new();
        for (index, (packet, expected)) in frames(&case).into_iter().enumerate() {
            let frame = compositor
                .render(&packet)
                .expect("the reference compositor accepts every extracted packet");
            if digest(frame.size, &frame.rgba) != expected {
                wrong.push(format!("{}, frame {index}", case.name));
            }
        }
    }
    assert!(wrong.is_empty(), "not the expected pictures: {wrong:?}");
}

/// Scenario "Cases of the measurement" of "Sprite placement of RGSS2".
#[test]
fn the_cases_of_place_and_size_are_the_expected_pictures() {
    compare(
        include_str!("../../../tests/data/sprite-rgss2-place.txt"),
        14,
    );
}

/// The opacities of the quads of the next frame, in draw order.
fn opacities(stage: &mut Stage) -> Vec<u8> {
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let quads = packet.ops.iter().filter_map(|op| match op {
        Op::Quad(quad) => Some(quad.opacity),
        _ => None,
    });
    quads.collect()
}

/// Scenario "Opacity, measured on the original".
#[test]
fn the_bush_rows_are_drawn_at_the_product_of_the_two_opacities() {
    for (opacity, bush_opacity, expected) in [
        (255, 128, Some(128)),
        (161, 300, Some(189)),
        (255, 256, None),
        (255, -1, None),
        (100, 255, Some(100)),
    ] {
        let mut stage = Stage::rgss2(32, 32);
        coded(&mut stage, 8, 8, |sprite| {
            sprite.set_opacity(opacity);
            sprite.bush_depth = 3;
            sprite.bush_opacity = bush_opacity;
        });
        let mut drawn = vec![opacity as u8];
        drawn.extend(expected);
        assert_eq!(opacities(&mut stage), drawn, "{opacity} {bush_opacity}");
    }
    // RGSS1 halves the opacity and does not look at the bush opacity.
    let mut stage = Stage::sized(32, 32);
    coded(&mut stage, 8, 8, |sprite| {
        sprite.bush_depth = 3;
        sprite.bush_opacity = 300;
    });
    assert_eq!(opacities(&mut stage), [255, 127]);
}

/// The rows of a bitmap of 16 rows that the 36 rows of a sprite at a zoom of 2.25 show
/// with a bush depth of 41 and of 37: topic `bushbig` of the measurement of 2026-10-12 on
/// the original runtime of VX.
const ROWS_AT_41: [u8; 36] = [
    0, 0, 1, 1, 2, 2, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 8, 8, 9, 9, 10, 10, 10, 11, 11, 12, 12,
    13, 13, 14, 14, 14, 15, 15,
];
const ROWS_AT_37: [u8; 36] = [
    0, 0, 1, 1, 2, 2, 3, 3, 3, 4, 4, 5, 5, 6, 6, 6, 7, 7, 8, 8, 9, 9, 9, 10, 10, 11, 11, 12, 12,
    12, 13, 13, 14, 14, 15, 15,
];

/// Scenario "Depth beyond the height, measured on the original".
#[test]
fn a_bush_depth_beyond_the_height_stretches_rows_from_above_the_sprite() {
    for (depth, expected) in [(41, ROWS_AT_41), (37, ROWS_AT_37)] {
        let mut stage = Stage::rgss2(16, 48);
        coded(&mut stage, 8, 16, |sprite| {
            sprite.y = 4;
            sprite.zoom_y = 2.25;
            sprite.bush_depth = depth;
            sprite.bush_opacity = 255;
        });
        let frame = stage.frame();
        let rows: Vec<u8> = (4..40).map(|y| frame.pixel(3, y)[1]).collect();
        assert_eq!(rows, expected, "depth {depth}");
        // Nothing is drawn above the sprite's top or below its bottom.
        assert_eq!(frame.pixel(3, 3), BLACK);
        assert_eq!(frame.pixel(3, 40), BLACK);
    }
}

/// A depth beyond the height of a sprite that is not stretched takes the rows above the
/// source rectangle one to one.
#[test]
fn a_bush_depth_beyond_the_height_of_an_unstretched_sprite_shifts_no_row() {
    let mut stage = Stage::rgss2(16, 16);
    coded(&mut stage, 8, 12, |sprite| {
        sprite.src_rect = Rect::new(0, 4, 8, 6);
        sprite.bush_depth = 9;
        sprite.bush_opacity = 255;
    });
    let frame = stage.frame();
    let rows: Vec<u8> = (0..6).map(|y| frame.pixel(0, y)[1]).collect();
    assert_eq!(rows, [4, 5, 6, 7, 8, 9]);
    assert_eq!(frame.pixel(0, 6), BLACK);
}

/// Scenario "No tone on bush rows, measured on the original", with the mirror.
#[test]
fn the_bush_rows_have_no_tone_no_colour_and_no_mirror() {
    let mut stage = Stage::rgss2(16, 16);
    let id = coded(&mut stage, 8, 8, |sprite| {
        sprite.tone = crate::Tone::new(0.0, 0.0, -200.0, 0.0);
        sprite.color = crate::Color::new(255.0, 255.0, 255.0, 255.0);
        sprite.mirror = true;
        sprite.bush_depth = 2;
        sprite.bush_opacity = 255;
    });
    let frame = stage.frame();
    // The upper rows show the colour, the bush rows the texels where they stand.
    assert_eq!(frame.pixel(1, 5), WHITE);
    assert_eq!(frame.pixel(1, 6), [1, 6, 200, 255]);
    assert_eq!(frame.pixel(6, 7), [6, 7, 200, 255]);
    // Without the colour the upper rows show the tone and the mirror.
    stage.scene.sprite_mut(id).unwrap().color = crate::Color::TRANSPARENT;
    let frame = stage.frame();
    assert_eq!(frame.pixel(1, 5), [6, 5, 0, 255]);
    assert_eq!(frame.pixel(1, 6), [1, 6, 200, 255]);
    // A flash colour does not reach the bush rows either.
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.flash(Some(crate::Color::new(255.0, 0.0, 0.0, 255.0)), 4);
    let frame = stage.frame();
    assert_eq!(frame.pixel(1, 5), RED);
    assert_eq!(frame.pixel(1, 6), [1, 6, 200, 255]);
}

/// A turned sprite keeps its mirror on the bush rows, and loses tone and colour there.
#[test]
fn the_bush_rows_of_a_turned_sprite_keep_the_mirror() {
    let turned = |depth: i32, color: crate::Color| {
        let mut stage = Stage::rgss2(16, 16);
        coded(&mut stage, 8, 8, |sprite| {
            sprite.x = 8;
            sprite.y = 8;
            sprite.angle = 180.0;
            sprite.mirror = true;
            sprite.color = color;
            sprite.bush_depth = depth;
            sprite.bush_opacity = 255;
        });
        stage.frame()
    };
    // At a bush opacity of 255 the bush rows show what the rows show without a depth.
    let plain = turned(0, crate::Color::TRANSPARENT);
    assert_eq!(turned(2, crate::Color::TRANSPARENT).rgba, plain.rgba);
    assert_eq!(plain.pixel(2, 7)[0] + 1, plain.pixel(3, 7)[0]);
    let white = crate::Color::new(255.0, 255.0, 255.0, 255.0);
    let frame = turned(2, white);
    // The columns the turned sprite covers.
    let covered: Vec<u32> = (0..16).filter(|x| plain.pixel(*x, 5) != BLACK).collect();
    assert!(covered.len() >= 6);
    for x in covered {
        assert_eq!(frame.pixel(x, 5), WHITE);
        assert_eq!(frame.pixel(x, 6), plain.pixel(x, 6));
        assert_eq!(frame.pixel(x, 7), plain.pixel(x, 7));
    }
}

/// Scenarios of the measurement for "Bush rows of RGSS2": the topics `bushbig` and
/// `bushop`.
#[test]
fn the_cases_of_bush_rows_are_the_expected_pictures() {
    compare(include_str!("../../../tests/data/sprite-rgss2-bush.txt"), 5);
}
