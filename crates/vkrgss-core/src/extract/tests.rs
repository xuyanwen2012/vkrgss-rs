use vkrgss_proto::BitmapId;

use proptest::prelude::*;
use vkrgss_pixel::Rgba;
use vkrgss_proto::{BitmapDelta, FrameRate, Size};

use super::testing::*;
use crate::bitmap::testing::{color, defaults};
use crate::{
    Bitmaps, Color, PlaneId, Rect, Rules, Scene, SharedTable, SpriteId, Table, TilemapId, Tone,
    ViewportId, WindowId,
};

/// A sprite showing `bitmap` at (`x`, `y`).
fn sprite(stage: &mut Stage, bitmap: BitmapId, x: i32, y: i32) -> SpriteId {
    sprite_in(stage, None, bitmap, x, y)
}

fn sprite_in(
    stage: &mut Stage,
    viewport: Option<ViewportId>,
    bitmap: BitmapId,
    x: i32,
    y: i32,
) -> SpriteId {
    let id = stage.scene.create_sprite(viewport).unwrap();
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
    sprite.x = x;
    sprite.y = y;
    id
}

/// Two overlapping opaque sprites, the red one created first. They share (4, 4).
fn red_then_green(stage: &mut Stage) -> (SpriteId, SpriteId) {
    let red = stage.solid(4, 4, RED);
    let green = stage.solid(4, 4, GREEN);
    (sprite(stage, red, 2, 2), sprite(stage, green, 4, 4))
}

#[test]
fn the_larger_z_is_in_front_whichever_was_created_first() {
    let mut stage = Stage::sized(16, 16);
    let (red, green) = red_then_green(&mut stage);
    stage.scene.set_sprite_z(red, 1).unwrap();
    stage.scene.set_sprite_z(green, 2).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(4, 4), GREEN);
    assert_eq!(frame.pixel(2, 2), RED);
    assert_eq!(frame.pixel(7, 7), GREEN);

    let mut stage = Stage::sized(16, 16);
    let (red, green) = red_then_green(&mut stage);
    stage.scene.set_sprite_z(red, 2).unwrap();
    stage.scene.set_sprite_z(green, 1).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(4, 4), RED);
    assert_eq!(frame.pixel(7, 7), GREEN);

    // Negative z is behind z 0.
    stage.scene.set_sprite_z(red, -1).unwrap();
    stage.scene.set_sprite_z(green, 0).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
}

#[test]
fn with_equal_z_the_later_created_is_in_front() {
    let mut stage = Stage::sized(16, 16);
    red_then_green(&mut stage);
    assert_eq!(stage.frame().pixel(4, 4), GREEN);

    let mut stage = Stage::sized(16, 16);
    let (red, green) = red_then_green(&mut stage);
    stage.scene.set_sprite_z(green, 5).unwrap();
    stage.scene.set_sprite_z(red, 5).unwrap();
    // Red was the last to be given a z, which does not count.
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
}

#[test]
fn moving_to_an_occupied_z_keeps_the_order_of_creation() {
    let mut stage = Stage::sized(16, 16);
    let (red, _) = red_then_green(&mut stage);
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
    stage.scene.set_sprite_z(red, 7).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), RED);
    stage.scene.set_sprite_z(red, 0).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
}

#[test]
fn setting_the_same_z_again_changes_nothing() {
    let mut stage = Stage::sized(16, 16);
    let (red, _) = red_then_green(&mut stage);
    stage.scene.set_sprite_z(red, 0).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
    stage.scene.set_sprite_z(red, 0).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
}

#[test]
fn viewports_take_part_in_the_order_of_the_screen() {
    // A red viewport sprite and a green screen sprite share (4, 4).
    let mut stage = Stage::sized(16, 16);
    let red = stage.solid(4, 4, RED);
    let green = stage.solid(4, 4, GREEN);
    let viewport = stage
        .scene
        .create_viewport(Rect::new(0, 0, 16, 16))
        .unwrap();
    sprite_in(&mut stage, Some(viewport), red, 2, 2);
    sprite(&mut stage, green, 4, 4);
    assert_eq!(stage.frame().pixel(4, 4), GREEN, "equal z: created later");

    stage.scene.set_viewport_z(viewport, 1).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), RED, "larger z");

    stage.scene.set_viewport_z(viewport, 0).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), GREEN, "back at the shared z");
}

#[test]
fn a_viewport_older_than_a_screen_sprite_is_behind_it() {
    // The viewport is created first, the green screen sprite next, the red sprite of the
    // viewport last: the viewport's creation counts, not that of its sprite.
    let mut stage = Stage::sized(16, 16);
    let red = stage.solid(4, 4, RED);
    let green = stage.solid(4, 4, GREEN);
    let viewport = stage
        .scene
        .create_viewport(Rect::new(0, 0, 16, 16))
        .unwrap();
    sprite(&mut stage, green, 4, 4);
    sprite_in(&mut stage, Some(viewport), red, 2, 2);
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
}

#[test]
fn elements_inside_a_viewport_are_ordered_among_themselves() {
    let mut stage = Stage::sized(16, 16);
    let viewport = stage
        .scene
        .create_viewport(Rect::new(0, 0, 16, 16))
        .unwrap();
    let red = stage.solid(4, 4, RED);
    let green = stage.solid(4, 4, GREEN);
    let red = sprite_in(&mut stage, Some(viewport), red, 2, 2);
    let green = sprite_in(&mut stage, Some(viewport), green, 4, 4);
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
    stage.scene.set_sprite_z(red, 3).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), RED);
    stage.scene.set_sprite_z(green, 3).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
    stage.scene.set_sprite_z(green, 3).unwrap();
    stage.scene.set_sprite_z(red, 3).unwrap();
    assert_eq!(
        stage.frame().pixel(4, 4),
        GREEN,
        "same z again changes nothing"
    );
}

/// The opaque pixel a [`grid`] bitmap has at (`x`, `y`).
fn cell(x: u32, y: u32) -> Rgba {
    [10 + 40 * x as u8, 10 + 40 * y as u8, 77, 255]
}

/// An opaque bitmap whose every pixel differs.
fn grid(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
    let pixels: Vec<Rgba> = (0..height)
        .flat_map(|y| (0..width).map(move |x| cell(x, y)))
        .collect();
    stage.pixels(width, height, &pixels)
}

const A: Rgba = [200, 10, 10, 255];
const B: Rgba = [10, 10, 200, 255];

/// The sprite of the zoom, rotation and mirror scenarios: a 2 by 1 bitmap holding A and
/// B, at (10, 10).
fn a_and_b(stage: &mut Stage) -> SpriteId {
    let bitmap = stage.pixels(2, 1, &[A, B]);
    sprite(stage, bitmap, 10, 10)
}

/// The pixels of the frame that are not black, as (x, y, pixel).
fn drawn(frame: &vkrgss_compose_ref::Frame) -> Vec<(u32, u32, Rgba)> {
    let mut found = Vec::new();
    for y in 0..frame.size.height {
        for x in 0..frame.size.width {
            if frame.pixel(x, y) != BLACK {
                found.push((x, y, frame.pixel(x, y)));
            }
        }
    }
    found
}

#[test]
fn the_origin_of_a_sprite_lands_on_its_position() {
    let mut stage = Stage::sized(32, 32);
    let bitmap = grid(&mut stage, 4, 4);
    let id = sprite(&mut stage, bitmap, 10, 20);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.ox = 1;
    sprite.oy = 1;
    let frame = stage.frame();
    assert_eq!(frame.pixel(10, 20), cell(1, 1));
    assert_eq!(frame.pixel(9, 19), cell(0, 0));
    assert_eq!(frame.pixel(12, 22), cell(3, 3));
    assert_eq!(frame.pixel(8, 19), BLACK);
    assert_eq!(frame.pixel(13, 22), BLACK);
    assert_eq!(drawn(&frame).len(), 16, "one bitmap pixel per screen pixel");
}

#[test]
fn assigning_a_bitmap_sets_the_source_rectangle_to_all_of_it() {
    let mut stage = Stage::sized(16, 16);
    let small = stage.solid(4, 3, RED);
    let large = stage.solid(9, 7, GREEN);
    let id = sprite(&mut stage, small, 0, 0);
    assert_eq!(stage.scene.sprite(id).unwrap().bitmap(), Some(small));
    assert_eq!(
        stage.scene.sprite(id).unwrap().src_rect,
        Rect::new(0, 0, 4, 3)
    );
    stage.scene.sprite_mut(id).unwrap().src_rect = Rect::new(1, 1, 2, 2);
    stage
        .scene
        .sprite_mut(id)
        .unwrap()
        .set_bitmap(Some(large), &stage.bitmaps);
    assert_eq!(
        stage.scene.sprite(id).unwrap().src_rect,
        Rect::new(0, 0, 9, 7)
    );
    assert_eq!(drawn(&stage.frame()).len(), 63);
    // Taking the bitmap away keeps the rectangle.
    stage
        .scene
        .sprite_mut(id)
        .unwrap()
        .set_bitmap(None, &stage.bitmaps);
    assert_eq!(stage.scene.sprite(id).unwrap().bitmap(), None);
    assert_eq!(
        stage.scene.sprite(id).unwrap().src_rect,
        Rect::new(0, 0, 9, 7)
    );
}

#[test]
fn the_source_rectangle_selects_what_is_drawn() {
    let mut stage = Stage::sized(32, 32);
    let bitmap = grid(&mut stage, 4, 4);
    let id = sprite(&mut stage, bitmap, 10, 20);
    stage.scene.sprite_mut(id).unwrap().src_rect = Rect::new(2, 0, 2, 4);
    let frame = stage.frame();
    assert_eq!(frame.pixel(10, 20), cell(2, 0));
    assert_eq!(frame.pixel(11, 23), cell(3, 3));
    assert_eq!(drawn(&frame).len(), 8, "columns 0 and 1 are not drawn");
    assert_eq!(frame.pixel(9, 20), BLACK);
    assert_eq!(frame.pixel(12, 20), BLACK);
}

#[test]
fn the_part_of_the_source_rectangle_outside_the_bitmap_draws_nothing() {
    let mut stage = Stage::sized(32, 32);
    let bitmap = grid(&mut stage, 4, 4);
    let id = sprite(&mut stage, bitmap, 10, 20);
    stage.scene.sprite_mut(id).unwrap().src_rect = Rect::new(-2, 0, 4, 4);
    let frame = stage.frame();
    assert_eq!(frame.pixel(12, 20), cell(0, 0));
    assert_eq!(frame.pixel(13, 23), cell(1, 3));
    assert_eq!(frame.pixel(10, 20), BLACK);
    assert_eq!(frame.pixel(11, 20), BLACK);
    assert_eq!(frame.pixel(14, 20), BLACK);
    assert_eq!(drawn(&frame).len(), 8);

    // Beyond the right and bottom edges, and with an origin.
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.src_rect = Rect::new(3, 2, 10, 10);
    sprite.ox = 1;
    let frame = stage.frame();
    assert_eq!(frame.pixel(9, 20), cell(3, 2));
    assert_eq!(frame.pixel(9, 21), cell(3, 3));
    assert_eq!(drawn(&frame).len(), 2);

    // Mirrored, the rectangle flips in place: its empty left half ends up on the right.
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.src_rect = Rect::new(-2, 0, 4, 4);
    sprite.ox = 0;
    sprite.mirror = true;
    let frame = stage.frame();
    assert_eq!(frame.pixel(10, 20), cell(1, 0));
    assert_eq!(frame.pixel(11, 20), cell(0, 0));
    assert_eq!(frame.pixel(12, 20), BLACK);
    assert_eq!(drawn(&frame).len(), 8);

    // Entirely outside, and without pixels.
    for rect in [
        Rect::new(4, 0, 4, 4),
        Rect::new(0, -4, 4, 4),
        Rect::new(0, 0, 0, 4),
        Rect::new(0, 0, 4, -1),
    ] {
        stage.scene.sprite_mut(id).unwrap().src_rect = rect;
        assert!(all(&stage.frame(), BLACK), "{rect:?}");
    }
}

#[test]
fn a_sprite_without_a_live_bitmap_draws_nothing() {
    let mut stage = Stage::sized(16, 16);
    let id = stage.scene.create_sprite(None).unwrap();
    assert!(all(&stage.frame(), BLACK), "no bitmap");

    let bitmap = stage.solid(4, 4, RED);
    stage
        .scene
        .sprite_mut(id)
        .unwrap()
        .set_bitmap(Some(bitmap), &stage.bitmaps);
    assert_eq!(stage.frame().pixel(0, 0), RED);
    stage.bitmaps.dispose(bitmap);
    assert!(all(&stage.frame(), BLACK), "disposed bitmap");

    // Assigning a disposed bitmap is no error either.
    stage
        .scene
        .sprite_mut(id)
        .unwrap()
        .set_bitmap(Some(bitmap), &stage.bitmaps);
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn an_invisible_or_disposed_sprite_is_not_drawn() {
    let mut stage = Stage::sized(16, 16);
    let bitmap = stage.solid(4, 4, RED);
    let id = sprite(&mut stage, bitmap, 0, 0);
    assert_eq!(stage.frame().pixel(0, 0), RED);
    stage.scene.sprite_mut(id).unwrap().visible = false;
    assert!(all(&stage.frame(), BLACK));
    stage.scene.sprite_mut(id).unwrap().visible = true;
    assert_eq!(stage.frame().pixel(0, 0), RED);
    stage.scene.dispose_sprite(id);
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn zoom_scales_a_sprite_about_its_origin() {
    let mut stage = Stage::sized(32, 32);
    let id = a_and_b(&mut stage);
    stage.scene.sprite_mut(id).unwrap().zoom_x = 2.0;
    let frame = stage.frame();
    assert_eq!(
        drawn(&frame),
        [(10, 10, A), (11, 10, A), (12, 10, B), (13, 10, B)]
    );

    // Independently in y, and about an origin that is not the corner.
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.zoom_x = 1.0;
    sprite.zoom_y = 3.0;
    sprite.ox = 1;
    let frame = stage.frame();
    assert_eq!(
        drawn(&frame),
        [
            (9, 10, A),
            (10, 10, B),
            (9, 11, A),
            (10, 11, B),
            (9, 12, A),
            (10, 12, B)
        ]
    );
}

#[test]
fn a_zoom_of_zero_draws_nothing() {
    let mut stage = Stage::sized(32, 32);
    let id = a_and_b(&mut stage);
    stage.scene.sprite_mut(id).unwrap().zoom_x = 0.0;
    assert!(all(&stage.frame(), BLACK));
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.zoom_x = 1.0;
    sprite.zoom_y = 0.0;
    assert!(all(&stage.frame(), BLACK));
    stage.scene.sprite_mut(id).unwrap().zoom_y = f64::NAN;
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn a_negative_zoom_draws_nothing_of_an_unrotated_sprite() {
    let mut stage = Stage::sized(32, 32);
    let id = a_and_b(&mut stage);
    for (zoom_x, zoom_y) in [(-1.0, 1.0), (1.0, -2.0), (-1.0, -1.0), (0.4, 1.0)] {
        let sprite = stage.scene.sprite_mut(id).unwrap();
        (sprite.zoom_x, sprite.zoom_y) = (zoom_x, zoom_y);
        // The last: two columns at 0.4 are less than one pixel.
        assert!(all(&stage.frame(), BLACK), "{zoom_x} by {zoom_y}");
    }
}

/// The pixel at a whole offset from the position shows the texel that offset turns to.
/// At right angles every position lies on a texel boundary, where the original runtime's
/// sine of pi and cosine of pi / 2, which are not quite 0, decide between two texels:
/// some texels are lost and others shown twice. The lists are what the measured rule
/// gives, each pixel with the texel it shows.
#[test]
fn the_angle_turns_a_sprite_counter_clockwise_about_its_origin() {
    let mut stage = Stage::sized(32, 32);
    let bitmap = grid(&mut stage, 4, 3);
    let id = sprite(&mut stage, bitmap, 10, 10);
    let mut shown = |angle: f64, zoom_x: f64| -> Vec<(u32, u32, (u32, u32))> {
        let sprite = stage.scene.sprite_mut(id).unwrap();
        (sprite.angle, sprite.zoom_x) = (angle, zoom_x);
        let texel = |pixel: Rgba| {
            let found = (0..4).flat_map(|x| (0..3).map(move |y| (x, y)));
            found
                .into_iter()
                .find(|&(x, y)| cell(x, y) == pixel)
                .unwrap()
        };
        let pixels = drawn(&stage.frame());
        pixels
            .iter()
            .map(|&(x, y, pixel)| (x, y, texel(pixel)))
            .collect()
    };
    // The columns of the bitmap run up the screen.
    assert_eq!(
        shown(90.0, 1.0),
        [
            (11, 7, (3, 0)),
            (12, 7, (3, 1)),
            (11, 8, (2, 0)),
            (12, 8, (2, 2)),
            (11, 9, (1, 0)),
            (12, 9, (1, 2))
        ]
    );
    // Upside down, left of and above the position.
    assert_eq!(
        shown(180.0, 1.0),
        [
            (7, 7, (3, 2)),
            (8, 7, (2, 2)),
            (9, 7, (1, 2)),
            (7, 8, (3, 1)),
            (8, 8, (2, 2)),
            (9, 8, (1, 2)),
            (7, 9, (3, 0)),
            (8, 9, (2, 0)),
            (9, 9, (1, 0))
        ]
    );
    // The columns of the bitmap run down the screen.
    assert_eq!(
        shown(270.0, 1.0),
        [
            (8, 10, (0, 2)),
            (9, 10, (0, 1)),
            (8, 11, (1, 2)),
            (9, 11, (1, 1)),
            (7, 12, (2, 2)),
            (8, 12, (2, 1)),
            (9, 12, (2, 0)),
            (7, 13, (3, 2)),
            (8, 13, (3, 1)),
            (9, 13, (3, 0))
        ]
    );
    // Turned and zoomed: twice as far up the screen.
    let zoomed = shown(90.0, 2.0);
    assert_eq!(zoomed.len(), 14);
    assert_eq!(zoomed[0], (11, 3, (3, 0)));
    assert_eq!(zoomed[13], (12, 9, (0, 2)));
}

/// One line of `sprite-zoom-samples.txt`: a sprite 16 pixels wide, and the source column
/// the original runtime showed at each screen pixel it covered.
#[test]
fn unrotated_sprites_show_the_measured_columns() {
    let column = |x: u32| [x as u8 * 10 + 5, 200, 30, 255];
    let samples = include_str!("../../../vkrgss-compose-ref/tests/data/sprite-zoom-samples.txt");
    let mut checked = 0;
    for line in samples.lines().filter(|line| !line.starts_with('#')) {
        let parts: Vec<&str> = line.split('|').collect();
        let given: Vec<&str> = parts[0].split_whitespace().collect();
        let mut stage = Stage::sized(640, 3);
        let bitmap = stage.pixels(16, 1, &(0..16).map(column).collect::<Vec<_>>());
        let id = sprite(&mut stage, bitmap, given[1].parse().unwrap(), 1);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.zoom_x = given[0].parse().unwrap();
        sprite.ox = given[2].parse().unwrap();
        sprite.mirror = given[3] == "1";
        let frame = stage.frame();

        let mut expected = Vec::new();
        if parts[1].trim() != "none" {
            let first: u32 = parts[1].split_whitespace().next().unwrap().parse().unwrap();
            for (i, shown) in parts[2].split_whitespace().enumerate() {
                expected.push((first + i as u32, 1, column(shown.parse().unwrap())));
            }
        }
        assert_eq!(drawn(&frame), expected, "{line}");
        checked += 1;
    }
    assert!(checked > 60);
}

/// The rows of a sprite follow the rule of its columns, and both count from the corner
/// of the source rectangle.
#[test]
fn unrotated_sprites_stretch_their_source_rectangle_along_both_axes() {
    let mut stage = Stage::sized(64, 64);
    let bitmap = grid(&mut stage, 6, 6);
    let id = sprite(&mut stage, bitmap, 20, 30);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.src_rect = Rect::new(1, 2, 5, 4);
    (sprite.ox, sprite.oy) = (3, 1);
    (sprite.zoom_x, sprite.zoom_y) = (2.3, 3.7);
    sprite.mirror = true;
    // 5 * 2.3 = 11.5 and 4 * 3.7 = 14.8; 20 - 3 * 2.3 = 13.1 and 30 - 3.7 = 26.3.
    let (width, height, left, top) = (11u32, 14u32, 13u32, 26u32);
    let (step_x, step_y) = ((5u64 << 20) / 11, (4u64 << 20) / 14);
    let frame = stage.frame();
    let mut expected = Vec::new();
    for j in 0..height {
        for i in 0..width {
            let u = ((u64::from(i) * step_x) >> 20) as u32;
            let v = ((u64::from(j) * step_y) >> 20) as u32;
            expected.push((left + i, top + j, cell(1 + 4 - u, 2 + v)));
        }
    }
    assert_eq!(drawn(&frame), expected);
}

#[test]
fn a_viewport_moves_an_unrotated_sprite_by_whole_pixels() {
    let mut stage = Stage::sized(64, 32);
    let bitmap = grid(&mut stage, 6, 2);
    let viewport = stage
        .scene
        .create_viewport(Rect::new(5, 4, 40, 20))
        .unwrap();
    stage.scene.viewport_mut(viewport).unwrap().ox = -3;
    let id = sprite_in(&mut stage, Some(viewport), bitmap, 2, 1);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.zoom_x = 1.5;
    sprite.ox = 1;
    // floor(2 - 1.5) = 0 inside the viewport, which begins at 5 and is scrolled by 3.
    let step = (6u64 << 20) / 9;
    let expected: Vec<_> = (0..2)
        .flat_map(|j| {
            (0..9u32).map(move |i| {
                let u = ((u64::from(i) * step) >> 20) as u32;
                (8 + i, 5 + j, cell(u, j))
            })
        })
        .collect();
    assert_eq!(drawn(&stage.frame()), expected);
}

/// A zoomed sprite whose source rectangle reaches outside its bitmap keeps its place and
/// shows only texels of the bitmap, in the columns the original runtime draws of a cut
/// rectangle: from column `a * dw / w` on, up to column `(b * dw - 1) / w`.
#[test]
fn a_zoomed_sprite_keeps_its_place_when_its_source_rectangle_is_cut() {
    for mirror in [false, true] {
        let mut stage = Stage::sized(64, 16);
        let bitmap = grid(&mut stage, 4, 2);
        let id = sprite(&mut stage, bitmap, 10, 3);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        // Two columns left of the bitmap, its four columns, two columns right of it.
        sprite.src_rect = Rect::new(-2, 0, 8, 2);
        sprite.zoom_x = 3.0;
        sprite.mirror = mirror;
        let pixels = drawn(&stage.frame());
        // 24 pixels for 8 columns: column 2, the first of the bitmap, begins at the 6th
        // pixel, 2 * 24 / 8, and the last pixel drawn is the 16th, (6 * 24 - 1) / 8 - 1.
        let row: Vec<(u32, Rgba)> = pixels
            .iter()
            .filter(|pixel| pixel.1 == 3)
            .map(|pixel| (pixel.0, pixel.2))
            .collect();
        let columns: Vec<u32> = row.iter().map(|pixel| pixel.0).collect();
        assert_eq!(columns, (16..27).collect::<Vec<_>>(), "mirror {mirror}");
        let (first, last) = if mirror { (3, 0) } else { (0, 3) };
        assert_eq!(row[0].1, cell(first, 0));
        assert_eq!(row[10].1, cell(last, 0));
        assert_eq!(pixels.len(), 22);
    }
}

/// The lines of `sprite-rotation-samples.txt`: a turned sprite of 200 by 150 at
/// (320, 240), and the texel the original runtime showed at offsets from there.
#[test]
fn rotated_sprites_show_the_measured_texels() {
    let samples = include_str!("../../tests/data/sprite-rotation-samples.txt");
    let lines: Vec<&str> = samples
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    let mut stage = Stage::new();
    let mut rgba = Vec::new();
    for y in 0..150u8 {
        for x in 0..200u8 {
            rgba.extend_from_slice(&[x, y, 100, 255]);
        }
    }
    let bitmap = stage.bitmap(200, 150, rgba);
    let id = sprite(&mut stage, bitmap, 320, 240);
    let mut checked = (0, 0);
    // Lines of one placement follow each other.
    for group in lines.chunk_by(|a, b| a.split('|').next() == b.split('|').next()) {
        let given: Vec<f64> = group[0]
            .split('|')
            .next()
            .unwrap()
            .split_whitespace()
            .map(|word| word.parse().unwrap())
            .collect();
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.angle = given[0];
        (sprite.ox, sprite.oy) = (given[1] as i32, given[2] as i32);
        (sprite.zoom_x, sprite.zoom_y) = (given[3], given[4]);
        sprite.mirror = given[5] == 1.0;
        let frame = stage.frame();
        for line in group {
            let (offset, shown) = line.split('|').nth(1).unwrap().split_once("->").unwrap();
            let offset: Vec<i32> = offset
                .split_whitespace()
                .map(|word| word.parse().unwrap())
                .collect();
            let texel: Vec<u8> = shown
                .split_whitespace()
                .map_while(|word| word.parse().ok())
                .collect();
            let expected = match texel.as_slice() {
                [u, v] => {
                    checked.0 += 1;
                    [*u, *v, 100, 255]
                }
                _ => {
                    checked.1 += 1;
                    BLACK
                }
            };
            let pixel = frame.pixel((320 + offset[0]) as u32, (240 + offset[1]) as u32);
            assert_eq!(pixel, expected, "{line}");
        }
    }
    // Covered pixels and pixels beside the sprite.
    assert!(checked.0 > 30 && checked.1 > 200, "{checked:?}");
}

/// The bitmaps of the sprite angle probe: texel (x, y) holds its own coordinates.
fn probe_bitmap(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let high = 100 + (x >> 8) + 2 * (y >> 8);
            rgba.extend_from_slice(&[x as u8, y as u8, high as u8, 255]);
        }
    }
    stage.bitmap(width, height, rgba)
}

fn fields(text: &str) -> Vec<f64> {
    text.split_whitespace()
        .map(|word| word.parse().unwrap())
        .collect()
}

/// The frames of `sprite-angle-frames.txt`: right angles, full turns, generic angles,
/// zoom, mirror, a source rectangle and sprites partly off screen. Every frame is
/// compared as a whole, by the number of pixels the sprite covers and by a hash, and
/// pixel by pixel in the committed crops.
#[test]
fn rotated_sprites_equal_the_measured_frames() {
    let data = include_str!("../../tests/data/sprite-angle-frames.txt");
    let mut lines = data
        .lines()
        .filter(|line| !line.starts_with('#'))
        .peekable();
    let mut stage = Stage::new();
    let bitmaps = [
        probe_bitmap(&mut stage, 200, 150),
        probe_bitmap(&mut stage, 329, 329),
    ];
    let id = stage.scene.create_sprite(None).unwrap();
    let mut cases = 0;
    while let Some(case) = lines.next() {
        let parts: Vec<&str> = case.strip_prefix("case ").unwrap().split('|').collect();
        let (size, given, src) = (fields(parts[0]), fields(parts[1]), fields(parts[2]));
        let covered: usize = parts[3].trim().parse().unwrap();
        let hash = u64::from_str_radix(parts[4].trim(), 16).unwrap();
        let bitmap = bitmaps[usize::from(size[0] != 200.0)];
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
        sprite.src_rect = Rect::new(src[0] as i32, src[1] as i32, src[2] as i32, src[3] as i32);
        sprite.angle = given[0];
        (sprite.x, sprite.y) = (given[1] as i32, given[2] as i32);
        (sprite.ox, sprite.oy) = (given[3] as i32, given[4] as i32);
        (sprite.zoom_x, sprite.zoom_y) = (given[5], given[6]);
        sprite.mirror = given[7] == 1.0;
        let frame = stage.frame();

        while let Some(crop) = lines.next_if(|line| line.starts_with("crop ")) {
            let at = fields(crop.strip_prefix("crop ").unwrap());
            for row in 0..at[3] as u32 {
                let line = lines.next().unwrap();
                for column in 0..at[2] as u32 {
                    let hex = &line[column as usize * 6..][..6];
                    let rgb = u32::from_str_radix(hex, 16).unwrap().to_be_bytes();
                    let (x, y) = (at[0] as u32 + column, at[1] as u32 + row);
                    assert_eq!(
                        frame.pixel(x, y),
                        [rgb[1], rgb[2], rgb[3], 255],
                        "({x}, {y}) of {case}"
                    );
                }
            }
        }
        assert_eq!(drawn(&frame).len(), covered, "{case}");
        let mut found = 0xcbf2_9ce4_8422_2325u64;
        for pixel in frame.rgba.as_chunks::<4>().0 {
            for byte in &pixel[..3] {
                found = (found ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
            }
        }
        assert_eq!(found, hash, "{case}");
        cases += 1;
    }
    assert_eq!(cases, 36);
}

/// A half turn about (100, 75) at (320, 240): the position shows the texel above the
/// origin, and the last column and row of the box are missing.
#[test]
fn a_half_turn_shows_what_was_measured() {
    let mut stage = Stage::new();
    let bitmap = probe_bitmap(&mut stage, 200, 150);
    let id = sprite(&mut stage, bitmap, 320, 240);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    (sprite.ox, sprite.oy) = (100, 75);
    sprite.angle = 180.0;
    let frame = stage.frame();
    assert_eq!(frame.pixel(320, 240), [100, 74, 100, 255]);
    assert_eq!(drawn(&frame).len(), 29651);
}

/// A full turn is drawn by the rule of turned sprites, not as a sprite that is not
/// turned: the last column and the last row of its box are missing.
#[test]
fn a_full_turn_is_drawn_as_a_turned_sprite() {
    let mut stage = Stage::sized(32, 32);
    let bitmap = grid(&mut stage, 6, 4);
    let id = sprite(&mut stage, bitmap, 10, 10);
    let upright = drawn(&stage.frame());
    assert_eq!(upright.len(), 24);
    for angle in [360.0, -360.0, 720.0] {
        stage.scene.sprite_mut(id).unwrap().angle = angle;
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let [vkrgss_proto::Op::Quad(quad)] = packet.ops.as_slice() else {
            panic!("one quad");
        };
        assert_ne!(quad.bitmap, bitmap, "angle {angle}");
        let pixels = drawn(&stage.render(&packet).unwrap());
        assert!(!pixels.is_empty() && pixels != upright, "angle {angle}");
        // The box ends before column 16 and row 14.
        assert!(
            pixels
                .iter()
                .all(|pixel| (10..16).contains(&pixel.0) && (10..14).contains(&pixel.1)),
            "angle {angle}: {pixels:?}"
        );
    }
}

/// The bitmap of a turned sprite is rendered again only when what it shows changes, and
/// is given back when the sprite is no longer turned or is disposed.
#[test]
fn the_bitmap_of_a_turned_sprite_is_kept_while_nothing_changes() {
    let mut stage = Stage::sized(64, 64);
    let bitmap = grid(&mut stage, 6, 4);
    let id = sprite(&mut stage, bitmap, 30, 30);
    assert_eq!(stage.bitmaps.len(), 1);
    stage.scene.sprite_mut(id).unwrap().angle = 30.0;
    let first = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), 2);
    // Only the rendered bitmap is uploaded.
    assert!(!delta_ids(&first.bitmaps).is_empty());
    assert!(
        delta_ids(&first.bitmaps)
            .iter()
            .all(|delta| delta.1 != bitmap)
    );

    // Unchanged, and moved: nothing is uploaded.
    assert!(stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
    let sprite = stage.scene.sprite_mut(id).unwrap();
    (sprite.x, sprite.y) = (20, 25);
    sprite.set_opacity(100);
    assert!(stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());

    // A changed source and a changed angle are rendered again.
    stage
        .bitmaps
        .set_pixel(bitmap, 0, 0, Color::new(1.0, 2.0, 3.0, 255.0))
        .unwrap();
    assert!(!stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
    stage.scene.sprite_mut(id).unwrap().angle = 31.0;
    assert!(!stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
    assert_eq!(stage.bitmaps.len(), 2);

    stage.scene.sprite_mut(id).unwrap().angle = 0.0;
    stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), 1);

    stage.scene.sprite_mut(id).unwrap().angle = 30.0;
    stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), 2);
    stage.scene.dispose_sprite(id);
    stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), 1);
}

/// A stretched sprite whose source rectangle leaves its bitmap is drawn from a bitmap of
/// its own, which is kept while nothing changes and given back when it is needed no more.
#[test]
fn the_bitmap_of_a_cut_sprite_is_kept_while_nothing_changes() {
    let mut stage = Stage::sized(64, 64);
    let bitmap = grid(&mut stage, 6, 4);
    let id = sprite(&mut stage, bitmap, 30, 30);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.src_rect = Rect::new(-2, 0, 6, 4);
    stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), 1, "not stretched");

    stage.scene.sprite_mut(id).unwrap().zoom_x = 2.0;
    assert!(!stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
    assert_eq!(stage.bitmaps.len(), 2);
    // Unchanged, and moved: nothing is uploaded.
    assert!(stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
    let sprite = stage.scene.sprite_mut(id).unwrap();
    (sprite.x, sprite.y) = (20, 25);
    assert!(stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
    // The faint rows are a stretch of their own.
    stage.scene.sprite_mut(id).unwrap().bush_depth = 2;
    assert!(!stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
    assert_eq!(stage.bitmaps.len(), 2);

    // Turned, the sprite is not drawn, and inside its bitmap it is drawn from that.
    stage.scene.sprite_mut(id).unwrap().angle = 30.0;
    assert!(stage.scene.extract(&mut stage.bitmaps).ops.is_empty());
    assert_eq!(stage.bitmaps.len(), 1);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.angle = 0.0;
    stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), 2);
    stage.scene.sprite_mut(id).unwrap().src_rect = Rect::new(0, 0, 6, 4);
    assert_eq!(stage.scene.extract(&mut stage.bitmaps).ops.len(), 2);
    assert_eq!(stage.bitmaps.len(), 1);
}

#[test]
fn mirroring_flips_a_sprite_in_place() {
    let mut stage = Stage::sized(32, 32);
    let id = a_and_b(&mut stage);
    stage.scene.sprite_mut(id).unwrap().mirror = true;
    assert_eq!(drawn(&stage.frame()), [(10, 10, B), (11, 10, A)]);
}

#[test]
fn a_sprite_draws_wherever_it_is_turned_at_any_angle() {
    // Not exact, but a turned square still covers about its area around its centre.
    let mut stage = Stage::sized(64, 64);
    let bitmap = stage.solid(16, 16, RED);
    let id = sprite(&mut stage, bitmap, 32, 32);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.ox = 8;
    sprite.oy = 8;
    for angle in [1.0, 33.3, 45.0, 123.0, -77.5, 359.0] {
        stage.scene.sprite_mut(id).unwrap().angle = angle;
        let frame = stage.frame();
        let count = drawn(&frame).len();
        assert!(
            (236..=276).contains(&count),
            "angle {angle}: {count} pixels"
        );
        assert_eq!(frame.pixel(32, 32), RED);
        assert_eq!(frame.pixel(31, 31), RED);
    }
}

/// A white 32 by 32 sprite at the top-left corner of a black stage, and its faint white.
fn bush(stage: &mut Stage, depth: i32) -> (SpriteId, Rgba) {
    let bitmap = stage.solid(32, 32, WHITE);
    let id = sprite(stage, bitmap, 0, 0);
    stage.scene.sprite_mut(id).unwrap().bush_depth = depth;
    (id, vkrgss_pixel::blend_normal(BLACK, WHITE, 127))
}

/// Whether the rows of the frame from `top` up to `bottom` are `pixel` in their first 32
/// columns.
fn rows(frame: &vkrgss_compose_ref::Frame, top: u32, bottom: u32, pixel: Rgba) -> bool {
    (top..bottom).all(|y| (0..32).all(|x| frame.pixel(x, y) == pixel))
}

#[test]
fn the_rows_of_the_bush_depth_are_drawn_with_half_the_opacity() {
    let mut stage = Stage::sized(48, 48);
    let (id, faint) = bush(&mut stage, 8);
    let frame = stage.frame();
    assert!(rows(&frame, 0, 24, WHITE));
    assert!(rows(&frame, 24, 32, faint));
    assert!(rows(&frame, 32, 48, BLACK));
    assert!((120..=135).contains(&faint[0]), "mid grey");

    // Half of the sprite's own opacity, rounded down.
    stage.scene.sprite_mut(id).unwrap().set_opacity(101);
    let frame = stage.frame();
    assert_eq!(
        frame.pixel(0, 0),
        vkrgss_pixel::blend_normal(BLACK, WHITE, 101)
    );
    assert_eq!(
        frame.pixel(0, 31),
        vkrgss_pixel::blend_normal(BLACK, WHITE, 50)
    );
}

#[test]
fn the_bush_depth_counts_screen_rows_whatever_the_zoom() {
    let mut stage = Stage::sized(48, 80);
    let (id, faint) = bush(&mut stage, 8);
    stage.scene.sprite_mut(id).unwrap().zoom_y = 2.0;
    let frame = stage.frame();
    assert!(rows(&frame, 0, 56, WHITE));
    assert!(rows(&frame, 56, 64, faint));
    assert!(rows(&frame, 64, 80, BLACK));

    // Zoomed out below the bush depth: every row is faint.
    stage.scene.sprite_mut(id).unwrap().zoom_y = 0.2;
    let frame = stage.frame();
    assert!(rows(&frame, 0, 6, faint));
    assert!(rows(&frame, 6, 80, BLACK));
}

#[test]
fn a_bush_depth_of_zero_or_less_changes_nothing() {
    let mut stage = Stage::sized(48, 48);
    let bitmap = stage.solid(32, 32, WHITE);
    sprite(&mut stage, bitmap, 0, 0);
    let plain = stage.scene.extract(&mut stage.bitmaps);
    let plain_frame = stage.render(&plain).unwrap();
    for depth in [0, -8, i32::MIN] {
        let mut stage = Stage::sized(48, 48);
        bush(&mut stage, depth);
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(packet.ops, plain.ops, "depth {depth}");
        assert_eq!(stage.render(&packet).unwrap().rgba, plain_frame.rgba);
    }
}

#[test]
fn a_bush_depth_beyond_the_source_height_covers_the_whole_sprite() {
    for depth in [32, 33, i32::MAX] {
        let mut stage = Stage::sized(48, 48);
        let (_, faint) = bush(&mut stage, depth);
        let frame = stage.frame();
        assert!(rows(&frame, 0, 32, faint), "depth {depth}");
        assert!(rows(&frame, 32, 48, BLACK));
    }
}

/// The lowest rows of the box of a turned sprite are the faint ones, whatever texels they
/// show: after a half turn those are the top rows of the source.
#[test]
fn the_bush_depth_of_a_turned_sprite_counts_rows_of_its_box() {
    let mut stage = Stage::sized(64, 64);
    let bitmap = grid(&mut stage, 6, 6);
    let id = sprite(&mut stage, bitmap, 30, 30);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    (sprite.zoom_x, sprite.zoom_y) = (3.0, 2.0);
    sprite.angle = 180.0;
    let plain = drawn(&stage.frame());
    stage.scene.sprite_mut(id).unwrap().bush_depth = 3;
    let bushy = drawn(&stage.frame());
    assert!(plain.len() > 150);
    assert_eq!(plain.len(), bushy.len());
    // The box ends at the position of the sprite, row 30, which is not drawn.
    assert_eq!(plain.iter().map(|pixel| pixel.1).max(), Some(29));
    let top_rows: Vec<Rgba> = (0..2)
        .flat_map(|y| (0..6).map(move |x| cell(x, y)))
        .collect();
    let mut faint = 0;
    for (full, found) in plain.iter().zip(&bushy) {
        let pixel = full.2;
        let expected = if full.1 >= 27 {
            faint += 1;
            assert!(top_rows.contains(&pixel), "{full:?}");
            vkrgss_pixel::blend_normal(BLACK, pixel, 127)
        } else {
            pixel
        };
        assert_eq!(*found, (full.0, full.1, expected));
    }
    assert!(faint > 30, "{faint}");
}

#[test]
fn opacity_scales_the_alpha_of_a_sprite() {
    let mut stage = Stage::sized(16, 16);
    let bitmap = stage.solid(4, 4, WHITE);
    let id = sprite(&mut stage, bitmap, 0, 0);
    for opacity in [255, 128, 1] {
        stage.scene.sprite_mut(id).unwrap().set_opacity(opacity);
        let expected = vkrgss_pixel::blend_normal(BLACK, WHITE, opacity as u8);
        assert_eq!(stage.frame().pixel(1, 1), expected, "opacity {opacity}");
    }
    stage.scene.sprite_mut(id).unwrap().set_opacity(0);
    assert!(all(&stage.frame(), BLACK));
    stage.scene.sprite_mut(id).unwrap().set_opacity(300);
    assert_eq!(stage.frame().pixel(1, 1), WHITE);
}

#[test]
fn blend_types_add_and_subtract() {
    const GREY: Rgba = [128, 128, 128, 255];
    let mut stage = Stage::sized(16, 16);
    let grey = stage.solid(8, 8, GREY);
    let white = stage.solid(4, 4, WHITE);
    let dim = stage.solid(4, 4, [100, 50, 200, 255]);
    sprite(&mut stage, grey, 0, 0);
    let over = sprite(&mut stage, white, 2, 2);

    stage.scene.sprite_mut(over).unwrap().blend_type = 2;
    let frame = stage.frame();
    assert_eq!(frame.pixel(3, 3), BLACK);
    assert_eq!(frame.pixel(0, 0), GREY);

    stage.scene.sprite_mut(over).unwrap().blend_type = 1;
    assert_eq!(stage.frame().pixel(3, 3), WHITE);

    stage
        .scene
        .sprite_mut(over)
        .unwrap()
        .set_bitmap(Some(dim), &stage.bitmaps);
    assert_eq!(stage.frame().pixel(3, 3), [228, 178, 255, 255]);
    stage.scene.sprite_mut(over).unwrap().blend_type = 2;
    assert_eq!(stage.frame().pixel(3, 3), [28, 78, 0, 255]);

    // Every other value blends normally.
    for blend_type in [0, 3, -1, i32::MAX] {
        stage.scene.sprite_mut(over).unwrap().blend_type = blend_type;
        assert_eq!(stage.frame().pixel(3, 3), [100, 50, 200, 255]);
    }
}

#[test]
fn the_colour_of_a_sprite_recolours_its_own_pixels_only() {
    let mut stage = Stage::sized(16, 16);
    let blue = stage.solid(16, 16, BLUE);
    // Opaque on the left, transparent on the right.
    let shape = stage.pixels(2, 1, &[GREEN, CLEAR]);
    sprite(&mut stage, blue, 0, 0);
    let id = sprite(&mut stage, shape, 4, 4);
    stage.scene.sprite_mut(id).unwrap().color = Color::new(255.0, 0.0, 0.0, 255.0);
    let frame = stage.frame();
    assert_eq!(frame.pixel(4, 4), RED);
    assert_eq!(
        frame.pixel(5, 4),
        BLUE,
        "transparent pixels stay transparent"
    );
    assert_eq!(frame.pixel(3, 4), BLUE);
    assert_eq!(frame.pixel(4, 5), BLUE);

    // A partial alpha mixes the colour into the sprite's own pixel.
    stage.scene.sprite_mut(id).unwrap().color = Color::new(255.0, 0.0, 0.0, 100.0);
    assert_eq!(
        stage.frame().pixel(4, 4),
        vkrgss_pixel::blend_color(GREEN, [255, 0, 0, 100])
    );
}

#[test]
fn the_tone_of_a_sprite_comes_before_its_colour_and_stays_inside_it() {
    let mut stage = Stage::sized(16, 16);
    let blue = stage.solid(16, 16, BLUE);
    let patch = stage.solid(2, 2, [200, 100, 0, 255]);
    sprite(&mut stage, blue, 0, 0);
    let id = sprite(&mut stage, patch, 4, 4);
    stage.scene.sprite_mut(id).unwrap().tone = Tone::new(-50.0, 50.0, 30.0, 0.0);
    let frame = stage.frame();
    assert_eq!(frame.pixel(4, 4), [150, 150, 30, 255]);
    assert_eq!(frame.pixel(3, 4), BLUE);
    assert_eq!(frame.pixel(6, 6), BLUE);

    stage.scene.sprite_mut(id).unwrap().tone = Tone::new(0.0, 0.0, 0.0, 255.0);
    let grey = vkrgss_pixel::apply_tone([200, 100, 0, 255], 0, 0, 0, 255);
    assert_eq!((grey[0], grey[1]), (grey[1], grey[2]), "full grey");
    assert_eq!(stage.frame().pixel(4, 4), grey);

    stage.scene.sprite_mut(id).unwrap().color = Color::new(0.0, 255.0, 0.0, 128.0);
    assert_eq!(
        stage.frame().pixel(4, 4),
        vkrgss_pixel::blend_color(grey, [0, 255, 0, 128])
    );
}

/// A sprite with one opaque black pixel at (0, 0).
fn dot(stage: &mut Stage) -> SpriteId {
    let bitmap = stage.solid(1, 1, [0, 0, 0, 255]);
    sprite(stage, bitmap, 0, 0)
}

/// What the black pixel of [`dot`] looks like under a white colour of `alpha`.
fn whitened(alpha: u8) -> Rgba {
    vkrgss_pixel::blend_color([0, 0, 0, 255], [255, 255, 255, alpha])
}

#[test]
fn a_flash_decays_with_each_update() {
    let mut stage = Stage::sized(4, 4);
    let id = dot(&mut stage);
    let white = Color::new(255.0, 255.0, 255.0, 200.0);
    stage.scene.sprite_mut(id).unwrap().flash(Some(white), 4);
    for alpha in [200, 150, 100, 50] {
        assert_eq!(stage.frame().pixel(0, 0), whitened(alpha), "alpha {alpha}");
        stage.scene.sprite_mut(id).unwrap().update();
    }
    assert_eq!(
        stage.frame().pixel(0, 0),
        BLACK,
        "over after the fourth update"
    );
    stage.scene.sprite_mut(id).unwrap().update();
    assert_eq!(stage.frame().pixel(0, 0), BLACK);

    // Each step is rounded down from the one before: 128 * 4 / 5, 102 * 3 / 4, ...
    let white = Color::new(255.0, 255.0, 255.0, 128.0);
    stage.scene.sprite_mut(id).unwrap().flash(Some(white), 5);
    for alpha in [128, 102, 76, 50, 25] {
        assert_eq!(stage.frame().pixel(0, 0), whitened(alpha), "alpha {alpha}");
        stage.scene.sprite_mut(id).unwrap().update();
    }
    assert_eq!(stage.frame().pixel(0, 0), BLACK);
}

#[test]
fn the_larger_alpha_of_flash_and_own_colour_wins_and_a_tie_goes_to_the_flash() {
    let mut stage = Stage::sized(4, 4);
    let id = dot(&mut stage);
    let own = vkrgss_pixel::blend_color([0, 0, 0, 255], [255, 0, 0, 120]);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.color = Color::new(255.0, 0.0, 0.0, 120.0);
    sprite.flash(Some(Color::new(255.0, 255.0, 255.0, 200.0)), 4);
    // Flash alphas 200 and 150 exceed 120; 100 and 50 do not.
    for expected in [whitened(200), whitened(150), own, own, own] {
        assert_eq!(stage.frame().pixel(0, 0), expected);
        stage.scene.sprite_mut(id).unwrap().update();
    }

    // 240, 180 and 120: the last equals the colour's alpha and still shows.
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.flash(Some(Color::new(255.0, 255.0, 255.0, 240.0)), 4);
    for expected in [whitened(240), whitened(180), whitened(120), own] {
        assert_eq!(stage.frame().pixel(0, 0), expected);
        stage.scene.sprite_mut(id).unwrap().update();
    }
}

#[test]
fn a_flash_without_a_colour_hides_the_sprite() {
    let mut stage = Stage::sized(4, 4);
    let bitmap = stage.solid(1, 1, RED);
    let id = sprite(&mut stage, bitmap, 0, 0);
    stage.scene.sprite_mut(id).unwrap().flash(None, 2);
    assert!(all(&stage.frame(), BLACK));
    stage.scene.sprite_mut(id).unwrap().update();
    assert!(all(&stage.frame(), BLACK));
    stage.scene.sprite_mut(id).unwrap().update();
    assert_eq!(stage.frame().pixel(0, 0), RED);
}

#[test]
fn a_flash_advances_only_when_the_sprite_is_updated() {
    let mut stage = Stage::sized(4, 4);
    let id = dot(&mut stage);
    let white = Color::new(255.0, 255.0, 255.0, 200.0);
    stage.scene.sprite_mut(id).unwrap().flash(Some(white), 4);
    let first = stage.frame();
    assert_eq!(first.pixel(0, 0), whitened(200));
    assert_eq!(stage.frame(), first);
    assert_eq!(stage.frame(), first);

    // A new flash replaces the running one, and one of duration 0 shows until the next
    // update.
    stage.scene.sprite_mut(id).unwrap().update();
    stage.scene.sprite_mut(id).unwrap().flash(Some(white), 2);
    assert_eq!(stage.frame().pixel(0, 0), whitened(200));
    stage.scene.sprite_mut(id).unwrap().flash(Some(white), 0);
    assert_eq!(stage.frame().pixel(0, 0), whitened(200));
    assert_eq!(stage.frame().pixel(0, 0), whitened(200));
    stage.scene.sprite_mut(id).unwrap().update();
    assert_eq!(stage.frame().pixel(0, 0), BLACK);
}

const MEASURED: &str = include_str!("../../tests/data/flash-bush-samples.txt");
/// The background of the measured frames.
const BACKDROP: Rgba = [88, 120, 200, 255];

/// The lines of [`MEASURED`] that begin with `kind`, as what follows it up to the colon
/// and what follows the colon.
fn measured(kind: &'static str) -> impl Iterator<Item = (&'static str, &'static str)> {
    MEASURED
        .lines()
        .filter_map(move |line| line.strip_prefix(kind)?.split_once(':'))
}

fn rgb(text: &str) -> Rgba {
    let channels: Vec<u8> = text.split(',').map(|c| c.parse().unwrap()).collect();
    [channels[0], channels[1], channels[2], 255]
}

/// A stage with the measured background behind everything.
fn measured_stage() -> Stage {
    let mut stage = Stage::sized(8, 80);
    let backdrop = stage.solid(8, 80, BACKDROP);
    sprite(&mut stage, backdrop, 0, 0);
    stage
}

/// The "flash" lines of `flash-bush-samples.txt`: an opaque grey pixel, flashed as the
/// row number says, before any update and after each of twelve.
#[test]
fn flashes_show_the_measured_pixels() {
    let red = Some(Color::new(255.0, 0.0, 0.0, 255.0));
    let mut checked = 0;
    for (row, pixels) in measured("flash ") {
        let row: u32 = row.parse().unwrap();
        let mut stage = measured_stage();
        let grey = stage.solid(1, 1, [100, 100, 100, 255]);
        let viewport = stage.scene.create_viewport(Rect::new(0, 0, 8, 8)).unwrap();
        stage.scene.set_viewport_z(viewport, 1).unwrap();
        let in_viewport = matches!(row, 6 | 7).then_some(viewport);
        let id = sprite_in(&mut stage, in_viewport, grey, 2, 2);
        stage.scene.set_sprite_z(id, 1).unwrap();
        let subject = stage.scene.sprite_mut(id).unwrap();
        match row {
            0 | 13 => subject.flash(red, 8),
            1 => subject.flash(Some(Color::new(0.0, 255.0, 0.0, 128.0)), 5),
            2 => subject.flash(None, 4),
            3 => {
                subject.color = Color::new(0.0, 0.0, 255.0, 100.0);
                subject.flash(Some(Color::new(255.0, 255.0, 255.0, 200.0)), 10);
            }
            4 => subject.flash(red, 1),
            6 => stage.scene.viewport_mut(viewport).unwrap().flash(red, 8),
            7 => stage.scene.viewport_mut(viewport).unwrap().flash(None, 4),
            9 => {
                subject.tone = Tone::new(50.0, -50.0, 0.0, 128.0);
                subject.flash(red, 8);
            }
            10 => {
                subject.set_opacity(128);
                subject.flash(red, 8);
            }
            11 => subject.flash(Some(Color::new(255.0, 255.0, 255.0, 255.0)), 16),
            12 => subject.flash(red, 8),
            14 => {
                subject.blend_type = 1;
                subject.flash(red, 8);
            }
            15 => subject.flash(red, 0),
            _ => panic!("no setup for flash row {row}"),
        }
        for (updates, expected) in pixels.split_whitespace().enumerate() {
            if row == 12 && updates == 4 {
                let blue = Some(Color::new(0.0, 0.0, 255.0, 255.0));
                stage.scene.sprite_mut(id).unwrap().flash(blue, 4);
            }
            assert_eq!(
                stage.frame().pixel(2, 2),
                rgb(expected),
                "flash row {row} after {updates} updates"
            );
            if row != 13 {
                stage.scene.sprite_mut(id).unwrap().update();
                stage.scene.viewport_mut(viewport).unwrap().update();
            }
        }
        checked += 1;
    }
    assert!(checked >= 12);
}

/// The "bush" lines of `flash-bush-samples.txt`: a sprite 32 rows high of one colour,
/// set up as the line says, and the colour of its screen rows as runs.
#[test]
fn bush_depths_show_the_measured_rows() {
    let mut checked = 0;
    for (setup, runs) in measured("bush ") {
        // The number that follows `name` in the setup, if the setup names it.
        let value = |name: &str| -> Option<f64> {
            let rest = &setup[setup.find(name)? + name.len()..];
            rest.split_whitespace()
                .next()?
                .trim_end_matches(',')
                .parse()
                .ok()
        };
        let mut stage = measured_stage();
        let alpha = value("pixel alpha").unwrap_or(255.0) as u8;
        let bitmap = stage.solid(4, 32, [100, 155, 128, alpha]);
        let id = sprite(&mut stage, bitmap, 2, 0);
        let subject = stage.scene.sprite_mut(id).unwrap();
        subject.bush_depth = value("depth").unwrap() as i32;
        subject.set_opacity(value("opacity").unwrap_or(255.0) as i32);
        subject.zoom_y = value("zoom_y").unwrap_or(1.0);
        subject.src_rect.height = value("src_rect height").unwrap_or(32.0) as i32;
        let frame = stage.frame();

        let runs: Vec<(u32, Rgba)> = runs
            .split_whitespace()
            .map(|run| {
                let (first, colour) = run.split_once(':').unwrap();
                (first.parse().unwrap(), rgb(colour))
            })
            .collect();
        for y in 0..80 {
            let run = runs.iter().rev().find(|run| run.0 <= y).unwrap();
            assert_eq!(frame.pixel(3, y), run.1, "row {y} of bush {setup}");
        }
        assert_eq!(runs.last().unwrap().1, BACKDROP);
        checked += 1;
    }
    assert!(checked >= 8);
}

/// The viewport of the viewport scenarios, 100, 50, 200, 100, holding a sprite at 0, 0
/// with a 300 by 10 bitmap whose columns all differ.
fn wide_viewport(stage: &mut Stage) -> (ViewportId, SpriteId) {
    let pixels: Vec<Rgba> = (0..10).flat_map(|_| (0..300u32).map(column)).collect();
    let bitmap = stage.pixels(300, 10, &pixels);
    let viewport = stage
        .scene
        .create_viewport(Rect::new(100, 50, 200, 100))
        .unwrap();
    let sprite = sprite_in(stage, Some(viewport), bitmap, 0, 0);
    (viewport, sprite)
}

/// The pixel of column `x` of the bitmap of [`wide_viewport`].
fn column(x: u32) -> Rgba {
    [(x % 256) as u8, (x / 256) as u8 + 1, 200, 255]
}

#[test]
fn a_viewport_offsets_and_clips_its_elements() {
    let mut stage = Stage::new();
    wide_viewport(&mut stage);
    let frame = stage.frame();
    assert_eq!(frame.pixel(100, 50), column(0));
    assert_eq!(frame.pixel(299, 59), column(199));
    assert_eq!(frame.pixel(99, 50), BLACK);
    assert_eq!(frame.pixel(100, 49), BLACK);
    assert_eq!(frame.pixel(100, 60), BLACK);
    for x in 300..640 {
        assert_eq!(frame.pixel(x, 50), BLACK, "x {x}");
    }
    assert_eq!(drawn(&frame).len(), 200 * 10);
}

#[test]
fn the_origin_of_a_viewport_scrolls_its_elements() {
    let mut stage = Stage::new();
    let (viewport, sprite) = wide_viewport(&mut stage);
    stage.scene.viewport_mut(viewport).unwrap().ox = 10;
    let frame = stage.frame();
    assert_eq!(frame.pixel(100, 50), column(10));
    assert_eq!(frame.pixel(299, 50), column(209));
    assert_eq!(frame.pixel(99, 50), BLACK);
    assert_eq!(frame.pixel(300, 50), BLACK);

    let viewport = stage.scene.viewport_mut(viewport).unwrap();
    viewport.ox = -5;
    viewport.oy = 4;
    stage.scene.sprite_mut(sprite).unwrap().y = 6;
    let frame = stage.frame();
    assert_eq!(frame.pixel(105, 52), column(0));
    assert_eq!(frame.pixel(104, 52), BLACK);
    assert_eq!(frame.pixel(105, 51), BLACK);
}

#[test]
fn the_z_of_a_viewport_outranks_the_z_of_its_elements() {
    let mut stage = Stage::sized(16, 16);
    let red = stage.solid(4, 4, RED);
    let green = stage.solid(4, 4, GREEN);
    let viewport = stage
        .scene
        .create_viewport(Rect::new(0, 0, 16, 16))
        .unwrap();
    stage.scene.set_viewport_z(viewport, 100).unwrap();
    let inside = sprite_in(&mut stage, Some(viewport), red, 2, 2);
    let outside = sprite(&mut stage, green, 4, 4);
    stage.scene.set_sprite_z(inside, 0).unwrap();
    stage.scene.set_sprite_z(outside, 50).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(4, 4), RED);
    assert_eq!(frame.pixel(7, 7), GREEN);

    stage.scene.set_sprite_z(outside, 101).unwrap();
    assert_eq!(stage.frame().pixel(4, 4), GREEN);
}

#[test]
fn an_invisible_or_disposed_viewport_hides_its_elements() {
    let mut stage = Stage::sized(16, 16);
    let red = stage.solid(4, 4, RED);
    let green = stage.solid(2, 2, GREEN);
    let viewport = stage
        .scene
        .create_viewport(Rect::new(0, 0, 16, 16))
        .unwrap();
    let inside = sprite_in(&mut stage, Some(viewport), red, 2, 2);
    sprite(&mut stage, green, 10, 10);
    assert_eq!(stage.frame().pixel(2, 2), RED);

    stage.scene.viewport_mut(viewport).unwrap().visible = false;
    let frame = stage.frame();
    assert_eq!(frame.pixel(2, 2), BLACK);
    assert_eq!(frame.pixel(10, 10), GREEN);
    assert_eq!(drawn(&frame).len(), 4);

    stage.scene.viewport_mut(viewport).unwrap().visible = true;
    assert_eq!(stage.frame().pixel(2, 2), RED);

    stage.scene.dispose_viewport(viewport, &mut stage.bitmaps);
    assert_eq!(drawn(&stage.frame()).len(), 4);
    // The sprite went with its viewport.
    assert!(stage.scene.is_sprite_disposed(inside));
    // One created in the disposed viewport is alive and draws nothing, also when the
    // slot is reused.
    let late = sprite_in(&mut stage, Some(viewport), red, 2, 2);
    let frame = stage.frame();
    assert_eq!(frame.pixel(2, 2), BLACK);
    assert_eq!(drawn(&frame).len(), 4);
    stage
        .scene
        .create_viewport(Rect::new(0, 0, 16, 16))
        .unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(2, 2), BLACK);
    assert_eq!(drawn(&frame).len(), 4);
    assert!(!stage.scene.is_sprite_disposed(late));
}

#[test]
fn a_viewport_without_pixels_on_the_screen_shows_nothing() {
    let mut stage = Stage::sized(16, 16);
    let red = stage.solid(4, 4, RED);
    for rect in [
        Rect::new(0, 0, 0, 16),
        Rect::new(0, 0, 16, -3),
        Rect::new(16, 0, 8, 8),
        Rect::new(-8, -8, 8, 8),
    ] {
        let viewport = stage.scene.create_viewport(rect).unwrap();
        stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(255.0, 255.0, 255.0, 0.0);
        sprite_in(&mut stage, Some(viewport), red, 0, 0);
        assert!(all(&stage.frame(), BLACK), "{rect:?}");
    }
    // Partly on the screen, only that part shows.
    let viewport = stage
        .scene
        .create_viewport(Rect::new(-2, 14, 8, 8))
        .unwrap();
    sprite_in(&mut stage, Some(viewport), red, 0, 0);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 14), RED);
    assert_eq!(frame.pixel(1, 15), RED);
    assert_eq!(drawn(&frame).len(), 4);
}

/// A scene with a striped sprite behind a viewport that covers the left half of it.
fn tinted_half(stage: &mut Stage) -> ViewportId {
    let stripes = stage.pixels(4, 1, &[RED, GREEN, BLUE, [200, 100, 0, 255]]);
    let behind = sprite(stage, stripes, 0, 0);
    stage.scene.sprite_mut(behind).unwrap().zoom_y = 4.0;
    let viewport = stage.scene.create_viewport(Rect::new(0, 0, 2, 4)).unwrap();
    stage.scene.set_viewport_z(viewport, 10).unwrap();
    viewport
}

#[test]
fn the_tone_of_a_viewport_reaches_what_is_behind_it() {
    let mut stage = Stage::sized(8, 8);
    let viewport = tinted_half(&mut stage);
    stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(0.0, 0.0, 0.0, 255.0);
    let frame = stage.frame();
    for (x, original) in [(0, RED), (1, GREEN)] {
        let grey = vkrgss_pixel::apply_tone(original, 0, 0, 0, 255);
        assert_eq!((grey[0], grey[1]), (grey[1], grey[2]));
        assert_eq!(frame.pixel(x, 0), grey);
        assert_eq!(frame.pixel(x, 3), grey);
    }
    assert_eq!(frame.pixel(2, 0), BLUE);
    assert_eq!(frame.pixel(3, 3), [200, 100, 0, 255]);
    assert_eq!(frame.pixel(0, 4), BLACK, "outside the rectangle");
}

#[test]
fn the_effect_of_a_viewport_covers_its_own_elements_too() {
    let mut stage = Stage::sized(8, 8);
    let viewport = tinted_half(&mut stage);
    let white = stage.solid(1, 1, WHITE);
    sprite_in(&mut stage, Some(viewport), white, 1, 1);
    stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(-100.0, -100.0, 0.0, 0.0);
    let frame = stage.frame();
    assert_eq!(frame.pixel(1, 1), [155, 155, 255, 255]);
    assert_eq!(frame.pixel(0, 0), [155, 0, 0, 255]);
}

#[test]
fn elements_in_front_of_a_viewport_keep_their_colours() {
    let mut stage = Stage::sized(8, 8);
    let viewport = tinted_half(&mut stage);
    stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(0.0, 0.0, 0.0, 255.0);
    let white = stage.solid(1, 1, [10, 200, 30, 255]);
    let front = sprite(&mut stage, white, 0, 0);
    stage.scene.set_sprite_z(front, 11).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), [10, 200, 30, 255]);
    assert_ne!(frame.pixel(0, 1), RED);

    // Behind the viewport, the same sprite is toned with the rest.
    stage.scene.set_sprite_z(front, 9).unwrap();
    assert_eq!(
        stage.frame().pixel(0, 0),
        vkrgss_pixel::apply_tone([10, 200, 30, 255], 0, 0, 0, 255)
    );
}

#[test]
fn the_colour_and_flash_of_a_viewport_fill_its_rectangle() {
    let mut stage = Stage::sized(8, 8);
    let viewport = tinted_half(&mut stage);
    stage.scene.viewport_mut(viewport).unwrap().color = Color::new(0.0, 0.0, 255.0, 100.0);
    let own = |pixel| vkrgss_pixel::blend_color(pixel, [0, 0, 255, 100]);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), own(RED));
    assert_eq!(frame.pixel(1, 2), own(GREEN));
    assert_eq!(frame.pixel(2, 0), BLUE);

    // A flash of alpha 200 for 4 updates: 200 and 150 exceed the colour's 100, and the
    // third step equals it, which still shows the flash.
    let white = Color::new(255.0, 255.0, 255.0, 200.0);
    stage
        .scene
        .viewport_mut(viewport)
        .unwrap()
        .flash(Some(white), 4);
    let flashed = |alpha| vkrgss_pixel::blend_color(RED, [255, 255, 255, alpha]);
    assert_eq!(stage.frame().pixel(0, 0), flashed(200));
    assert_eq!(
        stage.frame().pixel(0, 0),
        flashed(200),
        "no update, no progress"
    );
    for expected in [flashed(150), flashed(100), own(RED), own(RED)] {
        stage.scene.viewport_mut(viewport).unwrap().update();
        assert_eq!(stage.frame().pixel(0, 0), expected);
    }
}

#[test]
fn a_viewport_flash_without_a_colour_hides_its_elements() {
    let mut stage = Stage::sized(8, 8);
    let viewport = tinted_half(&mut stage);
    let white = stage.solid(1, 1, WHITE);
    sprite_in(&mut stage, Some(viewport), white, 1, 1);
    stage.scene.viewport_mut(viewport).unwrap().flash(None, 1);
    let frame = stage.frame();
    assert_eq!(frame.pixel(1, 1), GREEN, "what is behind still shows");
    stage.scene.viewport_mut(viewport).unwrap().update();
    assert_eq!(stage.frame().pixel(1, 1), WHITE);
}

// The rules of RGSS2.

/// A red sprite with `y` and then a green one with y 0, of equal z, both covering the
/// corner of the screen whatever their y.
fn red_with_y_then_green(stage: &mut Stage, y: i32) -> (SpriteId, SpriteId) {
    let red = stage.solid(4, 4, RED);
    let green = stage.solid(4, 4, GREEN);
    let older = sprite(stage, red, 0, y);
    stage.scene.sprite_mut(older).unwrap().oy = y;
    (older, sprite(stage, green, 0, 0))
}

/// Scenario "y breaks the tie, measured on the original".
#[test]
fn in_rgss2_the_larger_y_is_in_front_among_equal_z() {
    for (rules, front) in [(Rules::Rgss2, RED), (Rules::Rgss1, GREEN)] {
        let mut stage = Stage::with_rules(rules, 8, 8);
        red_with_y_then_green(&mut stage, 9);
        assert_eq!(stage.frame().pixel(1, 1), front, "{rules:?}");
    }
    // A y below 0 sorts behind 0, and z counts before y.
    let mut stage = Stage::with_rules(Rules::Rgss2, 8, 8);
    let (red, green) = red_with_y_then_green(&mut stage, 9);
    stage.scene.sprite_mut(green).unwrap().y = -3;
    stage.scene.sprite_mut(green).unwrap().oy = -3;
    assert_eq!(stage.frame().pixel(1, 1), RED);
    stage.scene.set_sprite_z(green, 1).unwrap();
    assert_eq!(stage.frame().pixel(1, 1), GREEN);
    stage.scene.set_sprite_z(green, 0).unwrap();
    stage.scene.sprite_mut(red).unwrap().y = -4;
    stage.scene.sprite_mut(red).unwrap().oy = -4;
    assert_eq!(stage.frame().pixel(1, 1), GREEN);
}

/// Scenario "The property, not the place, measured on the original".
#[test]
fn in_rgss2_the_y_of_a_sprite_orders_it_wherever_it_is_drawn() {
    let mut stage = Stage::with_rules(Rules::Rgss2, 8, 8);
    let (older, _) = red_with_y_then_green(&mut stage, 20);
    let sprite = stage.scene.sprite(older).unwrap();
    assert_eq!((sprite.y, sprite.oy), (20, 20));
    assert_eq!(stage.frame().pixel(1, 1), RED);
}

/// Scenario "Viewport, measured on the original".
#[test]
fn in_rgss2_a_viewport_takes_its_place_with_a_y_of_zero() {
    let mut stage = Stage::with_rules(Rules::Rgss2, 8, 8);
    let viewport = stage.scene.create_viewport(Rect::new(0, 0, 8, 8)).unwrap();
    let red = stage.solid(4, 4, RED);
    let inside = sprite_in(&mut stage, Some(viewport), red, 0, 50);
    stage.scene.sprite_mut(inside).unwrap().oy = 50;
    let green = stage.solid(4, 4, GREEN);
    let free = sprite(&mut stage, green, 0, 0);
    assert_eq!(stage.frame().pixel(1, 1), GREEN);
    // A free sprite with a y is in front of a younger viewport of its z.
    stage.scene.dispose_sprite(free);
    let blue = stage.solid(4, 4, BLUE);
    let older = sprite(&mut stage, blue, 0, 9);
    stage.scene.sprite_mut(older).unwrap().oy = 9;
    let younger = stage.scene.create_viewport(Rect::new(0, 0, 8, 8)).unwrap();
    sprite_in(&mut stage, Some(younger), green, 0, 0);
    assert_eq!(stage.frame().pixel(1, 1), BLUE);
    // Inside a viewport the same key orders its elements.
    let white = stage.solid(4, 4, WHITE);
    sprite_in(&mut stage, Some(viewport), white, 0, 0);
    stage.scene.dispose_sprite(older);
    stage.scene.dispose_viewport(younger, &mut stage.bitmaps);
    assert_eq!(stage.frame().pixel(1, 1), RED);
}

/// In RGSS2 a plane has a y of 0, whatever its origin.
#[test]
fn in_rgss2_a_plane_is_ordered_with_a_y_of_zero() {
    let mut stage = Stage::with_rules(Rules::Rgss2, 8, 8);
    let red = stage.solid(4, 4, RED);
    let older = sprite(&mut stage, red, 0, 4);
    stage.scene.sprite_mut(older).unwrap().oy = 4;
    let green = stage.solid(4, 4, GREEN);
    let plane = stage.scene.create_plane(None).unwrap();
    stage.scene.plane_mut(plane).unwrap().bitmap = Some(green);
    stage.scene.plane_mut(plane).unwrap().oy = 100;
    assert_eq!(stage.frame().pixel(1, 1), RED);
    stage.scene.sprite_mut(older).unwrap().y = 0;
    stage.scene.sprite_mut(older).unwrap().oy = 0;
    assert_eq!(stage.frame().pixel(1, 1), GREEN);
}

#[test]
fn in_rgss2_a_viewport_lays_its_flash_colour_over_its_colour() {
    let mut stage = Stage::with_rules(Rules::Rgss2, 8, 8);
    let viewport = tinted_half(&mut stage);
    let own = [0, 0, 255, 200];
    stage.scene.viewport_mut(viewport).unwrap().color = Color::new(0.0, 0.0, 255.0, 200.0);
    stage
        .scene
        .viewport_mut(viewport)
        .unwrap()
        .flash(Some(Color::new(255.0, 255.0, 255.0, 100.0)), 4);
    // The flash is weaker than the colour, which hides it in RGSS1.
    for alpha in [100, 75, 50, 25] {
        let both = |pixel| {
            let colored = vkrgss_pixel::blend_color(pixel, own);
            vkrgss_pixel::blend_color(colored, [255, 255, 255, alpha])
        };
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), both(RED), "alpha {alpha}");
        assert_eq!(frame.pixel(1, 3), both(GREEN), "alpha {alpha}");
        assert_eq!(frame.pixel(2, 0), BLUE, "outside the rectangle");
        stage.scene.viewport_mut(viewport).unwrap().update();
    }
    assert_eq!(
        stage.frame().pixel(0, 0),
        vkrgss_pixel::blend_color(RED, own)
    );
}

/// Scenario "Flash without a colour, measured on the original".
#[test]
fn in_rgss2_a_viewport_flash_without_a_colour_leaves_the_viewport_undone() {
    let mut stage = Stage::with_rules(Rules::Rgss2, 8, 8);
    let viewport = tinted_half(&mut stage);
    let white = stage.solid(1, 1, WHITE);
    sprite_in(&mut stage, Some(viewport), white, 1, 1);
    let tone = Tone::new(-100.0, -100.0, 0.0, 0.0);
    stage.scene.viewport_mut(viewport).unwrap().tone = tone;
    stage.scene.viewport_mut(viewport).unwrap().color = Color::new(9.0, 9.0, 9.0, 90.0);
    stage.scene.viewport_mut(viewport).unwrap().flash(None, 5);
    for update in 0..5 {
        let frame = stage.frame();
        for (x, behind) in [(0, RED), (1, GREEN), (2, BLUE)] {
            for y in 0..4 {
                assert_eq!(frame.pixel(x, y), behind, "update {update}");
            }
        }
        stage.scene.viewport_mut(viewport).unwrap().update();
    }
    let tinted = |pixel| {
        let toned = vkrgss_pixel::apply_tone(pixel, -100, -100, 0, 0);
        vkrgss_pixel::blend_color(toned, [9, 9, 9, 90])
    };
    let frame = stage.frame();
    assert_eq!(frame.pixel(1, 1), tinted(WHITE));
    assert_eq!(frame.pixel(0, 0), tinted(RED));
}

/// In RGSS1 the tone and colour of a viewport stay while a flash without a colour hides
/// its elements.
#[test]
fn in_rgss1_a_viewport_flash_without_a_colour_keeps_tone_and_colour() {
    let mut stage = Stage::sized(8, 8);
    let viewport = tinted_half(&mut stage);
    stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(-100.0, -100.0, 0.0, 0.0);
    stage.scene.viewport_mut(viewport).unwrap().flash(None, 5);
    assert_eq!(stage.frame().pixel(0, 0), [155, 0, 0, 255]);
}

#[test]
fn a_neutral_viewport_changes_nothing() {
    let mut stage = Stage::sized(8, 8);
    let stripes = stage.pixels(4, 1, &[RED, GREEN, BLUE, [200, 100, 0, 255]]);
    sprite(&mut stage, stripes, 0, 0);
    let without = stage.frame();

    let viewport = stage.scene.create_viewport(Rect::new(0, 0, 8, 8)).unwrap();
    stage.scene.set_viewport_z(viewport, 10).unwrap();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert!(
        packet
            .ops
            .iter()
            .all(|op| op.kind() != vkrgss_proto::OpKind::RectEffect),
        "a neutral viewport emits no rectangle effect"
    );
    assert_eq!(stage.frame(), without);

    // A colour without alpha and a finished flash are neutral too.
    let viewport = stage.scene.viewport_mut(viewport).unwrap();
    viewport.color = Color::new(255.0, 255.0, 255.0, 0.0);
    viewport.flash(Some(Color::new(255.0, 255.0, 255.0, 255.0)), 1);
    viewport.update();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert!(
        packet
            .ops
            .iter()
            .all(|op| op.kind() != vkrgss_proto::OpKind::RectEffect)
    );
    assert_eq!(stage.frame(), without);
}

#[test]
fn the_frame_count_follows_the_extracted_frames_and_can_be_set() {
    let mut scene = Scene::new();
    let mut bitmaps = Bitmaps::new();
    assert_eq!(scene.frame_count(), 0);
    for index in 0..3 {
        assert_eq!(scene.extract(&mut bitmaps).frame_index, index);
    }
    assert_eq!(scene.frame_count(), 3);
    scene.set_frame_count(100);
    assert_eq!(scene.frame_count(), 100);
    // The packet's own index is not the settable count.
    assert_eq!(scene.extract(&mut bitmaps).frame_index, 3);
    assert_eq!(scene.frame_count(), 101);

    scene.set_frame_count(-5);
    scene.extract(&mut bitmaps);
    assert_eq!(scene.frame_count(), -4);
    scene.set_frame_count(i64::MAX);
    assert_eq!(scene.extract(&mut bitmaps).frame_index, 5);
    assert_eq!(scene.frame_count(), i64::MIN);
}

/// Scenarios "Default", "Limits" and "Carried by the packet" of "Brightness of the scene".
#[test]
fn the_brightness_is_clamped_and_carried_by_every_packet() {
    let mut scene = Scene::new();
    let mut bitmaps = Bitmaps::new();
    assert_eq!(scene.brightness(), 255);
    assert_eq!(scene.extract(&mut bitmaps).brightness, 255);
    scene.set_brightness(-5);
    assert_eq!(scene.brightness(), 0);
    scene.set_brightness(300);
    assert_eq!(scene.brightness(), 255);
    assert_eq!(scene.extract(&mut bitmaps).brightness, 255);
    scene.set_brightness(100);
    assert_eq!(scene.extract(&mut bitmaps).brightness, 100);
    assert_eq!(scene.extract(&mut bitmaps).brightness, 100);
}

#[test]
fn the_frame_rate_is_clamped_and_carried_by_every_packet() {
    let mut scene = Scene::new();
    let mut bitmaps = Bitmaps::new();
    assert_eq!(scene.frame_rate().get(), 40);
    assert_eq!(scene.extract(&mut bitmaps).frame_rate, FrameRate::DEFAULT);
    scene.set_frame_rate(5);
    assert_eq!(scene.frame_rate().get(), 10);
    assert_eq!(scene.extract(&mut bitmaps).frame_rate.get(), 10);
    scene.set_frame_rate(500);
    assert_eq!(scene.frame_rate().get(), 120);
    assert_eq!(scene.extract(&mut bitmaps).frame_rate.get(), 120);
    scene.set_frame_rate(60);
    assert_eq!(scene.extract(&mut bitmaps).frame_rate.get(), 60);
    scene.set_frame_rate(-1);
    assert_eq!(scene.frame_rate().get(), 10);
}

#[test]
fn packets_carry_the_logical_size_of_the_scene() {
    let mut bitmaps = Bitmaps::new();
    let packet = Scene::new().extract(&mut bitmaps);
    assert_eq!(packet.logical_size, Size::new(640, 480));
    let packet = Scene::with_logical_size(Size::new(320, 240)).extract(&mut bitmaps);
    assert_eq!(packet.logical_size, Size::new(320, 240));

    // A sprite is clipped to that size.
    let mut stage = Stage::sized(8, 6);
    let red = stage.solid(20, 20, RED);
    sprite(&mut stage, red, 0, 0);
    let frame = stage.frame();
    assert_eq!(frame.size, Size::new(8, 6));
    assert!(all(&frame, RED));
}

/// The ids the deltas create, update and destroy, in order.
fn delta_ids(deltas: &[BitmapDelta]) -> Vec<(&'static str, BitmapId)> {
    deltas
        .iter()
        .map(|delta| match delta {
            BitmapDelta::Create { id, .. } => ("create", *id),
            BitmapDelta::Update { id, .. } => ("update", *id),
            BitmapDelta::Destroy { id } => ("destroy", *id),
        })
        .collect()
}

#[test]
fn deltas_cover_exactly_the_bitmaps_the_ops_refer_to() {
    let mut stage = Stage::sized(16, 16);
    let used = stage.solid(4, 4, RED);
    let unused = stage.solid(4, 4, GREEN);
    let hidden = stage.solid(4, 4, BLUE);
    sprite(&mut stage, used, 0, 0);
    let invisible = sprite(&mut stage, hidden, 0, 0);
    stage.scene.sprite_mut(invisible).unwrap().visible = false;

    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(
        delta_ids(&packet.bitmaps),
        [("create", used), ("update", used)]
    );
    assert_eq!(packet.ops.len(), 1);

    // Nothing changed: nothing is sent, also not for the bitmaps nobody shows.
    stage.bitmaps.set_pixel(unused, 0, 0, color(WHITE)).unwrap();
    stage.bitmaps.set_pixel(hidden, 0, 0, color(WHITE)).unwrap();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(delta_ids(&packet.bitmaps), []);

    // A change to a shown bitmap is sent, and a bitmap is uploaded once it is shown.
    stage.bitmaps.set_pixel(used, 1, 1, color(WHITE)).unwrap();
    stage.scene.sprite_mut(invisible).unwrap().visible = true;
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(
        delta_ids(&packet.bitmaps),
        [("create", hidden), ("update", hidden), ("update", used)]
    );

    // Two sprites with one bitmap upload it once.
    let shared = stage.solid(2, 2, WHITE);
    sprite(&mut stage, shared, 8, 8);
    sprite(&mut stage, shared, 10, 10);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(
        delta_ids(&packet.bitmaps),
        [("create", shared), ("update", shared)]
    );

    // A disposed bitmap the compositor knows is destroyed there, the unused one is not.
    stage.bitmaps.dispose(used);
    stage.bitmaps.dispose(unused);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(delta_ids(&packet.bitmaps), [("destroy", used)]);
}

#[test]
fn a_sprite_that_draws_nothing_does_not_upload_its_bitmap() {
    let mut stage = Stage::sized(16, 16);
    let bitmap = stage.solid(4, 4, RED);
    let id = sprite(&mut stage, bitmap, 0, 0);
    stage.scene.sprite_mut(id).unwrap().set_opacity(0);
    assert_eq!(stage.scene.extract(&mut stage.bitmaps).bitmaps, []);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.set_opacity(255);
    sprite.zoom_x = 0.0;
    assert_eq!(stage.scene.extract(&mut stage.bitmaps).bitmaps, []);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.zoom_x = 1.0;
    sprite.src_rect = Rect::new(4, 4, 4, 4);
    assert_eq!(stage.scene.extract(&mut stage.bitmaps).bitmaps, []);
}

/// One operation of a random script.
#[derive(Debug, Clone)]
enum Step {
    CreateBitmap(u32, u32, u8),
    CreateSkin(u8),
    DisposeBitmap(usize),
    Fill(usize, Rect, [u8; 4]),
    CreateViewport(Rect),
    DisposeViewport(usize),
    Viewport(usize, ViewportChange),
    CreateSprite(Option<usize>),
    DisposeSprite(usize),
    Sprite(usize, SpriteChange),
    CreateWindow(Option<usize>),
    DisposeWindow(usize),
    Window(usize, WindowChange),
    CreateSheet(u32, u32, u8),
    CreatePlane(Option<usize>),
    DisposePlane(usize),
    Plane(usize, PlaneChange),
    CreateMap(i32, i32, u32),
    CreatePriorities(u32),
    SetCell(usize, i32, i32, i32, i16),
    CreateTilemap(Option<usize>),
    DisposeTilemap(usize),
    Tilemap(usize, TilemapChange),
    Update,
}

#[derive(Debug, Clone)]
enum ViewportChange {
    Rect(Rect),
    Origin(i32, i32),
    Z(i32),
    Visible(bool),
    Color([u8; 4]),
    Tone([i16; 4]),
    Flash(Option<[u8; 4]>, i32),
}

#[derive(Debug, Clone)]
enum SpriteChange {
    Bitmap(Option<usize>),
    SrcRect(Rect),
    Position(i32, i32),
    Origin(i32, i32),
    Z(i32),
    Zoom(f64, f64),
    Angle(f64),
    Mirror(bool),
    Visible(bool),
    Opacity(i32),
    BlendType(i32),
    Color([u8; 4]),
    Tone([i16; 4]),
    Flash(Option<[u8; 4]>, i32),
}

#[derive(Debug, Clone)]
enum PlaneChange {
    Bitmap(Option<usize>),
    Origin(i32, i32),
    Z(i32),
    Zoom(f64, f64),
    Visible(bool),
    Opacity(i32),
    BlendType(i32),
    Color([u8; 4]),
    Tone([i16; 4]),
}

#[derive(Debug, Clone)]
enum TilemapChange {
    Tileset(Option<usize>),
    Autotile(usize, Option<usize>),
    MapData(Option<usize>),
    Priorities(Option<usize>),
    FlashData(Option<usize>),
    Origin(i32, i32),
    Visible(bool),
}

#[derive(Debug, Clone)]
enum WindowChange {
    Skin(Option<usize>),
    Contents(Option<usize>),
    Position(i32, i32),
    Size(i32, i32),
    Origin(i32, i32),
    Z(i32),
    Stretch(bool),
    Cursor(Rect),
    Active(bool),
    Visible(bool),
    Pause(bool),
    Opacities(i32, i32, i32),
}

/// A coordinate around a small screen, now and then one at the limits of its type.
fn coordinate() -> impl Strategy<Value = i32> {
    prop_oneof![
        12 => -40..80i32,
        1 => prop_oneof![
            Just(i32::MIN),
            Just(i32::MAX),
            Just(-70_000),
            Just(70_000),
            Just(32_767),
            Just(-32_768)
        ],
    ]
}

/// A length that may be empty or negative, and now and then one too large for a bitmap.
fn length() -> impl Strategy<Value = i32> {
    prop_oneof![
        12 => -4..70i32,
        1 => prop_oneof![Just(i32::MIN), Just(i32::MAX), Just(100_000)],
    ]
}

fn rect() -> impl Strategy<Value = Rect> {
    (coordinate(), coordinate(), length(), length())
        .prop_map(|(x, y, width, height)| Rect::new(x, y, width, height))
}

fn z() -> impl Strategy<Value = i32> {
    prop_oneof![8 => -3..6i32, 1 => prop_oneof![Just(i32::MIN), Just(i32::MAX)]]
}

fn zoom() -> impl Strategy<Value = f64> {
    prop_oneof![
        4 => Just(1.0),
        4 => -4.0..8.0f64,
        1 => prop_oneof![
            Just(0.0),
            Just(-0.0),
            Just(f64::NAN),
            Just(f64::INFINITY),
            Just(f64::NEG_INFINITY),
            Just(1e-300),
            Just(1e300),
            Just(1.0 / 65536.0),
            Just(40_000.0)
        ],
    ]
}

fn angle() -> impl Strategy<Value = f64> {
    prop_oneof![
        3 => prop_oneof![Just(0.0), Just(90.0), Just(180.0), Just(270.0), Just(-90.0)],
        6 => -800.0..800.0f64,
        1 => prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(1e300), Just(-1e-300)],
    ]
}

fn opacity() -> impl Strategy<Value = i32> {
    prop_oneof![Just(255), Just(0), -50..320i32]
}

fn tone() -> impl Strategy<Value = [i16; 4]> {
    prop_oneof![Just([0; 4]), any::<[i16; 4]>()]
}

fn flash() -> impl Strategy<Value = (Option<[u8; 4]>, i32)> {
    (proptest::option::of(any::<[u8; 4]>()), -2..6i32)
}

fn viewport_change() -> impl Strategy<Value = ViewportChange> {
    prop_oneof![
        rect().prop_map(ViewportChange::Rect),
        (coordinate(), coordinate()).prop_map(|(x, y)| ViewportChange::Origin(x, y)),
        z().prop_map(ViewportChange::Z),
        any::<bool>().prop_map(ViewportChange::Visible),
        any::<[u8; 4]>().prop_map(ViewportChange::Color),
        tone().prop_map(ViewportChange::Tone),
        flash().prop_map(|(color, duration)| ViewportChange::Flash(color, duration)),
    ]
}

fn sprite_change() -> impl Strategy<Value = SpriteChange> {
    prop_oneof![
        3 => proptest::option::weighted(0.9, any::<usize>()).prop_map(SpriteChange::Bitmap),
        2 => rect().prop_map(SpriteChange::SrcRect),
        3 => (coordinate(), coordinate()).prop_map(|(x, y)| SpriteChange::Position(x, y)),
        1 => (coordinate(), coordinate()).prop_map(|(x, y)| SpriteChange::Origin(x, y)),
        1 => z().prop_map(SpriteChange::Z),
        2 => (zoom(), zoom()).prop_map(|(x, y)| SpriteChange::Zoom(x, y)),
        2 => angle().prop_map(SpriteChange::Angle),
        1 => any::<bool>().prop_map(SpriteChange::Mirror),
        1 => any::<bool>().prop_map(SpriteChange::Visible),
        1 => opacity().prop_map(SpriteChange::Opacity),
        1 => (-1..4i32).prop_map(SpriteChange::BlendType),
        1 => any::<[u8; 4]>().prop_map(SpriteChange::Color),
        1 => tone().prop_map(SpriteChange::Tone),
        1 => flash().prop_map(|(color, duration)| SpriteChange::Flash(color, duration)),
    ]
}

fn window_change() -> impl Strategy<Value = WindowChange> {
    prop_oneof![
        3 => proptest::option::weighted(0.9, any::<usize>()).prop_map(WindowChange::Skin),
        2 => proptest::option::weighted(0.9, any::<usize>())
            .prop_map(WindowChange::Contents),
        2 => (coordinate(), coordinate()).prop_map(|(x, y)| WindowChange::Position(x, y)),
        3 => (length(), length()).prop_map(|(w, h)| WindowChange::Size(w, h)),
        1 => (coordinate(), coordinate()).prop_map(|(x, y)| WindowChange::Origin(x, y)),
        1 => z().prop_map(WindowChange::Z),
        1 => any::<bool>().prop_map(WindowChange::Stretch),
        2 => rect().prop_map(WindowChange::Cursor),
        1 => any::<bool>().prop_map(WindowChange::Active),
        1 => any::<bool>().prop_map(WindowChange::Visible),
        1 => any::<bool>().prop_map(WindowChange::Pause),
        1 => (opacity(), opacity(), opacity())
            .prop_map(|(a, b, c)| WindowChange::Opacities(a, b, c)),
    ]
}

fn plane_change() -> impl Strategy<Value = PlaneChange> {
    prop_oneof![
        3 => proptest::option::weighted(0.9, any::<usize>()).prop_map(PlaneChange::Bitmap),
        3 => (coordinate(), coordinate()).prop_map(|(x, y)| PlaneChange::Origin(x, y)),
        1 => z().prop_map(PlaneChange::Z),
        3 => (zoom(), zoom()).prop_map(|(x, y)| PlaneChange::Zoom(x, y)),
        1 => any::<bool>().prop_map(PlaneChange::Visible),
        1 => opacity().prop_map(PlaneChange::Opacity),
        1 => (-1..4i32).prop_map(PlaneChange::BlendType),
        1 => any::<[u8; 4]>().prop_map(PlaneChange::Color),
        1 => tone().prop_map(PlaneChange::Tone),
    ]
}

fn tilemap_change() -> impl Strategy<Value = TilemapChange> {
    let bitmap = || proptest::option::weighted(0.9, any::<usize>());
    let table = || proptest::option::weighted(0.9, any::<usize>());
    prop_oneof![
        2 => bitmap().prop_map(TilemapChange::Tileset),
        3 => (0..7usize, bitmap())
            .prop_map(|(slot, bitmap)| TilemapChange::Autotile(slot, bitmap)),
        2 => table().prop_map(TilemapChange::MapData),
        2 => table().prop_map(TilemapChange::Priorities),
        1 => table().prop_map(TilemapChange::FlashData),
        4 => (coordinate(), coordinate()).prop_map(|(x, y)| TilemapChange::Origin(x, y)),
        1 => any::<bool>().prop_map(TilemapChange::Visible),
    ]
}

/// The size of a tileset or of an autotile sheet, whole or undersized.
fn sheet_size() -> impl Strategy<Value = (u32, u32)> {
    prop_oneof![
        Just((256, 96)),
        Just((96, 128)),
        Just((288, 128)),
        Just((32, 32)),
        Just((96, 32)),
        Just((70, 50)),
        Just((200, 16)),
    ]
}

fn step() -> impl Strategy<Value = Step> {
    let viewport = || proptest::option::of(any::<usize>());
    prop_oneof![
        3 => (1..40u32, 1..40u32, any::<u8>())
            .prop_map(|(width, height, seed)| Step::CreateBitmap(width, height, seed)),
        1 => any::<u8>().prop_map(Step::CreateSkin),
        1 => any::<usize>().prop_map(Step::DisposeBitmap),
        2 => (any::<usize>(), rect(), any::<[u8; 4]>())
            .prop_map(|(bitmap, rect, pixel)| Step::Fill(bitmap, rect, pixel)),
        2 => rect().prop_map(Step::CreateViewport),
        1 => any::<usize>().prop_map(Step::DisposeViewport),
        4 => (any::<usize>(), viewport_change())
            .prop_map(|(index, change)| Step::Viewport(index, change)),
        3 => viewport().prop_map(Step::CreateSprite),
        1 => any::<usize>().prop_map(Step::DisposeSprite),
        10 => (any::<usize>(), sprite_change())
            .prop_map(|(index, change)| Step::Sprite(index, change)),
        2 => viewport().prop_map(Step::CreateWindow),
        1 => any::<usize>().prop_map(Step::DisposeWindow),
        10 => (any::<usize>(), window_change())
            .prop_map(|(index, change)| Step::Window(index, change)),
        2 => (sheet_size(), any::<u8>())
            .prop_map(|((width, height), seed)| Step::CreateSheet(width, height, seed)),
        2 => viewport().prop_map(Step::CreatePlane),
        1 => any::<usize>().prop_map(Step::DisposePlane),
        8 => (any::<usize>(), plane_change())
            .prop_map(|(index, change)| Step::Plane(index, change)),
        2 => (0..7i32, 0..7i32, any::<u32>())
            .prop_map(|(width, height, seed)| Step::CreateMap(width, height, seed)),
        1 => any::<u32>().prop_map(Step::CreatePriorities),
        3 => (any::<usize>(), 0..440i32, 0..7i32, 0..4i32, any::<i16>())
            .prop_map(|(table, x, y, z, value)| Step::SetCell(table, x, y, z, value)),
        2 => viewport().prop_map(Step::CreateTilemap),
        1 => any::<usize>().prop_map(Step::DisposeTilemap),
        8 => (any::<usize>(), tilemap_change())
            .prop_map(|(index, change)| Step::Tilemap(index, change)),
        3 => Just(Step::Update),
    ]
}

/// The elements a random script has made. Disposed ones stay listed, so that the script
/// also works with stale handles.
#[derive(Default)]
struct Made {
    bitmaps: Vec<BitmapId>,
    viewports: Vec<ViewportId>,
    sprites: Vec<SpriteId>,
    windows: Vec<WindowId>,
    planes: Vec<PlaneId>,
    tilemaps: Vec<TilemapId>,
    tables: Vec<SharedTable>,
}

/// Numbers that follow from `seed`, for filling tables.
fn numbers(seed: u32) -> impl Iterator<Item = u32> {
    let mut state = seed;
    std::iter::repeat_with(move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        state >> 16
    })
}

fn pick<T: Copy>(made: &[T], index: usize) -> Option<T> {
    (!made.is_empty()).then(|| made[index % made.len()])
}

fn perform(stage: &mut Stage, made: &mut Made, step: Step) {
    let rgba =
        |[r, g, b, a]: [u8; 4]| Color::new(f64::from(r), f64::from(g), f64::from(b), f64::from(a));
    let toned = |[r, g, b, gray]: [i16; 4]| {
        Tone::new(f64::from(r), f64::from(g), f64::from(b), f64::from(gray))
    };
    let scene = &mut stage.scene;
    match step {
        Step::CreateBitmap(width, height, seed) => {
            let pixels = crate::bitmap::testing::pattern(width, height, seed);
            let id = stage
                .bitmaps
                .create_from_rgba(width, height, pixels, &defaults());
            made.bitmaps.push(id.unwrap());
        }
        Step::CreateSkin(seed) => {
            let pixels = crate::bitmap::testing::pattern(192, 128, seed);
            let id = stage
                .bitmaps
                .create_from_rgba(192, 128, pixels, &defaults());
            made.bitmaps.push(id.unwrap());
        }
        Step::DisposeBitmap(index) => {
            if let Some(id) = pick(&made.bitmaps, index) {
                stage.bitmaps.dispose(id);
            }
        }
        Step::Fill(index, rect, pixel) => {
            if let Some(id) = pick(&made.bitmaps, index) {
                let _ = stage.bitmaps.fill_rect(id, rect, color(pixel));
            }
        }
        Step::CreateViewport(rect) => {
            made.viewports.push(scene.create_viewport(rect).unwrap());
        }
        Step::DisposeViewport(index) => {
            if let Some(id) = pick(&made.viewports, index) {
                scene.dispose_viewport(id, &mut stage.bitmaps);
            }
        }
        Step::Viewport(index, change) => {
            let Some(id) = pick(&made.viewports, index) else {
                return;
            };
            if let ViewportChange::Z(z) = change {
                let _ = scene.set_viewport_z(id, z);
                return;
            }
            let Ok(viewport) = scene.viewport_mut(id) else {
                return;
            };
            match change {
                ViewportChange::Rect(rect) => viewport.rect = rect,
                ViewportChange::Origin(x, y) => (viewport.ox, viewport.oy) = (x, y),
                ViewportChange::Z(_) => {}
                ViewportChange::Visible(visible) => viewport.visible = visible,
                ViewportChange::Color(pixel) => viewport.color = rgba(pixel),
                ViewportChange::Tone(tone) => viewport.tone = toned(tone),
                ViewportChange::Flash(pixel, duration) => {
                    viewport.flash(pixel.map(rgba), duration);
                }
            }
        }
        Step::CreateSprite(viewport) => {
            let viewport = viewport.and_then(|index| pick(&made.viewports, index));
            if let Ok(id) = scene.create_sprite(viewport) {
                made.sprites.push(id);
            }
        }
        Step::DisposeSprite(index) => {
            if let Some(id) = pick(&made.sprites, index) {
                scene.dispose_sprite(id);
            }
        }
        Step::Sprite(index, change) => {
            let Some(id) = pick(&made.sprites, index) else {
                return;
            };
            if let SpriteChange::Z(z) = change {
                let _ = scene.set_sprite_z(id, z);
                return;
            }
            let Ok(sprite) = scene.sprite_mut(id) else {
                return;
            };
            match change {
                SpriteChange::Bitmap(bitmap) => {
                    let bitmap = bitmap.and_then(|index| pick(&made.bitmaps, index));
                    sprite.set_bitmap(bitmap, &stage.bitmaps);
                }
                SpriteChange::SrcRect(rect) => sprite.src_rect = rect,
                SpriteChange::Position(x, y) => (sprite.x, sprite.y) = (x, y),
                SpriteChange::Origin(x, y) => (sprite.ox, sprite.oy) = (x, y),
                SpriteChange::Z(_) => {}
                SpriteChange::Zoom(x, y) => (sprite.zoom_x, sprite.zoom_y) = (x, y),
                SpriteChange::Angle(angle) => sprite.angle = angle,
                SpriteChange::Mirror(mirror) => sprite.mirror = mirror,
                SpriteChange::Visible(visible) => sprite.visible = visible,
                SpriteChange::Opacity(opacity) => sprite.set_opacity(opacity),
                SpriteChange::BlendType(blend_type) => sprite.blend_type = blend_type,
                SpriteChange::Color(pixel) => sprite.color = rgba(pixel),
                SpriteChange::Tone(tone) => sprite.tone = toned(tone),
                SpriteChange::Flash(pixel, duration) => {
                    sprite.flash(pixel.map(rgba), duration);
                }
            }
        }
        Step::CreateWindow(viewport) => {
            let viewport = viewport.and_then(|index| pick(&made.viewports, index));
            if let Ok(id) = scene.create_window(viewport) {
                made.windows.push(id);
            }
        }
        Step::DisposeWindow(index) => {
            if let Some(id) = pick(&made.windows, index) {
                scene.dispose_window(id, &mut stage.bitmaps);
            }
        }
        Step::Window(index, change) => {
            let Some(id) = pick(&made.windows, index) else {
                return;
            };
            if let WindowChange::Z(z) = change {
                let _ = scene.set_window_z(id, z);
                return;
            }
            let Ok(window) = scene.window_mut(id) else {
                return;
            };
            match change {
                WindowChange::Skin(bitmap) => {
                    window.windowskin = bitmap.and_then(|index| pick(&made.bitmaps, index));
                }
                WindowChange::Contents(bitmap) => {
                    window.contents = bitmap.and_then(|index| pick(&made.bitmaps, index));
                }
                WindowChange::Position(x, y) => (window.x, window.y) = (x, y),
                WindowChange::Size(width, height) => {
                    (window.width, window.height) = (width, height);
                }
                WindowChange::Origin(x, y) => (window.ox, window.oy) = (x, y),
                WindowChange::Z(_) => {}
                WindowChange::Stretch(stretch) => window.stretch = stretch,
                WindowChange::Cursor(rect) => window.cursor_rect = rect,
                WindowChange::Active(active) => window.active = active,
                WindowChange::Visible(visible) => window.visible = visible,
                WindowChange::Pause(pause) => window.pause = pause,
                WindowChange::Opacities(opacity, back, contents) => {
                    window.set_opacity(opacity);
                    window.set_back_opacity(back);
                    window.set_contents_opacity(contents);
                }
            }
        }
        Step::CreateSheet(width, height, seed) => {
            let pixels = crate::bitmap::testing::pattern(width, height, seed);
            let id = stage
                .bitmaps
                .create_from_rgba(width, height, pixels, &defaults());
            made.bitmaps.push(id.unwrap());
        }
        Step::CreatePlane(viewport) => {
            let viewport = viewport.and_then(|index| pick(&made.viewports, index));
            if let Ok(id) = scene.create_plane(viewport) {
                made.planes.push(id);
            }
        }
        Step::DisposePlane(index) => {
            if let Some(id) = pick(&made.planes, index) {
                scene.dispose_plane(id, &mut stage.bitmaps);
            }
        }
        Step::Plane(index, change) => {
            let Some(id) = pick(&made.planes, index) else {
                return;
            };
            if let PlaneChange::Z(z) = change {
                let _ = scene.set_plane_z(id, z);
                return;
            }
            let Ok(plane) = scene.plane_mut(id) else {
                return;
            };
            match change {
                PlaneChange::Bitmap(bitmap) => {
                    plane.bitmap = bitmap.and_then(|index| pick(&made.bitmaps, index));
                }
                PlaneChange::Origin(x, y) => (plane.ox, plane.oy) = (x, y),
                PlaneChange::Z(_) => {}
                PlaneChange::Zoom(x, y) => (plane.zoom_x, plane.zoom_y) = (x, y),
                PlaneChange::Visible(visible) => plane.visible = visible,
                PlaneChange::Opacity(opacity) => plane.set_opacity(opacity),
                PlaneChange::BlendType(blend_type) => plane.blend_type = blend_type,
                PlaneChange::Color(pixel) => plane.color = rgba(pixel),
                PlaneChange::Tone(tone) => plane.tone = toned(tone),
            }
        }
        Step::CreateMap(width, height, seed) => {
            // Tiles of every kind: empty, autotiles, the tileset, now and then anything.
            let mut table = Table::new_3d(width, height, 3).unwrap();
            let mut numbers = numbers(seed);
            for z in 0..3 {
                for y in 0..height {
                    for x in 0..width {
                        let n = numbers.next().unwrap_or(0);
                        let id = match n % 8 {
                            0 => 0,
                            1..=3 => 48 + n / 8 % 336,
                            4..=6 => 384 + n / 8 % 32,
                            _ => n / 8,
                        };
                        table.set(x, y, z, i64::from(id));
                    }
                }
            }
            made.tables.push(SharedTable::new(table));
        }
        Step::CreatePriorities(seed) => {
            let mut table = Table::new(430).unwrap();
            for (id, n) in (0..430).zip(numbers(seed)) {
                table.set(id, 0, 0, i64::from(n % 9) - 1);
            }
            made.tables.push(SharedTable::new(table));
        }
        Step::SetCell(index, x, y, z, value) => {
            if let Some(table) = made.tables.get(index % made.tables.len().max(1)) {
                table.set(x, y, z, i64::from(value));
            }
        }
        Step::CreateTilemap(viewport) => {
            let viewport = viewport.and_then(|index| pick(&made.viewports, index));
            if let Ok(id) = scene.create_tilemap(viewport) {
                made.tilemaps.push(id);
            }
        }
        Step::DisposeTilemap(index) => {
            if let Some(id) = pick(&made.tilemaps, index) {
                scene.dispose_tilemap(id, &mut stage.bitmaps);
            }
        }
        Step::Tilemap(index, change) => {
            let Some(id) = pick(&made.tilemaps, index) else {
                return;
            };
            let Ok(tilemap) = scene.tilemap_mut(id) else {
                return;
            };
            let bitmap = |index: Option<usize>| index.and_then(|i| pick(&made.bitmaps, i));
            let table = |index: Option<usize>| {
                let index = index.filter(|_| !made.tables.is_empty())?;
                Some(made.tables[index % made.tables.len()].clone())
            };
            match change {
                TilemapChange::Tileset(index) => tilemap.tileset = bitmap(index),
                TilemapChange::Autotile(slot, index) => {
                    tilemap.autotiles[slot] = bitmap(index);
                }
                TilemapChange::MapData(index) => tilemap.map_data = table(index),
                TilemapChange::Priorities(index) => tilemap.priorities = table(index),
                TilemapChange::FlashData(index) => tilemap.flash_data = table(index),
                TilemapChange::Origin(x, y) => (tilemap.ox, tilemap.oy) = (x, y),
                TilemapChange::Visible(visible) => tilemap.visible = visible,
            }
        }
        Step::Update => {
            for &id in &made.sprites {
                if let Ok(sprite) = scene.sprite_mut(id) {
                    sprite.update();
                }
            }
            for &id in &made.viewports {
                if let Ok(viewport) = scene.viewport_mut(id) {
                    viewport.update();
                }
            }
            for &id in &made.windows {
                if let Ok(window) = scene.window_mut(id) {
                    window.update();
                }
            }
            // Enough updates for an autotile to show its next frame now and then.
            for &id in &made.tilemaps {
                if let Ok(tilemap) = scene.tilemap_mut(id) {
                    (0..8).for_each(|_| tilemap.update());
                }
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn the_reference_compositor_renders_every_extracted_packet(
        steps in proptest::collection::vec(step(), 1..50),
    ) {
        let mut stage = Stage::sized(40, 30);
        let mut made = Made::default();
        // Something to look at from the first frame on: sprites inside and outside a
        // viewport, a window with everything it can show, a plane, and a tilemap with
        // a tileset, autotile sheets, a map and priorities.
        for step in [
            Step::CreateBitmap(12, 9, 1),
            Step::CreateBitmap(70, 50, 2),
            Step::CreateSkin(3),
            Step::CreateViewport(Rect::new(4, 3, 30, 20)),
            Step::CreateSprite(None),
            Step::Sprite(0, SpriteChange::Bitmap(Some(0))),
            Step::CreateSprite(Some(0)),
            Step::Sprite(1, SpriteChange::Bitmap(Some(1))),
            Step::CreateWindow(None),
            Step::Window(0, WindowChange::Skin(Some(2))),
            Step::Window(0, WindowChange::Contents(Some(1))),
            Step::Window(0, WindowChange::Size(38, 36)),
            Step::Window(0, WindowChange::Cursor(Rect::new(0, 0, 6, 4))),
            Step::Window(0, WindowChange::Pause(true)),
            Step::CreatePlane(None),
            Step::Plane(0, PlaneChange::Bitmap(Some(0))),
            Step::CreateSheet(256, 96, 4),
            Step::CreateSheet(288, 128, 5),
            Step::CreateSheet(96, 32, 6),
            Step::CreateMap(5, 4, 7),
            Step::CreatePriorities(8),
            Step::CreateTilemap(Some(0)),
            Step::Tilemap(0, TilemapChange::Tileset(Some(3))),
            Step::Tilemap(0, TilemapChange::Autotile(0, Some(4))),
            Step::Tilemap(0, TilemapChange::Autotile(1, Some(5))),
            Step::Tilemap(0, TilemapChange::MapData(Some(0))),
            Step::Tilemap(0, TilemapChange::Priorities(Some(1))),
        ] {
            perform(&mut stage, &mut made, step);
        }
        for step in steps {
            perform(&mut stage, &mut made, step.clone());
            let packet = stage.scene.extract(&mut stage.bitmaps);
            let rendered = stage.render(&packet);
            prop_assert!(rendered.is_ok(), "after {:?}: {:?}", step, rendered.err());
            // Every delta is for a bitmap an op draws, or destroys one that is gone.
            for delta in &packet.bitmaps {
                let (BitmapDelta::Create { id, .. }
                | BitmapDelta::Update { id, .. }
                | BitmapDelta::Destroy { id }) = delta;
                let destroyed = matches!(delta, BitmapDelta::Destroy { .. });
                let drawn = packet.ops.iter().any(|op| match op {
                    vkrgss_proto::Op::Quad(quad) => quad.bitmap == *id,
                    vkrgss_proto::Op::TilemapLayer(layer) => {
                        let sheets = layer.autotiles.iter().chain([&layer.tileset]);
                        sheets.flatten().any(|sheet| sheet == id)
                    }
                    _ => false,
                });
                prop_assert_eq!(drawn, !destroyed, "{:?} after {:?}", id, step);
            }
        }
    }
}

mod rgss3;
