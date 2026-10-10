use vkxp_compose_ref::Frame;
use vkxp_pixel::Rgba;

use super::*;
use crate::bitmap::testing::{color, defaults};
use crate::extract::testing::*;
use crate::{Color, WindowId};

const BACK: Rgba = [0, 0, 160, 255];
const TOP_LEFT: Rgba = [255, 0, 0, 255];
const TOP_RIGHT: Rgba = [0, 255, 0, 255];
const BOTTOM_LEFT: Rgba = [255, 255, 0, 255];
const BOTTOM_RIGHT: Rgba = [255, 0, 255, 255];
const TOP: Rgba = [90, 10, 10, 255];
const BOTTOM: Rgba = [10, 90, 10, 255];
const LEFT: Rgba = [90, 90, 10, 255];
const RIGHT: Rgba = [90, 10, 90, 255];
const ARROW_UP: Rgba = [1, 200, 200, 255];
const ARROW_DOWN: Rgba = [2, 200, 200, 255];
const ARROW_LEFT: Rgba = [3, 200, 200, 255];
const ARROW_RIGHT: Rgba = [4, 200, 200, 255];
const CURSOR_EDGE: Rgba = [250, 250, 250, 255];
const CURSOR_FILL: Rgba = [120, 120, 120, 255];
const PAUSE_IMAGES: [Rgba; 4] = [
    [11, 50, 50, 255],
    [22, 50, 50, 255],
    [33, 50, 50, 255],
    [44, 50, 50, 255],
];

/// A window skin of 192 by 128 pixels in which every part has a colour of its own.
fn skin(stage: &mut Stage) -> BitmapId {
    let id = stage.bitmaps.create(192, 128, &defaults()).unwrap();
    let mut fill = |x, y, width, height, pixel| {
        stage
            .bitmaps
            .fill_rect(id, Rect::new(x, y, width, height), color(pixel))
            .unwrap();
    };
    fill(0, 0, 128, 128, BACK);
    fill(144, 0, 32, 16, TOP);
    fill(144, 48, 32, 16, BOTTOM);
    fill(128, 16, 16, 32, LEFT);
    fill(176, 16, 16, 32, RIGHT);
    fill(128, 0, 16, 16, TOP_LEFT);
    fill(176, 0, 16, 16, TOP_RIGHT);
    fill(128, 48, 16, 16, BOTTOM_LEFT);
    fill(176, 48, 16, 16, BOTTOM_RIGHT);
    fill(152, 16, 16, 8, ARROW_UP);
    fill(152, 40, 16, 8, ARROW_DOWN);
    fill(144, 24, 8, 16, ARROW_LEFT);
    fill(168, 24, 8, 16, ARROW_RIGHT);
    fill(128, 64, 32, 32, CURSOR_EDGE);
    fill(130, 66, 28, 28, CURSOR_FILL);
    fill(160, 64, 16, 16, PAUSE_IMAGES[0]);
    fill(176, 64, 16, 16, PAUSE_IMAGES[1]);
    fill(160, 80, 16, 16, PAUSE_IMAGES[2]);
    fill(176, 80, 16, 16, PAUSE_IMAGES[3]);
    id
}

/// A skin that has nothing but a background, each pixel of which differs from its
/// neighbours.
fn patterned_skin(stage: &mut Stage) -> BitmapId {
    let mut rgba = vec![0; 192 * 128 * 4];
    for y in 0..128 {
        for x in 0..128 {
            let offset = (y * 192 + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&pattern(x as u32, y as u32));
        }
    }
    stage.bitmap(192, 128, rgba)
}

/// The pixel at (`x`, `y`) of the background of [`patterned_skin`].
fn pattern(x: u32, y: u32) -> Rgba {
    [x as u8 + 1, y as u8 + 1, 99, 255]
}

/// The window of the scenarios: 160 by 96 at 0, 0, with `skin`.
fn window(stage: &mut Stage, skin: Option<BitmapId>) -> WindowId {
    let id = stage.scene.create_window(None).unwrap();
    let window = stage.scene.window_mut(id).unwrap();
    window.windowskin = skin;
    window.width = 160;
    window.height = 96;
    id
}

/// An opaque bitmap whose every pixel differs from its neighbours.
fn contents(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&content(x, y));
        }
    }
    stage.bitmap(width, height, rgba)
}

/// The pixel at (`x`, `y`) of a [`contents`] bitmap.
fn content(x: u32, y: u32) -> Rgba {
    [(x % 251) as u8 + 1, (y % 241) as u8 + 1, 7, 255]
}

/// Number of pixels of the frame that are not black.
fn drawn(frame: &Frame) -> usize {
    let pixels = frame.rgba.as_chunks::<4>().0;
    pixels.iter().filter(|pixel| **pixel != BLACK).count()
}

#[test]
fn the_background_fills_the_window_inside_a_margin_of_two() {
    let mut stage = Stage::new();
    // Only a background: the frame area of the skin is transparent.
    let skin = stage.bitmaps.create(192, 128, &defaults()).unwrap();
    stage
        .bitmaps
        .fill_rect(skin, Rect::new(0, 0, 128, 128), color(BACK))
        .unwrap();
    window(&mut stage, Some(skin));
    let frame = stage.frame();
    for y in 0..100 {
        for x in 0..164 {
            let inside = (2..=157).contains(&x) && (2..=93).contains(&y);
            let expected = if inside { BACK } else { BLACK };
            assert_eq!(frame.pixel(x, y), expected, "({x}, {y})");
        }
    }
}

#[test]
fn the_background_shows_between_the_frame() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    window(&mut stage, Some(skin));
    let frame = stage.frame();
    for y in 16..80 {
        for x in 16..144 {
            assert_eq!(frame.pixel(x, y), BACK, "({x}, {y})");
        }
    }
}

#[test]
fn the_frame_is_built_from_the_corners_and_edges_of_the_skin() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    let window = stage.scene.window_mut(id).unwrap();
    window.x = 40;
    window.y = 30;
    let frame = stage.frame();
    let at = |x: u32, y: u32| frame.pixel(40 + x, 30 + y);
    for (x, y, expected) in [
        (0, 0, TOP_LEFT),
        (15, 15, TOP_LEFT),
        (159, 0, TOP_RIGHT),
        (144, 15, TOP_RIGHT),
        (0, 95, BOTTOM_LEFT),
        (15, 80, BOTTOM_LEFT),
        (159, 95, BOTTOM_RIGHT),
        (144, 80, BOTTOM_RIGHT),
        (16, 0, TOP),
        (143, 15, TOP),
        (80, 7, TOP),
        (16, 80, BOTTOM),
        (143, 95, BOTTOM),
        (0, 16, LEFT),
        (15, 79, LEFT),
        (144, 16, RIGHT),
        (159, 79, RIGHT),
        (16, 16, BACK),
        (143, 79, BACK),
    ] {
        assert_eq!(at(x, y), expected, "({x}, {y})");
    }
    assert_eq!(frame.pixel(39, 30), BLACK);
    assert_eq!(frame.pixel(200, 30), BLACK);
    assert_eq!(frame.pixel(40, 126), BLACK);
    assert_eq!(drawn(&frame), 160 * 96);
}

#[test]
fn frame_edges_repeat_their_piece_of_the_skin() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    // Mark the first column of the top piece and the first row of the left piece.
    stage
        .bitmaps
        .fill_rect(skin, Rect::new(144, 0, 1, 16), color(WHITE))
        .unwrap();
    stage
        .bitmaps
        .fill_rect(skin, Rect::new(128, 16, 16, 1), color(WHITE))
        .unwrap();
    let id = window(&mut stage, Some(skin));
    stage.scene.window_mut(id).unwrap().width = 100;
    let frame = stage.frame();
    // The pieces start 8 pixels inside the window, under the corner.
    for x in 16..84 {
        let expected = if (x - 8) % 32 == 0 { WHITE } else { TOP };
        assert_eq!(frame.pixel(x, 3), expected, "x {x}");
    }
    assert_eq!(frame.pixel(84, 3), TOP_RIGHT);
    for y in 16..80 {
        let expected = if (y - 8) % 32 == 0 { WHITE } else { LEFT };
        assert_eq!(frame.pixel(3, y), expected, "y {y}");
    }
    assert_eq!(frame.pixel(3, 80), BOTTOM_LEFT);
}

#[test]
fn back_opacity_fades_the_background_alone() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    stage.scene.window_mut(id).unwrap().set_back_opacity(0);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), TOP_LEFT);
    assert_eq!(frame.pixel(80, 0), TOP);
    assert_eq!(frame.pixel(80, 48), BLACK);
    assert_eq!(frame.pixel(16, 16), BLACK);

    stage.scene.window_mut(id).unwrap().set_back_opacity(100);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), TOP_LEFT);
    assert_eq!(
        frame.pixel(80, 48),
        vkxp_pixel::blend_window(BLACK, BACK, 100)
    );
}

#[test]
fn opacity_fades_frame_and_background_but_not_the_contents() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    let bitmap = contents(&mut stage, 128, 64);
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(bitmap);
    window.set_opacity(0);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), BLACK);
    assert_eq!(frame.pixel(80, 0), BLACK);
    assert_eq!(frame.pixel(2, 2), BLACK);
    assert_eq!(frame.pixel(16, 16), content(0, 0));
    assert_eq!(drawn(&frame), 128 * 64);

    // The background takes opacity times back opacity over 255.
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = None;
    window.set_opacity(200);
    window.set_back_opacity(100);
    let frame = stage.frame();
    assert_eq!(
        frame.pixel(0, 0),
        vkxp_pixel::blend_window(BLACK, TOP_LEFT, 200)
    );
    assert_eq!(
        frame.pixel(80, 48),
        vkxp_pixel::blend_window(BLACK, BACK, 78)
    );
}

#[test]
fn a_window_without_a_live_skin_draws_neither_frame_nor_background() {
    let mut stage = Stage::new();
    let id = window(&mut stage, None);
    let bitmap = contents(&mut stage, 128, 64);
    stage.scene.window_mut(id).unwrap().contents = Some(bitmap);
    let frame = stage.frame();
    assert_eq!(drawn(&frame), 128 * 64, "only the contents");

    let skin = skin(&mut stage);
    stage.scene.window_mut(id).unwrap().windowskin = Some(skin);
    assert_eq!(drawn(&stage.frame()), 160 * 96);
    stage.bitmaps.dispose(skin);
    assert_eq!(drawn(&stage.frame()), 128 * 64, "disposed skin");

    let skin = self::skin(&mut stage);
    stage.scene.window_mut(id).unwrap().windowskin = Some(skin);
    assert_eq!(drawn(&stage.frame()), 160 * 96);
    stage.scene.window_mut(id).unwrap().windowskin = None;
    assert_eq!(drawn(&stage.frame()), 128 * 64, "skin taken away");
}

#[test]
fn an_invisible_or_disposed_window_draws_nothing() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    let bitmap = contents(&mut stage, 128, 64);
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(bitmap);
    window.cursor_rect = Rect::new(0, 0, 32, 32);
    window.pause = true;
    assert_eq!(drawn(&stage.frame()), 160 * 96);
    stage.scene.window_mut(id).unwrap().visible = false;
    assert!(all(&stage.frame(), BLACK));
    stage.scene.window_mut(id).unwrap().visible = true;
    assert_eq!(drawn(&stage.frame()), 160 * 96);
    stage.scene.dispose_window(id, &mut stage.bitmaps);
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn the_stretch_flag_stretches_or_tiles_the_background() {
    let mut stage = Stage::new();
    let skin = patterned_skin(&mut stage);
    let id = stage.scene.create_window(None).unwrap();
    let window = stage.scene.window_mut(id).unwrap();
    window.windowskin = Some(skin);
    window.width = 204;
    window.height = 164;
    let stretched = stage.frame();
    for (x, y) in [(0, 0), (1, 0), (57, 3), (199, 159), (100, 80)] {
        // Larger than half the pattern both ways: 127 texels are stretched, not 128.
        let from_x = vkxp_pixel::stretch_index(x, 127, 200);
        let from_y = vkxp_pixel::stretch_index(y, 127, 160);
        assert_eq!(
            stretched.pixel(2 + x, 2 + y),
            pattern(from_x, from_y),
            "({x}, {y})"
        );
    }

    stage.scene.window_mut(id).unwrap().stretch = false;
    let tiled = stage.frame();
    assert_ne!(tiled, stretched);
    for (x, y) in [(0, 0), (127, 127), (128, 0), (199, 159), (130, 140)] {
        assert_eq!(
            tiled.pixel(2 + x, 2 + y),
            pattern(x % 128, y % 128),
            "({x}, {y})"
        );
    }
    assert_eq!(tiled.pixel(202, 2), BLACK);
    assert_eq!(tiled.pixel(2, 162), BLACK);

    stage.scene.window_mut(id).unwrap().stretch = true;
    assert_eq!(stage.frame(), stretched);
}

#[test]
fn changing_the_size_redraws_frame_and_background() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    let before = stage.frame();
    assert_eq!(before.pixel(159, 95), BOTTOM_RIGHT);

    let window = stage.scene.window_mut(id).unwrap();
    window.width = 200;
    window.height = 64;
    let after = stage.frame();
    assert_ne!(after, before);
    assert_eq!(after.pixel(199, 63), BOTTOM_RIGHT);
    assert_eq!(after.pixel(159, 63), BOTTOM);
    assert_eq!(after.pixel(100, 30), BACK);
    assert_eq!(after.pixel(100, 64), BLACK);
    assert_eq!(drawn(&after), 200 * 64);

    // Back at the first size, the first picture.
    let window = stage.scene.window_mut(id).unwrap();
    window.width = 160;
    window.height = 96;
    assert_eq!(stage.frame(), before);

    // Sizes without pixels show nothing and do not fail.
    for (width, height) in [(0, 96), (160, 0), (-5, 96), (160, -1), (4, 4), (1, 1)] {
        let window = stage.scene.window_mut(id).unwrap();
        window.width = width;
        window.height = height;
        let frame = stage.frame();
        let expected = (width.max(0) * height.max(0)) as usize;
        assert_eq!(drawn(&frame), expected, "{width} by {height}");
    }
}

#[test]
fn drawing_into_the_skin_redraws_the_window() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    stage.scene.window_mut(id).unwrap().stretch = false;
    let before = stage.frame();
    assert_eq!(before.pixel(16, 16), BACK);
    assert_eq!(before.pixel(0, 0), TOP_LEFT);

    // One pixel of the background pattern, and one of a frame corner.
    stage.bitmaps.set_pixel(skin, 14, 14, color(WHITE)).unwrap();
    let after = stage.frame();
    assert_eq!(after.pixel(16, 16), WHITE);
    assert_eq!(after.pixel(17, 16), BACK);
    stage.bitmaps.set_pixel(skin, 128, 0, color(WHITE)).unwrap();
    let after = stage.frame();
    assert_eq!(after.pixel(0, 0), WHITE);
    assert_eq!(after.pixel(1, 0), TOP_LEFT);

    // Another skin altogether.
    let other = stage.solid(192, 128, GREEN);
    stage.scene.window_mut(id).unwrap().windowskin = Some(other);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), GREEN);
    assert_eq!(frame.pixel(80, 48), GREEN);
}

#[test]
fn two_windows_share_a_skin() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let first = window(&mut stage, Some(skin));
    let second = window(&mut stage, Some(skin));
    stage.scene.window_mut(second).unwrap().x = 200;
    // The second window is hidden while the skin changes.
    stage.frame();
    stage.scene.window_mut(second).unwrap().visible = false;
    stage.bitmaps.set_pixel(skin, 128, 0, color(WHITE)).unwrap();
    assert_eq!(stage.frame().pixel(0, 0), WHITE);
    stage.scene.window_mut(second).unwrap().visible = true;
    let frame = stage.frame();
    assert_eq!(frame.pixel(200, 0), WHITE);
    assert!(!stage.scene.is_window_disposed(first));
}

#[test]
fn the_contents_are_clipped_to_the_area_inside_the_padding() {
    let mut stage = Stage::new();
    let id = window(&mut stage, None);
    let bitmap = contents(&mut stage, 300, 300);
    stage.scene.window_mut(id).unwrap().contents = Some(bitmap);
    let frame = stage.frame();
    assert_eq!(frame.pixel(16, 16), content(0, 0));
    assert_eq!(frame.pixel(143, 79), content(127, 63));
    assert_eq!(frame.pixel(15, 16), BLACK);
    assert_eq!(frame.pixel(16, 15), BLACK);
    assert_eq!(frame.pixel(144, 16), BLACK);
    assert_eq!(frame.pixel(16, 80), BLACK);
    assert_eq!(drawn(&frame), 128 * 64);

    // Smaller contents are not stretched.
    let small = contents(&mut stage, 10, 5);
    stage.scene.window_mut(id).unwrap().contents = Some(small);
    let frame = stage.frame();
    assert_eq!(frame.pixel(25, 20), content(9, 4));
    assert_eq!(drawn(&frame), 50);

    // Disposed contents draw nothing.
    stage.bitmaps.dispose(small);
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn the_origin_of_a_window_scrolls_its_contents() {
    let mut stage = Stage::new();
    let id = window(&mut stage, None);
    let bitmap = contents(&mut stage, 300, 300);
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(bitmap);
    window.oy = 32;
    let frame = stage.frame();
    assert_eq!(frame.pixel(16, 16), content(0, 32));
    assert_eq!(frame.pixel(143, 79), content(127, 95));
    assert_eq!(drawn(&frame), 128 * 64);

    let window = stage.scene.window_mut(id).unwrap();
    window.ox = 100;
    window.oy = -8;
    window.x = 50;
    window.y = 60;
    let frame = stage.frame();
    assert_eq!(frame.pixel(66, 84), content(100, 0));
    assert_eq!(frame.pixel(66, 83), BLACK);
    assert_eq!(frame.pixel(193, 139), content(227, 55));
    assert_eq!(drawn(&frame), 128 * 56);
}

#[test]
fn the_contents_opacity_fades_the_contents() {
    let mut stage = Stage::new();
    let id = window(&mut stage, None);
    let bitmap = stage.solid(128, 64, WHITE);
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(bitmap);
    window.set_contents_opacity(90);
    assert_eq!(
        stage.frame().pixel(16, 16),
        vkxp_pixel::blend_window(BLACK, WHITE, 90)
    );
    stage.scene.window_mut(id).unwrap().set_contents_opacity(0);
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn a_sprite_can_stand_between_frame_and_contents() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    // Contents that are opaque in their left half only.
    let bitmap = stage.bitmaps.create(128, 64, &defaults()).unwrap();
    stage
        .bitmaps
        .fill_rect(bitmap, Rect::new(0, 0, 64, 64), color(WHITE))
        .unwrap();
    stage.scene.window_mut(id).unwrap().contents = Some(bitmap);
    stage.scene.set_window_z(id, 100).unwrap();

    let green = stage.solid(160, 96, GREEN);
    let sprite = stage.scene.create_sprite(None).unwrap();
    stage
        .scene
        .sprite_mut(sprite)
        .unwrap()
        .set_bitmap(Some(green), &stage.bitmaps);

    stage.scene.set_sprite_z(sprite, 101).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), GREEN, "in front of the frame");
    assert_eq!(frame.pixel(100, 40), GREEN, "in front of the background");
    assert_eq!(frame.pixel(20, 40), WHITE, "behind the contents");

    stage.scene.set_sprite_z(sprite, 99).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), TOP_LEFT);
    assert_eq!(frame.pixel(100, 40), BACK);
    assert_eq!(frame.pixel(20, 40), WHITE);

    stage.scene.set_sprite_z(sprite, 103).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), GREEN);
    assert_eq!(frame.pixel(20, 40), GREEN);

    // At the z of the contents, the one given its z later is in front.
    stage.scene.set_sprite_z(sprite, 102).unwrap();
    assert_eq!(stage.frame().pixel(20, 40), GREEN);
    stage.scene.set_window_z(id, 0).unwrap();
    stage.scene.set_window_z(id, 100).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(20, 40), WHITE);
    assert_eq!(frame.pixel(100, 40), GREEN);
}

#[test]
fn a_window_in_a_viewport_is_offset_and_clipped() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let viewport = stage
        .scene
        .create_viewport(Rect::new(100, 50, 100, 60))
        .unwrap();
    stage.scene.viewport_mut(viewport).unwrap().ox = 10;
    let id = stage.scene.create_window(Some(viewport)).unwrap();
    let bitmap = contents(&mut stage, 128, 64);
    let window = stage.scene.window_mut(id).unwrap();
    window.windowskin = Some(skin);
    window.contents = Some(bitmap);
    window.width = 160;
    window.height = 96;
    window.cursor_rect = Rect::new(0, 0, 128, 64);
    window.pause = true;
    let frame = stage.frame();
    // The window's corner is at 90, 50: ten pixels of it are cut off at the left.
    assert_eq!(frame.pixel(100, 50), TOP_LEFT);
    assert_eq!(frame.pixel(105, 50), TOP_LEFT);
    assert_eq!(frame.pixel(106, 50), TOP);
    assert_eq!(frame.pixel(106, 66), content(0, 0));
    assert_eq!(frame.pixel(199, 109), content(93, 43));
    assert_eq!(frame.pixel(99, 50), BLACK);
    assert_eq!(drawn(&frame), 100 * 60, "nothing outside the viewport");
}

/// A window with the test skin and a cursor of 128 by 32 at 0, 32.
fn with_cursor(stage: &mut Stage) -> WindowId {
    let skin = skin(stage);
    let id = window(stage, Some(skin));
    stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 32, 128, 32);
    id
}

#[test]
fn the_cursor_covers_its_rectangle_inside_the_contents_area() {
    let mut stage = Stage::new();
    with_cursor(&mut stage);
    let frame = stage.frame();
    for y in 16..80 {
        for x in 16..144 {
            let inside = (48..80).contains(&y);
            let edge = inside && (!(18..142).contains(&x) || !(50..78).contains(&y));
            let expected = match (inside, edge) {
                (false, _) => BACK,
                (true, true) => CURSOR_EDGE,
                (true, false) => CURSOR_FILL,
            };
            assert_eq!(frame.pixel(x, y), expected, "({x}, {y})");
        }
    }
    // The frame around the contents area is untouched.
    assert_eq!(frame.pixel(15, 48), LEFT);
    assert_eq!(frame.pixel(144, 79), RIGHT);
}

#[test]
fn the_cursor_follows_its_rectangle_and_the_window() {
    let mut stage = Stage::new();
    let id = with_cursor(&mut stage);
    let window = stage.scene.window_mut(id).unwrap();
    window.x = 100;
    window.y = 200;
    window.cursor_rect = Rect::new(8, 4, 40, 10);
    // The window's origin scrolls the contents, not the cursor.
    window.ox = 5;
    window.oy = 5;
    let frame = stage.frame();
    assert_eq!(frame.pixel(124, 220), CURSOR_EDGE);
    assert_eq!(frame.pixel(126, 222), CURSOR_FILL);
    assert_eq!(frame.pixel(161, 227), CURSOR_FILL);
    assert_eq!(frame.pixel(163, 229), CURSOR_EDGE);
    assert_eq!(frame.pixel(123, 220), BACK);
    assert_eq!(frame.pixel(164, 229), BACK);
    assert_eq!(frame.pixel(124, 230), BACK);

    // Too narrow for a centre, it shows its borders, the right one over the left one.
    stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 0, 3, 32);
    let frame = stage.frame();
    assert_eq!(frame.pixel(116, 216), CURSOR_EDGE);
    assert_eq!(frame.pixel(117, 220), CURSOR_EDGE);
    assert_eq!(frame.pixel(118, 220), CURSOR_EDGE);
    assert_eq!(frame.pixel(119, 220), BACK);

    // It is cut off by the window, not by the contents area.
    stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(-16, -16, 200, 20);
    let frame = stage.frame();
    assert_eq!(frame.pixel(100, 200), CURSOR_EDGE);
    assert_eq!(frame.pixel(259, 210), CURSOR_FILL);
    assert_eq!(frame.pixel(260, 210), BLACK);
}

#[test]
fn the_cursor_is_behind_the_contents_and_in_front_of_the_background() {
    let mut stage = Stage::new();
    let id = with_cursor(&mut stage);
    // Contents that are opaque in their left half only.
    let bitmap = stage.bitmaps.create(128, 64, &defaults()).unwrap();
    stage
        .bitmaps
        .fill_rect(bitmap, Rect::new(0, 0, 64, 64), color(GREEN))
        .unwrap();
    stage.scene.window_mut(id).unwrap().contents = Some(bitmap);
    let frame = stage.frame();
    assert_eq!(frame.pixel(40, 60), GREEN);
    assert_eq!(frame.pixel(100, 60), CURSOR_FILL);
    assert_eq!(frame.pixel(100, 30), BACK);

    // It belongs to the upper part of the window: a sprite at z + 1 is behind it.
    let red = stage.solid(160, 96, RED);
    let sprite = stage.scene.create_sprite(None).unwrap();
    stage
        .scene
        .sprite_mut(sprite)
        .unwrap()
        .set_bitmap(Some(red), &stage.bitmaps);
    stage.scene.set_sprite_z(sprite, 1).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(100, 60), CURSOR_FILL);
    assert_eq!(frame.pixel(100, 30), RED);
}

/// The colour of the cursor's inside over the background at `alpha`.
fn blinking(alpha: u8) -> Rgba {
    vkxp_pixel::blend_window(BACK, CURSOR_FILL, alpha)
}

#[test]
fn the_cursor_of_an_active_window_blinks_over_32_updates() {
    let mut stage = Stage::new();
    let id = with_cursor(&mut stage);
    assert_eq!(stage.frame().pixel(80, 60), blinking(255));
    assert_eq!(
        stage.frame().pixel(80, 60),
        blinking(255),
        "no update, no blink"
    );

    let mut alphas = Vec::new();
    for _ in 0..32 {
        stage.scene.window_mut(id).unwrap().update();
        alphas.push(stage.scene.window(id).unwrap().cursor_opacity());
    }
    let falling: Vec<u8> = (1..=16).map(|step| 255 - 8 * step).collect();
    let rising: Vec<u8> = (1..=16).map(|step| 127 + 8 * step).collect();
    assert_eq!(alphas[..16], falling);
    assert_eq!(alphas[16..], rising);

    let mut stage = Stage::new();
    let id = with_cursor(&mut stage);
    for _ in 0..16 {
        stage.scene.window_mut(id).unwrap().update();
    }
    assert_eq!(stage.frame().pixel(80, 60), blinking(127));
    stage.scene.window_mut(id).unwrap().update();
    assert_eq!(stage.frame().pixel(80, 60), blinking(135));
    for _ in 0..15 {
        stage.scene.window_mut(id).unwrap().update();
    }
    assert_eq!(stage.frame().pixel(80, 60), blinking(255));
    assert_eq!(stage.frame().pixel(80, 60), CURSOR_FILL);
}

#[test]
fn the_cursor_of_an_inactive_window_stands_at_its_twelfth_step() {
    let mut stage = Stage::new();
    let id = with_cursor(&mut stage);
    for _ in 0..5 {
        stage.scene.window_mut(id).unwrap().update();
    }
    assert_eq!(stage.frame().pixel(80, 60), blinking(215));
    stage.scene.window_mut(id).unwrap().set_active(false);
    assert_eq!(stage.scene.window(id).unwrap().cursor_opacity(), 159);
    assert_eq!(stage.frame().pixel(80, 60), blinking(159));
    for _ in 0..7 {
        stage.scene.window_mut(id).unwrap().update();
        assert_eq!(stage.frame().pixel(80, 60), blinking(159));
    }
    // It goes on from there.
    stage.scene.window_mut(id).unwrap().set_active(true);
    assert_eq!(stage.frame().pixel(80, 60), blinking(159));
    stage.scene.window_mut(id).unwrap().update();
    assert_eq!(stage.frame().pixel(80, 60), blinking(151));
    // Active again without an update in between: from the twelfth step all the same.
    let window = stage.scene.window_mut(id).unwrap();
    window.set_active(false);
    window.set_active(true);
    window.update();
    assert_eq!(window.cursor_opacity(), 151);
    // Making an active window active changes nothing.
    window.set_active(true);
    assert_eq!(window.cursor_opacity(), 151);
}

#[test]
fn flags_assigned_as_fields_are_followed_at_the_next_frame_or_update() {
    let mut stage = Stage::new();
    let id = with_cursor(&mut stage);
    for _ in 0..5 {
        stage.scene.window_mut(id).unwrap().update();
    }
    stage.scene.window_mut(id).unwrap().active = false;
    assert_eq!(stage.frame().pixel(80, 60), blinking(159));
    stage.scene.window_mut(id).unwrap().active = true;
    stage.scene.window_mut(id).unwrap().update();
    assert_eq!(stage.frame().pixel(80, 60), blinking(151));

    let window = stage.scene.window_mut(id).unwrap();
    window.active = false;
    window.update();
    assert_eq!(window.cursor_opacity(), 159);

    // The pause sign starts over when the flag has changed.
    window.pause = true;
    for _ in 0..3 {
        window.update();
    }
    assert_eq!(window.pause_sign(), Some((96, 0)));
    window.pause = false;
    window.update();
    assert_eq!(window.pause_sign(), None);
    window.pause = true;
    window.update();
    assert_eq!(window.pause_sign(), Some((32, 0)));
}

#[test]
fn the_contents_opacity_is_what_the_blink_takes_from() {
    let mut stage = Stage::new();
    let id = with_cursor(&mut stage);
    stage
        .scene
        .window_mut(id)
        .unwrap()
        .set_contents_opacity(100);
    assert_eq!(stage.frame().pixel(80, 60), blinking(100));
    for _ in 0..4 {
        stage.scene.window_mut(id).unwrap().update();
    }
    assert_eq!(stage.frame().pixel(80, 60), blinking(68));
    for _ in 0..12 {
        stage.scene.window_mut(id).unwrap().update();
    }
    assert_eq!(stage.frame().pixel(80, 60), BACK);
    // Opacity and back opacity do not act on the cursor.
    let window = stage.scene.window_mut(id).unwrap();
    window.set_contents_opacity(255);
    window.set_opacity(0);
    assert_eq!(
        stage.frame().pixel(80, 60),
        vkxp_pixel::blend_window(BLACK, CURSOR_FILL, 127)
    );
}

#[test]
fn an_empty_cursor_rectangle_draws_no_cursor() {
    let mut stage = Stage::new();
    let id = with_cursor(&mut stage);
    let with = stage.frame();
    stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 32, 0, 32);
    let without = stage.frame();
    assert_ne!(without, with);
    for y in 16..80 {
        for x in 16..144 {
            assert_eq!(without.pixel(x, y), BACK, "({x}, {y})");
        }
    }
    for rect in [
        Rect::new(0, 32, 128, 0),
        Rect::new(0, 0, -4, 32),
        Rect::default(),
    ] {
        stage.scene.window_mut(id).unwrap().cursor_rect = rect;
        assert_eq!(stage.frame(), without, "{rect:?}");
    }
    stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 32, 128, 32);
    assert_eq!(stage.frame(), with);
}

#[test]
fn the_pause_sign_fades_in_and_steps_through_its_four_images() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    let without = stage.frame();
    assert_eq!(without.pixel(72, 80), BOTTOM);

    // Before the first update it is not drawn.
    stage.scene.window_mut(id).unwrap().set_pause(true);
    assert_eq!(stage.frame(), without);

    let sign =
        |alpha: u8, image: usize| vkxp_pixel::blend_window(BOTTOM, PAUSE_IMAGES[image], alpha);
    stage.scene.window_mut(id).unwrap().update();
    let frame = stage.frame();
    for y in 78..98 {
        for x in 70..90 {
            let inside = (72..88).contains(&x) && (80..96).contains(&y);
            let expected = if inside {
                sign(32, 0)
            } else {
                without.pixel(x, y)
            };
            assert_eq!(frame.pixel(x, y), expected, "({x}, {y})");
        }
    }
    stage.scene.window_mut(id).unwrap().update();
    assert_eq!(stage.frame().pixel(72, 80), sign(64, 0));
    for updates in 3..=7 {
        stage.scene.window_mut(id).unwrap().update();
        let alpha = 32 * updates;
        assert_eq!(stage.frame().pixel(72, 80), sign(alpha, 0), "{updates}");
    }
    // From the eighth update it is opaque and shows its second image.
    stage.scene.window_mut(id).unwrap().update();
    assert_eq!(stage.scene.window(id).unwrap().pause_sign(), Some((255, 1)));
    assert_eq!(stage.frame().pixel(72, 80), PAUSE_IMAGES[1]);
    assert_eq!(stage.frame().pixel(87, 95), PAUSE_IMAGES[1]);

    // Eight updates per image from then on.
    for round in 0..2 {
        for image in [2, 3, 0, 1] {
            for step in 0..8 {
                let previous = PAUSE_IMAGES[(image + 3) % 4];
                assert_eq!(
                    stage.frame().pixel(72, 80),
                    previous,
                    "{round} {image} {step}"
                );
                stage.scene.window_mut(id).unwrap().update();
            }
            assert_eq!(stage.frame().pixel(72, 80), PAUSE_IMAGES[image]);
        }
    }

    // No opacity acts on it.
    let window = stage.scene.window_mut(id).unwrap();
    window.set_opacity(0);
    window.set_back_opacity(0);
    window.set_contents_opacity(0);
    assert_eq!(stage.frame().pixel(72, 80), PAUSE_IMAGES[1]);
    let window = stage.scene.window_mut(id).unwrap();
    window.set_opacity(255);
    window.set_back_opacity(255);

    // Without the flag there is no sign at once, and it starts over when the flag
    // returns, with or without an update in between.
    stage.scene.window_mut(id).unwrap().set_pause(false);
    assert_eq!(stage.frame(), without);
    stage.scene.window_mut(id).unwrap().update();
    stage.scene.window_mut(id).unwrap().set_pause(true);
    assert_eq!(stage.frame(), without);
    stage.scene.window_mut(id).unwrap().update();
    assert_eq!(stage.frame().pixel(72, 80), sign(32, 0));
    let window = stage.scene.window_mut(id).unwrap();
    window.set_pause(false);
    window.set_pause(true);
    window.update();
    assert_eq!(window.pause_sign(), Some((32, 0)));
    // Setting the flag of a pausing window changes nothing.
    window.set_pause(true);
    window.update();
    assert_eq!(window.pause_sign(), Some((64, 0)));
}

#[test]
fn the_pause_sign_is_in_front_of_the_contents_and_the_down_arrow() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = stage.scene.create_window(None).unwrap();
    let bitmap = contents(&mut stage, 128, 200);
    let window = stage.scene.window_mut(id).unwrap();
    window.windowskin = Some(skin);
    window.contents = Some(bitmap);
    window.width = 160;
    window.height = 96;
    window.oy = -20;
    window.set_pause(true);
    for _ in 0..8 {
        window.update();
    }
    stage.scene.set_window_z(id, 10).unwrap();
    // A sprite between the two parts of the window is behind the sign.
    let green = stage.solid(160, 96, GREEN);
    let sprite = stage.scene.create_sprite(None).unwrap();
    stage
        .scene
        .sprite_mut(sprite)
        .unwrap()
        .set_bitmap(Some(green), &stage.bitmaps);
    stage.scene.set_sprite_z(sprite, 11).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(72, 80), PAUSE_IMAGES[1]);
    assert_eq!(frame.pixel(60, 80), GREEN);
    assert_eq!(frame.pixel(60, 70), content(44, 34));
    // The down arrow, 4 rows lower, is under the sign and shows nowhere.
    assert_eq!(frame.pixel(72, 84), PAUSE_IMAGES[1]);
    assert_eq!(frame.pixel(87, 91), PAUSE_IMAGES[1]);
}

/// Which arrows the frame shows: up, down, left, right.
fn arrows(frame: &Frame) -> [bool; 4] {
    let shows = |x: u32, y: u32, width: u32, height: u32, pixel: Rgba| {
        let first = frame.pixel(x, y) == pixel;
        let last = frame.pixel(x + width - 1, y + height - 1) == pixel;
        assert_eq!(first, last, "an arrow is drawn whole or not at all");
        first
    };
    [
        shows(72, 4, 16, 8, ARROW_UP),
        shows(72, 84, 16, 8, ARROW_DOWN),
        shows(4, 40, 8, 16, ARROW_LEFT),
        shows(148, 40, 8, 16, ARROW_RIGHT),
    ]
}

#[test]
fn arrows_show_where_the_contents_go_on() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    assert_eq!(arrows(&stage.frame()), [false; 4], "no contents");

    // Exactly the visible area: nothing is hidden.
    let fitting = contents(&mut stage, 128, 64);
    stage.scene.window_mut(id).unwrap().contents = Some(fitting);
    assert_eq!(arrows(&stage.frame()), [false; 4]);

    // Taller than the visible area.
    let tall = contents(&mut stage, 128, 200);
    stage.scene.window_mut(id).unwrap().contents = Some(tall);
    let frame = stage.frame();
    assert_eq!(arrows(&frame), [false, true, false, false]);
    assert_eq!(frame.pixel(71, 84), BOTTOM);
    assert_eq!(frame.pixel(88, 91), BOTTOM);
    assert_eq!(frame.pixel(72, 83), BOTTOM);
    assert_eq!(frame.pixel(72, 92), BOTTOM);

    stage.scene.window_mut(id).unwrap().oy = 32;
    assert_eq!(arrows(&stage.frame()), [true, true, false, false]);
    stage.scene.window_mut(id).unwrap().oy = 136;
    assert_eq!(arrows(&stage.frame()), [true, false, false, false]);
    stage.scene.window_mut(id).unwrap().oy = 135;
    assert_eq!(arrows(&stage.frame()), [true, true, false, false]);

    // Wider than the visible area.
    let wide = contents(&mut stage, 300, 64);
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(wide);
    window.oy = 0;
    assert_eq!(arrows(&stage.frame()), [false, false, false, true]);
    stage.scene.window_mut(id).unwrap().ox = 1;
    assert_eq!(arrows(&stage.frame()), [false, false, true, true]);
    stage.scene.window_mut(id).unwrap().ox = 172;
    assert_eq!(arrows(&stage.frame()), [false, false, true, false]);

    // Without a skin there is nothing to draw them from.
    stage.scene.window_mut(id).unwrap().windowskin = None;
    assert_eq!(arrows(&stage.frame()), [false; 4]);
}

#[test]
fn arrows_are_in_front_of_what_stands_between_the_parts_of_the_window() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    let tall = contents(&mut stage, 128, 200);
    stage.scene.window_mut(id).unwrap().contents = Some(tall);
    let green = stage.solid(160, 96, GREEN);
    let sprite = stage.scene.create_sprite(None).unwrap();
    stage
        .scene
        .sprite_mut(sprite)
        .unwrap()
        .set_bitmap(Some(green), &stage.bitmaps);
    stage.scene.set_sprite_z(sprite, 1).unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(72, 84), ARROW_DOWN);
    assert_eq!(frame.pixel(71, 84), GREEN);
}

#[test]
fn a_skin_smaller_than_the_layout_draws_what_it_has() {
    let mut stage = Stage::new();
    let small = stage.solid(40, 40, BACK);
    let id = window(&mut stage, Some(small));
    let tall = contents(&mut stage, 128, 200);
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(tall);
    window.cursor_rect = Rect::new(0, 0, 32, 32);
    window.pause = true;
    window.stretch = false;
    let frame = stage.frame();
    assert_eq!(frame.pixel(2, 2), BACK);
    assert_eq!(frame.pixel(0, 0), BLACK);
    assert_eq!(frame.pixel(41, 10), BACK);
    assert_eq!(frame.pixel(42, 10), BLACK);
    assert_eq!(frame.pixel(130, 10), BACK, "the next tile");
    assert_eq!(frame.pixel(72, 84), BLACK);
}

#[test]
fn cached_bitmaps_are_disposed_with_the_window() {
    let mut stage = Stage::new();
    let skin = skin(&mut stage);
    let bitmap = contents(&mut stage, 128, 200);
    let before = stage.bitmaps.len();
    assert_eq!(before, 2);

    let id = window(&mut stage, Some(skin));
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(bitmap);
    window.cursor_rect = Rect::new(0, 0, 128, 32);
    window.pause = true;
    stage.frame();
    assert_eq!(
        stage.bitmaps.len(),
        before + 3,
        "background, frame and cursor"
    );

    // Changes that redraw them do not let them pile up.
    let window = stage.scene.window_mut(id).unwrap();
    window.width = 200;
    window.stretch = false;
    window.cursor_rect = Rect::new(0, 0, 64, 32);
    stage.frame();
    stage
        .bitmaps
        .set_pixel(skin, 0, 0, Color::new(1.0, 2.0, 3.0, 255.0))
        .unwrap();
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 3);

    // A cursor rectangle without pixels gives its bitmap back.
    stage.scene.window_mut(id).unwrap().cursor_rect = Rect::default();
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 2);

    stage.scene.dispose_window(id, &mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), before);
    assert!(!stage.bitmaps.is_disposed(skin));
    assert!(!stage.bitmaps.is_disposed(bitmap));
    // The compositor is told, and accepts the frame that follows.
    assert!(all(&stage.frame(), BLACK));

    // Losing the skin gives them back as well.
    let id = self::window(&mut stage, Some(skin));
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 2);
    stage.scene.window_mut(id).unwrap().windowskin = None;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before);
}

// The samples measured on the original runtime.

const MEASURED: &str = include_str!("../../tests/data/window-samples.txt");

/// The samples of one kind, each split at its bars, without the kind.
fn samples(kind: &str) -> Vec<Vec<&'static str>> {
    let rows: Vec<Vec<&str>> = MEASURED
        .lines()
        .filter_map(|line| line.strip_prefix(kind)?.strip_prefix(' '))
        .map(|row| row.split('|').map(str::trim).collect())
        .collect();
    assert!(!rows.is_empty(), "no `{kind}` samples");
    rows
}

fn numbers(text: &str) -> Vec<i32> {
    text.split_whitespace()
        .map(|word| word.parse().unwrap_or_else(|_| panic!("a number: {word}")))
        .collect()
}

/// A list of `length` values of which the middle may be left out as "...": each value
/// that is listed, with its position.
fn listed(text: &str, length: i32) -> Vec<(i32, i32)> {
    // Some lists carry a caption that ends in a colon.
    let text = text.rsplit(':').next().unwrap();
    let mut parts = text.split("...");
    let first = numbers(parts.next().unwrap());
    let mut values: Vec<(i32, i32)> = (0..).zip(first).collect();
    match parts.next() {
        Some(rest) => {
            let rest = numbers(rest);
            let start = length - rest.len() as i32;
            assert!(start >= values.len() as i32, "{text}");
            values.extend((start..).zip(rest));
        }
        None => assert_eq!(values.len() as i32, length, "{text}"),
    }
    values
}

/// The skin of the traced probes, of which only the texels that `keep` names are there:
/// texel (x, y) has the colour (x, y, 255).
fn traced_skin(stage: &mut Stage, keep: impl Fn(u32, u32) -> bool) -> BitmapId {
    let mut rgba = Vec::with_capacity(192 * 128 * 4);
    for y in 0..128 {
        for x in 0..192 {
            let texel = if keep(x, y) { traced(x, y) } else { CLEAR };
            rgba.extend_from_slice(&texel);
        }
    }
    stage.bitmap(192, 128, rgba)
}

fn traced(x: u32, y: u32) -> Rgba {
    [x as u8, y as u8, 255, 255]
}

/// The contents of the probes: texel (x, y) has the colour (x mod 256, y mod 256, 100).
fn traced_contents(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&[x as u8, y as u8, 100, 255]);
        }
    }
    stage.bitmap(width, height, rgba)
}

/// A window of `width` by `height` at 0, 0.
fn sized(stage: &mut Stage, skin: BitmapId, width: i32, height: i32) -> WindowId {
    let id = stage.scene.create_window(None).unwrap();
    let window = stage.scene.window_mut(id).unwrap();
    window.windowskin = Some(skin);
    window.width = width;
    window.height = height;
    id
}

/// The smallest rectangle around the pixels of the frame that `wanted` names, as its
/// first and last column and row.
fn bounds(frame: &Frame, wanted: impl Fn(Rgba) -> bool) -> Option<[i32; 4]> {
    let mut bounds: Option<[i32; 4]> = None;
    for y in 0..frame.size.height {
        for x in 0..frame.size.width {
            if wanted(frame.pixel(x, y)) {
                let (x, y) = (x as i32, y as i32);
                let [x0, y0, x1, y1] = bounds.unwrap_or([x, y, x, y]);
                bounds = Some([x0.min(x), y0.min(y), x1.max(x), y1.max(y)]);
            }
        }
    }
    bounds
}

/// Checks the background of a window of each sampled size against the texels listed for
/// its first row and its first column.
fn background_shows_the_measured_texels(kind: &str, stretch: bool) {
    let mut stage = Stage::new();
    let skin = traced_skin(&mut stage, |x, _| x < 128);
    let id = sized(&mut stage, skin, 0, 0);
    stage.scene.window_mut(id).unwrap().stretch = stretch;
    for sample in samples(kind) {
        let &[width, height] = numbers(sample[0]).as_slice() else {
            panic!("{sample:?}");
        };
        let [across, down] = [sample[sample.len() - 2], sample[sample.len() - 1]];
        let across = listed(across, width - 4);
        let down = listed(down, height - 4);
        let window = stage.scene.window_mut(id).unwrap();
        (window.width, window.height) = (width, height);
        let frame = stage.frame();
        for &(i, x) in &across {
            let expected = traced(x as u32, down[0].1 as u32);
            assert_eq!(frame.pixel(2 + i as u32, 2), expected, "{sample:?} {i}");
        }
        for &(j, y) in &down {
            let expected = traced(across[0].1 as u32, y as u32);
            assert_eq!(frame.pixel(2, 2 + j as u32), expected, "{sample:?} {j}");
        }
        // Nothing in the margin of two pixels.
        let drawn = bounds(&frame, |pixel| pixel != BLACK);
        assert_eq!(drawn, Some([2, 2, width - 3, height - 3]), "{sample:?}");
    }
}

#[test]
fn the_stretched_background_shows_the_measured_texels() {
    background_shows_the_measured_texels("bg", true);
}

#[test]
fn the_tiled_background_shows_the_measured_texels() {
    background_shows_the_measured_texels("tiled", false);
}

#[test]
fn the_background_is_stretched_from_the_measured_number_of_texels() {
    let mut stage = Stage::sized(210, 210);
    let skin = traced_skin(&mut stage, |x, _| x < 128);
    let id = sized(&mut stage, skin, 0, 0);
    let mut seen = [false; 2];
    for sample in samples("bgnum") {
        for case in sample[0].split(';') {
            let &[width, height, part] = numbers(case).as_slice() else {
                panic!("{case}");
            };
            seen[usize::from(part == 127)] = true;
            let window = stage.scene.window_mut(id).unwrap();
            (window.width, window.height) = (width, height);
            let frame = stage.frame();
            // The last column and the last row tell the two apart.
            let last = |length: i32| {
                vkxp_pixel::stretch_index(length as u32 - 1, part as u32, length as u32)
            };
            let corner = frame.pixel(width as u32 - 3, height as u32 - 3);
            assert_eq!(corner, traced(last(width - 4), last(height - 4)), "{case}");
        }
    }
    assert_eq!(seen, [true; 2]);
}

#[test]
fn the_background_takes_the_measured_alpha() {
    const ALPHAS: [u8; 6] = [4, 60, 124, 131, 195, 255];
    let mut stage = Stage::sized(120, 20);
    let windows = ALPHAS.map(|alpha| {
        let mut skin = vec![0; 192 * 128 * 4];
        for texel in skin.as_chunks_mut::<4>().0 {
            *texel = [255, 255, 255, alpha];
        }
        let skin = stage.bitmap(192, 128, skin);
        // Only the background: nothing of the frame.
        stage
            .bitmaps
            .fill_rect(skin, Rect::new(128, 0, 64, 128), color(CLEAR))
            .unwrap();
        sized(&mut stage, skin, 20, 20)
    });
    for (index, &id) in windows.iter().enumerate() {
        stage.scene.window_mut(id).unwrap().x = 20 * index as i32;
    }
    for sample in samples("back") {
        let &[opacity, back_opacity] = numbers(sample[0]).as_slice() else {
            panic!("{sample:?}");
        };
        for &id in &windows {
            let window = stage.scene.window_mut(id).unwrap();
            window.set_opacity(opacity);
            window.set_back_opacity(back_opacity);
        }
        let frame = stage.frame();
        for (index, alpha) in numbers(sample[1]).into_iter().enumerate() {
            // White over black in the window blend is the alpha itself.
            let alpha = alpha as u8;
            let pixel = frame.pixel(20 * index as u32 + 10, 10);
            assert_eq!(pixel, [alpha, alpha, alpha, 255], "{sample:?} {index}");
        }
    }
}

#[test]
fn the_frame_shows_the_measured_texels() {
    let mut stage = Stage::sized(110, 110);
    let skin = traced_skin(&mut stage, |x, y| x >= 128 && y < 64);
    let id = sized(&mut stage, skin, 0, 0);
    // Along the top the texel's x is the same whatever the height, and down the left
    // side its y whatever the width.
    for (kind, channel) in [("frame-top", 0), ("frame-left", 1)] {
        let mut small = 0;
        for sample in samples(kind) {
            let side: i32 = sample[0].parse().unwrap();
            small += i32::from(side < 32);
            let window = stage.scene.window_mut(id).unwrap();
            (window.width, window.height) = (side, side);
            let frame = stage.frame();
            for (at, texel) in listed(sample[1], side) {
                let (x, y) = if channel == 0 { (at, 0) } else { (0, at) };
                let pixel = frame.pixel(x as u32, y as u32);
                assert_eq!(i32::from(pixel[channel]), texel, "{kind} {side} at {at}");
                assert_eq!(pixel[2], 255, "{kind} {side} at {at}");
            }
        }
        assert!(small >= 4, "windows smaller than 32 pixels");
    }
}

#[test]
fn contents_and_arrows_show_as_measured() {
    let mut stage = Stage::sized(160, 96);
    let skin = traced_skin(&mut stage, |_, _| true);
    let id = sized(&mut stage, skin, 0, 0);
    for sample in samples("contents") {
        let &[width, height, across, down, ox, oy] = numbers(sample[0]).as_slice() else {
            panic!("{sample:?}");
        };
        let bitmap = traced_contents(&mut stage, across as u32, down as u32);
        let window = stage.scene.window_mut(id).unwrap();
        (window.width, window.height) = (width, height);
        (window.ox, window.oy) = (ox, oy);
        let old = window.contents.replace(bitmap);
        if let Some(old) = old {
            stage.bitmaps.dispose(old);
        }
        let frame = stage.frame();
        let corner = frame.pixel(16, 16);
        match sample[1] {
            "none" => assert_eq!(corner[2], 255, "{sample:?}"),
            texel => {
                let texel: Vec<u8> = texel
                    .split(',')
                    .map(|word| word.parse::<i32>().unwrap() as u8)
                    .collect();
                assert_eq!(corner, [texel[0], texel[1], 100, 255], "{sample:?}");
            }
        }
        let (width, height) = (width as u32, height as u32);
        let arrows = [
            (4, height / 2 - 8, traced(144, 24)),
            (width / 2 - 8, 4, traced(152, 16)),
            (width - 12, height / 2 - 8, traced(168, 24)),
            (width / 2 - 8, height - 12, traced(152, 40)),
        ];
        let shown: Vec<i32> = arrows
            .iter()
            .map(|&(x, y, texel)| i32::from(frame.pixel(x, y) == texel))
            .collect();
        assert_eq!(shown, numbers(sample[2]), "{sample:?}");
    }
}

#[test]
fn arrows_sit_and_overlap_as_measured() {
    let mut stage = Stage::sized(300, 200);
    let skin = traced_skin(&mut stage, |_, _| true);
    let bitmap = traced_contents(&mut stage, 300, 200);
    let id = sized(&mut stage, skin, 0, 0);
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(bitmap);
    (window.ox, window.oy) = (20, 20);
    // The four arrows are the frame area's middle, which is nothing else's source.
    let arrow = |pixel: Rgba| {
        (144..176).contains(&pixel[0]) && (16..48).contains(&pixel[1]) && pixel[2] == 255
    };
    for sample in samples("arrows") {
        let &[width, height] = numbers(sample[0]).as_slice() else {
            panic!("{sample:?}");
        };
        let window = stage.scene.window_mut(id).unwrap();
        (window.width, window.height) = (width, height);
        let frame = stage.frame();
        let drawn = bounds(&frame, arrow).map(Vec::from);
        assert_eq!(drawn, Some(numbers(sample[1])), "{sample:?}");
        let places = [
            (4, height / 2 - 8),
            (width / 2 - 8, 4),
            (width - 12, height / 2 - 8),
            (width / 2 - 8, height - 12),
        ];
        for ((x, y), texel) in places.into_iter().zip(sample[2].split_whitespace()) {
            let texel: Vec<u32> = texel.split(',').map(|word| word.parse().unwrap()).collect();
            let expected = traced(texel[0], texel[1]);
            assert_eq!(frame.pixel(x as u32, y as u32), expected, "{sample:?}");
        }
    }
}

#[test]
fn arrows_take_no_opacity_and_need_contents() {
    let mut stage = Stage::sized(160, 96);
    let skin = skin(&mut stage);
    let id = window(&mut stage, Some(skin));
    let bitmap = stage.bitmaps.create(300, 200, &defaults()).unwrap();
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(bitmap);
    (window.ox, window.oy) = (20, 20);
    window.set_opacity(0);
    window.set_back_opacity(0);
    window.set_contents_opacity(0);
    let frame = stage.frame();
    assert_eq!(arrows(&frame), [true; 4]);
    assert_eq!(drawn(&frame), 4 * 16 * 8);
    stage.scene.window_mut(id).unwrap().contents = None;
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn the_cursor_shows_the_measured_texels() {
    let mut stage = Stage::sized(124, 78);
    let skin = traced_skin(&mut stage, |x, y| (128..160).contains(&x) && y >= 64);
    let id = sized(&mut stage, skin, 124, 78);
    for sample in samples("cursor") {
        let &[width, height] = numbers(sample[0]).as_slice() else {
            panic!("{sample:?}");
        };
        stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 0, width, height);
        let frame = stage.frame();
        let at = |x: i32, y: i32| frame.pixel(16 + x as u32, 16 + y as u32);
        let rows = [(0, 1), (height / 2, 2)];
        for (row, list) in rows {
            for (i, texel) in listed(sample[list], width) {
                let pixel = at(i, row);
                assert_eq!(
                    i32::from(pixel[0]) - 128,
                    texel,
                    "{sample:?} row {row} at {i}"
                );
                assert_eq!(pixel[2], 255);
            }
        }
        let columns = [(0, 3), (width / 2, 4)];
        for (column, list) in columns {
            for (j, texel) in listed(sample[list], height) {
                let pixel = at(column, j);
                assert_eq!(
                    i32::from(pixel[1]) - 64,
                    texel,
                    "{sample:?} column {column} at {j}"
                );
            }
        }
        let drawn = bounds(&frame, |pixel| pixel != BLACK);
        assert_eq!(drawn, Some([16, 16, 15 + width, 15 + height]), "{sample:?}");
    }
}

/// Renders each sample of `kind`, a window with the listed texels for a skin, and checks
/// the listed pixels; returns whether any of the texels had an alpha of 0.
fn bilinear_samples_show_as_measured(kind: &str) -> bool {
    let mut transparent = false;
    for sample in samples(kind) {
        let head = numbers(sample[0]);
        let (width, height) = (head[0], head[1]);
        let mut rgba = vec![0; 192 * 128 * 4];
        for texel in sample[1].split_whitespace() {
            let &[x, y, r, g, b, a] = numbers(&texel.replace(',', " ")).as_slice() else {
                panic!("{texel}");
            };
            let offset = (y as usize * 192 + x as usize) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[r as u8, g as u8, b as u8, a as u8]);
            transparent |= a == 0;
        }
        let mut stage = Stage::sized(width as u32, height as u32);
        let skin = stage.bitmap(192, 128, rgba);
        let id = sized(&mut stage, skin, width, height);
        if let &[_, _, x, y, cursor_width, cursor_height] = head.as_slice() {
            let rect = Rect::new(x, y, cursor_width, cursor_height);
            stage.scene.window_mut(id).unwrap().cursor_rect = rect;
        }
        let frame = stage.frame();
        for pixel in sample[2].split_whitespace() {
            let &[x, y, r, g, b] = numbers(&pixel.replace(',', " ")).as_slice() else {
                panic!("{pixel}");
            };
            let expected = [r as u8, g as u8, b as u8, 255];
            assert_eq!(
                frame.pixel(x as u32, y as u32),
                expected,
                "{head:?} {pixel}"
            );
        }
    }
    transparent
}

#[test]
fn the_background_is_interpolated_as_measured() {
    assert!(bilinear_samples_show_as_measured("bilinear-bg"));
}

#[test]
fn the_centre_of_the_cursor_is_interpolated_as_measured() {
    assert!(bilinear_samples_show_as_measured("bilinear-cursor"));
}

#[test]
fn a_pixel_between_two_texels_is_not_the_nearest_texel() {
    let mut stage = Stage::sized(480, 100);
    let mut rgba = vec![0; 192 * 128 * 4];
    for y in 0..128 {
        for (x, value) in [(63, 70), (64, 69)] {
            let offset = (y * 192 + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    let skin = stage.bitmap(192, 128, rgba);
    sized(&mut stage, skin, 480, 100);
    // Pixel 238 of 476 lies at texel 63 and 127 of 256, and 70 - 127 / 256 truncates
    // to 69.
    assert_eq!(stage.frame().pixel(2 + 238, 50), [69, 69, 69, 255]);
}

#[test]
fn a_transparent_texel_lends_no_colour() {
    // The row's colour is the other texel's, whichever of the two is transparent, and
    // its alpha is interpolated.
    let (opaque, clear): (Mixed, Mixed) = ((Some([200, 100, 50]), 255), (None, 0));
    assert_eq!(mix(opaque, clear, 64), (Some([200, 100, 50]), 191));
    assert_eq!(mix(clear, opaque, 64), (Some([200, 100, 50]), 63));
    assert_eq!(mix(clear, clear, 64), (None, 0));

    // On the screen: texel 63 transparent, texel 64 opaque, as in the example above.
    let mut stage = Stage::sized(480, 100);
    let mut rgba = vec![0; 192 * 128 * 4];
    for y in 0..128 {
        let offset = (y * 192 + 64) * 4;
        rgba[offset..offset + 4].copy_from_slice(&[200, 100, 50, 255]);
        // A colour that must not show.
        rgba[offset - 4..offset].copy_from_slice(&[9, 9, 9, 0]);
    }
    let skin = stage.bitmap(192, 128, rgba);
    sized(&mut stage, skin, 480, 100);
    // The alpha is 0 + ((255 * 127) >> 8) = 126, and the window blend over black gives
    // (colour * 127) >> 8.
    let expected = [200, 100, 50].map(|c: i32| ((c * 127) >> 8) as u8);
    let [r, g, b] = expected;
    assert_eq!(stage.frame().pixel(2 + 238, 50), [r, g, b, 255]);
}

/// The skin of the animation probes: a white cursor and four pause images.
fn animation_skin(stage: &mut Stage) -> BitmapId {
    let id = stage.bitmaps.create(192, 128, &defaults()).unwrap();
    let mut fill = |x, y, width, height, pixel| {
        stage
            .bitmaps
            .fill_rect(id, Rect::new(x, y, width, height), color(pixel))
            .unwrap();
    };
    fill(128, 64, 32, 32, WHITE);
    for (index, image) in SIGNS.into_iter().enumerate() {
        let index = index as i32;
        fill(160 + 16 * (index % 2), 64 + 16 * (index / 2), 16, 16, image);
    }
    id
}

const SIGNS: [Rgba; 4] = [
    [255, 60, 1, 255],
    [255, 120, 2, 255],
    [255, 180, 3, 255],
    [255, 240, 4, 255],
];

/// Runs the loop of the animation probes on a window of 64 by 64: on pass `t`, `script`
/// changes the window and says how often it is updated, then a frame is drawn, of which
/// the pixel at (`x`, `y`) is returned.
fn animate(
    frames: usize,
    (x, y): (u32, u32),
    mut script: impl FnMut(usize, &mut Window) -> usize,
) -> Vec<Rgba> {
    let mut stage = Stage::sized(64, 64);
    let skin = animation_skin(&mut stage);
    let id = sized(&mut stage, skin, 64, 64);
    (0..frames)
        .map(|t| {
            let window = stage.scene.window_mut(id).unwrap();
            for _ in 0..script(t, window) {
                window.update();
            }
            stage.frame().pixel(x, y)
        })
        .collect()
}

#[test]
fn the_cursor_blinks_as_measured() {
    let mut names = Vec::new();
    for sample in samples("blink") {
        let name = sample[0];
        names.push(name);
        let expected = numbers(sample[1]);
        let pixels = animate(expected.len(), (32, 32), |t, window| {
            if t == 0 {
                window.cursor_rect = Rect::new(0, 0, 32, 32);
            }
            let start = |window: &mut Window, active: bool, opacity: i32| {
                if t == 0 {
                    window.set_active(active);
                    window.set_contents_opacity(opacity);
                }
            };
            match name {
                "active-updated" => 1,
                "active-never-updated" => 0,
                "updated-twice-per-frame" => 2,
                "updated-on-even-t-only" => usize::from(t % 2 == 0),
                "inactive-updated" => {
                    start(window, false, 255);
                    1
                }
                "inactive-never-updated" => {
                    start(window, false, 255);
                    0
                }
                "contents_opacity=128" => {
                    start(window, true, 128);
                    1
                }
                "contents_opacity=100" => {
                    start(window, true, 100);
                    1
                }
                "inactive,contents_opacity=100" => {
                    start(window, false, 100);
                    1
                }
                "active;inactive-at-t=100;active-at-t=150" => {
                    match t {
                        100 => window.set_active(false),
                        150 => window.set_active(true),
                        _ => {}
                    }
                    1
                }
                "invisible-until-t=30" => {
                    window.visible = t >= 30;
                    1
                }
                "t=20:active=false;active=true-before-update" => {
                    if t == 20 {
                        window.set_active(false);
                        window.set_active(true);
                    }
                    1
                }
                "inactive-never-updated;t=20:active=true" => {
                    start(window, false, 255);
                    if t == 20 {
                        window.set_active(true);
                    }
                    0
                }
                "t=20:active=false;t=21:active=true" => {
                    match t {
                        20 => window.set_active(false),
                        21 => window.set_active(true),
                        _ => {}
                    }
                    1
                }
                other => panic!("no script for {other}"),
            }
        });
        // A white cursor over black: the red channel is the cursor's alpha.
        let alphas: Vec<i32> = pixels.iter().map(|pixel| i32::from(pixel[0])).collect();
        assert_eq!(alphas, expected, "{name}");
    }
    assert!(names.len() >= 10, "{names:?}");
}

#[test]
fn the_pause_sign_animates_as_measured() {
    let mut names = Vec::new();
    for sample in samples("pause") {
        let name = sample[0];
        names.push(name);
        let expected: Vec<Rgba> = sample[1]
            .split_whitespace()
            .map(|step| {
                let (alpha, image) = step.split_once('/').unwrap();
                let (alpha, image): (u8, usize) = (alpha.parse().unwrap(), image.parse().unwrap());
                match image {
                    0 => BLACK,
                    image => vkxp_pixel::blend_window(BLACK, SIGNS[image - 1], alpha),
                }
            })
            .collect();
        let pixels = animate(expected.len(), (24, 48), |t, window| {
            if t == 0 {
                window.set_pause(true);
            }
            match name {
                "updated-from-creation" => 1,
                "never-updated" => 0,
                "updated-twice-per-frame" => 2,
                "contents_opacity=128" => {
                    window.set_contents_opacity(128);
                    1
                }
                "opacity=100,back_opacity=100" => {
                    window.set_opacity(100);
                    window.set_back_opacity(100);
                    1
                }
                "t=20:pause=false;pause=true-before-update;t=50:false;t=51:true" => {
                    match t {
                        20 => {
                            window.set_pause(false);
                            window.set_pause(true);
                        }
                        50 => window.set_pause(false),
                        51 => window.set_pause(true),
                        _ => {}
                    }
                    1
                }
                "invisible-t=20..29" => {
                    window.visible = !(20..30).contains(&t);
                    1
                }
                other => panic!("no script for {other}"),
            }
        });
        assert_eq!(pixels, expected, "{name}");
    }
    assert!(names.len() >= 5, "{names:?}");
}

#[test]
fn windows_in_viewports_are_placed_and_cut_as_measured() {
    for sample in samples("viewport") {
        let mut stage = Stage::new();
        let skin = traced_skin(&mut stage, |_, _| true);
        let bitmap = traced_contents(&mut stage, 300, 200);
        let &[vx, vy, vw, vh, vox, voy] = numbers(sample[0]).as_slice() else {
            panic!("{sample:?}");
        };
        let &[x, y, width, height] = numbers(sample[1]).as_slice() else {
            panic!("{sample:?}");
        };
        let viewport = stage
            .scene
            .create_viewport(Rect::new(vx, vy, vw, vh))
            .unwrap();
        let view = stage.scene.viewport_mut(viewport).unwrap();
        (view.ox, view.oy) = (vox, voy);
        let id = stage.scene.create_window(Some(viewport)).unwrap();
        let window = stage.scene.window_mut(id).unwrap();
        window.windowskin = Some(skin);
        window.contents = Some(bitmap);
        (window.x, window.y, window.width, window.height) = (x, y, width, height);
        (window.ox, window.oy) = (20, 10);
        window.cursor_rect = Rect::new(5, 5, 60, 30);
        let frame = stage.frame();
        let drawn = bounds(&frame, |pixel| pixel != BLACK).unwrap();
        assert_eq!(Vec::from(drawn), numbers(sample[2]), "{sample:?}");
        let (kind, texel) = sample[3].split_once(' ').unwrap();
        let texel: Vec<u8> = texel.split(',').map(|word| word.parse().unwrap()).collect();
        let blue = if kind == "skin" { 255 } else { 100 };
        assert_eq!(
            frame.pixel(drawn[0] as u32, drawn[1] as u32),
            [texel[0], texel[1], blue, 255],
            "{sample:?}"
        );
    }
}

// The one pixel of an odd row that the window blend treats differently.

/// A skin whose every texel is half transparent, of which only the texels that `keep`
/// names are there.
fn half_skin(stage: &mut Stage, keep: impl Fn(u32, u32) -> bool) -> BitmapId {
    let mut rgba = Vec::with_capacity(192 * 128 * 4);
    for y in 0..128 {
        for x in 0..192 {
            let texel = if keep(x, y) { [0, 0, 0, 4] } else { CLEAR };
            rgba.extend_from_slice(&texel);
        }
    }
    stage.bitmap(192, 128, rgba)
}

/// A grey backdrop over which the two formulas differ for the texels of [`half_skin`]:
/// 158 by the window blend, 157 by the normal one.
fn backdrop(stage: &mut Stage) {
    let size = stage.scene.logical_size();
    let grey = stage.solid(size.width, size.height, [161, 161, 161, 255]);
    let sprite = stage.scene.create_sprite(None).unwrap();
    stage
        .scene
        .sprite_mut(sprite)
        .unwrap()
        .set_bitmap(Some(grey), &stage.bitmaps);
    stage.scene.set_sprite_z(sprite, -100).unwrap();
}

/// The columns of row `y` that were blended with the normal formula.
fn tails(frame: &Frame, y: u32) -> Vec<u32> {
    (0..frame.size.width)
        .filter(|&x| frame.pixel(x, y)[0] == 157)
        .collect()
}

#[test]
fn an_odd_background_row_has_its_exception_only_where_it_is_cut() {
    let mut stage = Stage::sized(64, 40);
    backdrop(&mut stage);
    let skin = half_skin(&mut stage, |x, _| x < 128);
    // Inner widths of 29 and 30.
    for (width, x, expected) in [
        (33, 10, vec![]),
        (34, 10, vec![]),
        // Cut by the right edge of the screen to 23, 22 and 29 pixels.
        (33, 39, vec![63]),
        (34, 40, vec![]),
        (33, 33, vec![63]),
        // Cut by the left edge to 22 and 21 pixels: the row ends inside the screen.
        (33, -9, vec![]),
        (34, -11, vec![]),
    ] {
        let id = sized(&mut stage, skin, width, 30);
        stage.scene.window_mut(id).unwrap().x = x;
        let frame = stage.frame();
        assert_eq!(tails(&frame, 10), expected, "{width} wide at {x}");
        let first = (x + 2).max(0) as u32;
        let last = (x + width - 3).min(63) as u32;
        let blended = bounds(&frame, |pixel| pixel[0] != 161).unwrap();
        assert_eq!(
            [blended[0] as u32, blended[2] as u32],
            [first, last],
            "{width} wide at {x}"
        );
        stage.scene.dispose_window(id, &mut stage.bitmaps);
    }
}

#[test]
fn an_odd_frame_row_counts_the_whole_width_of_the_window() {
    let mut stage = Stage::sized(64, 64);
    backdrop(&mut stage);
    let skin = half_skin(&mut stage, |x, y| x >= 128 && y < 64);
    for (width, x, expected) in [
        // The last pixel of the measured window: 33 wide at 10.
        (33, 10, vec![42]),
        (34, 10, vec![]),
        (34, 31, vec![63]),
        (34, -5, vec![28]),
        (33, -5, vec![]),
    ] {
        let id = sized(&mut stage, skin, width, 60);
        stage.scene.window_mut(id).unwrap().x = x;
        let frame = stage.frame();
        assert_eq!(tails(&frame, 3), expected, "{width} wide at {x}");
        // In the rows of the side edges the middle is transparent, and counts.
        assert_eq!(tails(&frame, 30), expected, "{width} wide at {x}");
        stage.scene.dispose_window(id, &mut stage.bitmaps);
    }
}

#[test]
fn odd_rows_of_the_upper_parts_count_what_the_window_leaves_of_them() {
    let mut stage = Stage::sized(64, 80);
    backdrop(&mut stage);
    let none = stage.bitmaps.create(192, 128, &defaults()).unwrap();
    let contents = stage.solid(21, 20, [0, 0, 0, 4]);
    let id = sized(&mut stage, none, 60, 60);
    let window = stage.scene.window_mut(id).unwrap();
    window.contents = Some(contents);
    // 21 columns from 16.
    assert_eq!(tails(&stage.frame(), 20), [36]);
    // Scrolled so that the contents area leaves 16 of them, then 15.
    stage.scene.window_mut(id).unwrap().ox = 5;
    assert_eq!(tails(&stage.frame(), 20), []);
    stage.scene.window_mut(id).unwrap().ox = 6;
    assert_eq!(tails(&stage.frame(), 20), [30]);

    // A cursor of 33 columns, whole and cut by the window to 28 and 27.
    let skin = half_skin(&mut stage, |x, y| (128..160).contains(&x) && y >= 64);
    let window = stage.scene.window_mut(id).unwrap();
    window.windowskin = Some(skin);
    window.contents = None;
    window.cursor_rect = Rect::new(0, 0, 33, 33);
    assert_eq!(tails(&stage.frame(), 30), [48]);
    stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(16, 0, 33, 33);
    assert_eq!(tails(&stage.frame(), 30), []);
    stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(17, 0, 33, 33);
    assert_eq!(tails(&stage.frame(), 30), [59]);
}
