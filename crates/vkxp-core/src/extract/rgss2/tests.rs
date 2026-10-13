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
