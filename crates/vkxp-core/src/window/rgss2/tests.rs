use vkxp_compose_ref::{Compositor, Frame};
use vkxp_pixel::Rgba;

use super::*;
use crate::WindowId;
use crate::bitmap::testing::{color, defaults};
use crate::extract::testing::*;
use crate::test_support::window_cases::{cases, digest};

const BACK: Rgba = [0, 0, 160, 255];
const FRAME_PIECES: Rgba = [90, 10, 10, 255];
const ARROW_UP: Rgba = [1, 200, 200, 255];
const ARROW_DOWN: Rgba = [2, 200, 200, 255];
const ARROW_LEFT: Rgba = [3, 200, 200, 255];
const ARROW_RIGHT: Rgba = [4, 200, 200, 255];

/// The red of image `image` of the pause sign, as in the probe of the measurement.
fn pause_red(image: u8) -> u8 {
    60 * (image + 1)
}

fn fill(stage: &mut Stage, id: BitmapId, rect: (i32, i32, i32, i32), pixel: Rgba) {
    let (x, y, width, height) = rect;
    stage
        .bitmaps
        .fill_rect(id, Rect::new(x, y, width, height), color(pixel))
        .unwrap();
}

/// A window skin of 128 by 128 pixels with an opaque background, no pattern, a frame of one
/// colour, arrows with a colour each, a white cursor and the pause images of the probe.
fn skin(stage: &mut Stage) -> BitmapId {
    let id = stage.bitmaps.create(128, 128, &defaults()).unwrap();
    fill(stage, id, (0, 0, 64, 64), BACK);
    fill(stage, id, (64, 0, 64, 64), FRAME_PIECES);
    fill(stage, id, (80, 16, 32, 32), CLEAR);
    fill(stage, id, (88, 16, 16, 8), ARROW_UP);
    fill(stage, id, (88, 40, 16, 8), ARROW_DOWN);
    fill(stage, id, (80, 24, 8, 16), ARROW_LEFT);
    fill(stage, id, (104, 24, 8, 16), ARROW_RIGHT);
    fill(stage, id, (64, 64, 32, 32), WHITE);
    for image in 0..4 {
        let at = (96 + 16 * (image % 2), 64 + 16 * (image / 2), 16, 16);
        fill(stage, id, at, [pause_red(image as u8), 0, 0, 255]);
    }
    id
}

/// A skin that has nothing but the cursor and the pause sign, so that both stand over black.
fn bare_skin(stage: &mut Stage) -> BitmapId {
    let id = skin(stage);
    fill(stage, id, (0, 0, 128, 64), CLEAR);
    id
}

fn call(stage: &mut Stage, id: WindowId, call: WindowCall) {
    let window = stage.scene.window_mut(id).unwrap();
    window.call(&mut stage.bitmaps, call);
}

/// A window of RGSS2 of `width` by `height` at (`x`, `y`) with `skin`.
fn window(stage: &mut Stage, skin: BitmapId, x: i32, y: i32, width: i32, height: i32) -> WindowId {
    let id = stage.scene.create_window_rgss2(None).unwrap();
    for assignment in [
        WindowCall::Windowskin(Some(skin)),
        WindowCall::X(x),
        WindowCall::Y(y),
        WindowCall::Width(width),
        WindowCall::Height(height),
    ] {
        call(stage, id, assignment);
    }
    id
}

/// The number of pixels of `frame` that are `pixel`.
fn count(frame: &Frame, pixel: Rgba) -> usize {
    let pixels = frame.rgba.as_chunks::<4>().0;
    pixels.iter().filter(|found| **found == pixel).count()
}

// The window as one element.

#[test]
fn a_window_is_one_element_at_its_z() {
    let mut stage = Stage::sized(120, 120);
    let across = stage.solid(100, 10, RED);
    let down = stage.solid(10, 100, GREEN);
    for (bitmap, x, y, z) in [(across, 10, 30, 101), (down, 60, 10, 102)] {
        let id = stage.scene.create_sprite(None).unwrap();
        stage.scene.set_sprite_z(id, z).unwrap();
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
        (sprite.x, sprite.y) = (x, y);
    }
    let skin = skin(&mut stage);
    let id = window(&mut stage, skin, 10, 10, 100, 100);
    stage.scene.set_window_z(id, 100).unwrap();
    let contents = stage.solid(68, 68, WHITE);
    call(&mut stage, id, WindowCall::Contents(Some(contents)));

    let frame = stage.frame();
    // The contents, and the sprites over the frame, over the contents and over each other.
    assert_eq!(frame.pixel(40, 60), WHITE);
    assert_eq!(frame.pixel(12, 35), RED);
    assert_eq!(frame.pixel(50, 35), RED);
    assert_eq!(frame.pixel(65, 12), GREEN);
    assert_eq!(frame.pixel(65, 60), GREEN);
    assert_eq!(frame.pixel(65, 35), GREEN);
}

/// Scenario "Window, measured on the original" of "Draw order of RGSS2".
#[test]
fn the_y_of_a_sprite_orders_it_against_a_window_of_its_z() {
    for (y, over) in [(50, true), (-50, false)] {
        let mut stage = Stage::with_rules(crate::Rules::Rgss2, 64, 64);
        let red = stage.solid(20, 20, RED);
        let id = stage.scene.create_sprite(None).unwrap();
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.set_bitmap(Some(red), &stage.bitmaps);
        // Drawn at (20, 20) whatever its y.
        (sprite.x, sprite.y, sprite.oy) = (20, y, y - 20);
        let skin = skin(&mut stage);
        // The y of the window is no part of the order.
        let window = window(&mut stage, skin, 0, 10, 64, 54);
        assert_eq!(stage.scene.window(window).unwrap().z(), 0);
        // The background of the window is over the sprite, or the sprite as it is.
        assert_eq!(stage.frame().pixel(30, 30) == RED, over, "y {y}");
    }
}

#[test]
fn a_window_without_a_width_or_a_height_draws_nothing() {
    let mut stage = Stage::sized(64, 64);
    let skin = skin(&mut stage);
    let contents = stage.solid(40, 40, WHITE);
    for (width, height) in [(0, 50), (50, 0), (-5, 50), (50, -5)] {
        let id = window(&mut stage, skin, 4, 4, width, height);
        call(&mut stage, id, WindowCall::Contents(Some(contents)));
        call(&mut stage, id, WindowCall::Pause(true));
        call(&mut stage, id, WindowCall::Update);
    }
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn the_bitmaps_of_a_window_are_disposed_with_it() {
    let mut stage = Stage::sized(64, 64);
    let skin = skin(&mut stage);
    let before = stage.bitmaps.len();
    let id = window(&mut stage, skin, 0, 0, 48, 48);
    call(
        &mut stage,
        id,
        WindowCall::CursorRect(Rect::new(0, 0, 8, 8)),
    );
    assert_eq!(stage.bitmaps.len(), before + 3);
    stage.scene.dispose_window(id, &mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), before);
}

// Background and frame.

const CASES: &str = include_str!("../../../tests/data/window-rgss2-cases.txt");

/// Asserts that every frame of every scene of `text` is the picture its digest names.
fn assert_frames(text: &str) {
    let mut wrong = Vec::new();
    let mut frames = 0;
    for case in cases(text) {
        let mut compositor = Compositor::new();
        for (index, (packet, expected)) in case.frames().into_iter().enumerate() {
            let frame = compositor.render(&packet).unwrap();
            frames += 1;
            if digest(frame.size, &frame.rgba) != expected {
                wrong.push(format!("{}, frame {index}", case.name));
            }
        }
    }
    assert!(frames > 0, "no frame was compared");
    assert!(wrong.is_empty(), "frames differ:\n{}", wrong.join("\n"));
}

/// The cases of every requirement of the window of RGSS2: sizes even and odd, below and
/// above 32 and 64, opacities, skins with alpha; openness; contents and cursors; pause sign
/// and arrows; what is latched; viewports.
#[test]
fn the_cases_of_the_measurement_are_drawn_as_the_original_draws_them() {
    assert_frames(CASES);
}

/// Thirty scenes of the random generator of the measurement, from its held-out set.
#[test]
fn the_random_scenes_of_the_measurement_are_drawn_as_the_original_draws_them() {
    let text = include_str!("../../../tests/data/window-rgss2-random.txt");
    assert_eq!(cases(text).len(), 30);
    assert_frames(text);
}

#[test]
fn the_pattern_gives_the_background_its_colour_and_keeps_the_alpha() {
    let mut stage = Stage::sized(8, 8);
    let skin = stage.bitmaps.create(128, 128, &defaults()).unwrap();
    fill(&mut stage, skin, (0, 0, 64, 64), [10, 20, 30, 128]);
    fill(&mut stage, skin, (0, 64, 64, 64), [200, 100, 50, 255]);
    let id = window(&mut stage, skin, 0, 0, 68, 68);
    let background = stage.scene.window(id).unwrap().caches.background.bitmap;
    let background = stage.bitmaps.get(background.unwrap()).unwrap();
    for (x, y) in [(2, 2), (30, 40), (65, 65)] {
        assert_eq!(background.pixel(x, y), [200, 100, 50, 128], "({x}, {y})");
    }
    // The margin stays transparent.
    assert_eq!(background.pixel(1, 30)[3], 0);
    assert_eq!(background.pixel(66, 30)[3], 0);
}

// Openness.

const MEASURED: &str = include_str!("../../../tests/data/window-rgss2-samples.txt");

/// The samples of one kind: what follows the bar of each of its lines, and what stands
/// between the kind and the bar.
fn samples(kind: &str) -> Vec<(&'static str, &'static str)> {
    let rows: Vec<(&str, &str)> = MEASURED
        .lines()
        .filter(|line| line.split(' ').next() == Some(kind))
        .map(|line| {
            let (head, values) = line.split_once('|').unwrap();
            (head[kind.len()..].trim(), values.trim())
        })
        .collect();
    assert!(!rows.is_empty(), "no `{kind}` samples");
    rows
}

fn numbers(text: &str) -> Vec<i32> {
    text.split([' ', ','])
        .map(|word| word.parse().unwrap_or_else(|_| panic!("a number: {word}")))
        .collect()
}

#[test]
fn the_rows_of_an_opening_window_are_those_measured() {
    let mut compared = 0;
    for (height, values) in samples("open") {
        let height: i32 = height.parse().unwrap();
        for (openness, measured) in numbers(values).chunks(2).enumerate() {
            let (rows, top) = open_rows(height, openness as u8);
            assert_eq!(rows, measured[1], "rows of {height} at {openness}");
            if rows > 0 {
                assert_eq!(top, measured[0], "first row of {height} at {openness}");
            }
            compared += 1;
        }
    }
    assert_eq!(compared, 16 * 255);
}

#[test]
fn a_half_open_window_shows_sampled_rows_and_no_contents() {
    let mut stage = Stage::sized(120, 120);
    let skin = skin(&mut stage);
    let id = window(&mut stage, skin, 10, 10, 100, 100);
    let contents = stage.solid(68, 68, WHITE);
    call(&mut stage, id, WindowCall::Contents(Some(contents)));
    call(
        &mut stage,
        id,
        WindowCall::CursorRect(Rect::new(0, 0, 20, 20)),
    );
    let open = stage.frame();
    assert!(count(&open, WHITE) > 0);

    call(&mut stage, id, WindowCall::Openness(128));
    let half = stage.frame();
    assert_eq!(count(&half, WHITE), 0);
    for y in 0..120 {
        for x in 0..120 {
            let shown = (34..84).contains(&y) && (10..110).contains(&x);
            assert_eq!(half.pixel(x, y) != BLACK, shown, "({x}, {y})");
        }
    }
    // 50 rows of 100: every second row of the window, here where the frame is.
    for row in 0..50 {
        for x in 10..26 {
            let (found, wanted) = (half.pixel(x, 34 + row), open.pixel(x, 10 + 2 * row));
            assert_eq!(found, wanted, "row {row}, column {x}");
        }
    }
}

// Contents and cursor.

/// A window of 64 by 64 over black with a cursor, of which (20, 20) is a pixel.
fn with_cursor(stage: &mut Stage) -> WindowId {
    let skin = bare_skin(stage);
    let id = window(stage, skin, 0, 0, 64, 64);
    call(stage, id, WindowCall::CursorRect(Rect::new(0, 0, 16, 16)));
    id
}

#[test]
fn the_cursor_of_an_active_window_blinks_as_measured() {
    let mut stage = Stage::sized(64, 64);
    let id = with_cursor(&mut stage);
    assert_eq!(stage.frame().pixel(20, 20), WHITE);
    let measured = numbers(samples("blink")[0].1);
    assert_eq!(measured.len(), 80);
    assert_eq!(measured[..40], measured[40..]);
    for (update, opacity) in measured.into_iter().enumerate() {
        call(&mut stage, id, WindowCall::Update);
        let window = stage.scene.window(id).unwrap();
        assert_eq!(
            i32::from(window.cursor_opacity_rgss2()),
            opacity,
            "{update}"
        );
        // White over black at an opacity is that opacity.
        let level = opacity as u8;
        assert_eq!(
            stage.frame().pixel(20, 20),
            [level, level, level, 255],
            "{update}"
        );
    }
}

#[test]
fn the_cursor_of_a_window_made_inactive_stands_at_127() {
    let mut stage = Stage::sized(64, 64);
    let id = with_cursor(&mut stage);
    for _ in 0..3 {
        call(&mut stage, id, WindowCall::Update);
    }
    let measured = numbers(samples("inactive")[0].1);
    assert!(measured.iter().all(|&opacity| opacity == 127));
    call(&mut stage, id, WindowCall::Active(false));
    for opacity in measured {
        let window = stage.scene.window(id).unwrap();
        assert_eq!(i32::from(window.cursor_opacity_rgss2()), opacity);
        assert_eq!(stage.frame().pixel(20, 20), [127, 127, 127, 255]);
        call(&mut stage, id, WindowCall::Update);
    }
}

// Pause sign and arrows.

#[test]
fn the_pause_sign_fades_in_and_changes_its_image_as_measured() {
    let mut stage = Stage::sized(64, 64);
    let skin = bare_skin(&mut stage);
    let id = window(&mut stage, skin, 0, 0, 64, 64);
    call(&mut stage, id, WindowCall::Pause(true));
    assert!(all(&stage.frame(), BLACK));
    let measured = numbers(samples("pause")[0].1);
    assert_eq!(measured.len(), 70);
    for (update, red) in measured.into_iter().enumerate() {
        call(&mut stage, id, WindowCall::Update);
        let steps = update as u32 + 1;
        let window = stage.scene.window(id).unwrap();
        let (alpha, image) = window.pause_sign_rgss2().unwrap();
        assert_eq!(
            (u32::from(alpha), image),
            ((32 * steps).min(255), steps / 16 % 4)
        );
        // The sign is at (24, 48); what is measured is its red over black.
        let wanted = (u32::from(pause_red(image as u8)) * (u32::from(alpha) + 1)) >> 8;
        assert_eq!(wanted, red as u32, "{update}");
        assert_eq!(
            stage.frame().pixel(28, 52),
            [red as u8, 0, 0, 255],
            "{update}"
        );
    }
}

#[test]
fn arrows_are_drawn_where_the_contents_go_on() {
    let mut stage = Stage::sized(100, 100);
    let skin = skin(&mut stage);
    let id = window(&mut stage, skin, 0, 0, 100, 100);
    let contents = stage.bitmaps.create(200, 200, &defaults()).unwrap();
    call(&mut stage, id, WindowCall::Contents(Some(contents)));
    call(&mut stage, id, WindowCall::Ox(10));
    call(&mut stage, id, WindowCall::Oy(0));
    let frame = stage.frame();
    assert_eq!(frame.pixel(45, 90), ARROW_DOWN);
    assert_eq!(frame.pixel(6, 45), ARROW_LEFT);
    assert_eq!(frame.pixel(90, 45), ARROW_RIGHT);
    assert_eq!(count(&frame, ARROW_UP), 0);
    assert_eq!(frame.pixel(45, 6), FRAME_PIECES);
}

// Latched state.

#[test]
fn contents_given_to_a_hidden_window_show_after_the_next_update() {
    let mut stage = Stage::sized(64, 64);
    let skin = skin(&mut stage);
    let id = window(&mut stage, skin, 0, 0, 64, 64);
    stage.scene.window_mut(id).unwrap().visible = false;
    let contents = stage.solid(32, 32, WHITE);
    call(&mut stage, id, WindowCall::Contents(Some(contents)));
    stage.scene.window_mut(id).unwrap().visible = true;

    let frame = stage.frame();
    assert_eq!(count(&frame, WHITE), 0);
    assert_eq!(frame.pixel(30, 30), BACK);
    assert_eq!(frame.pixel(3, 3), FRAME_PIECES);

    call(&mut stage, id, WindowCall::Update);
    assert_eq!(stage.frame().pixel(30, 30), WHITE);
}

/// A window of 64 by 64 whose pause sign is opaque, and its skin, which was made green all
/// over after it was assigned.
fn with_repainted_skin(stage: &mut Stage) -> (WindowId, BitmapId) {
    let skin = skin(stage);
    let id = window(stage, skin, 0, 0, 64, 64);
    call(stage, id, WindowCall::Pause(true));
    for _ in 0..8 {
        call(stage, id, WindowCall::Update);
    }
    fill(stage, skin, (0, 0, 128, 128), GREEN);
    (id, skin)
}

#[test]
fn later_pixels_of_the_skin_reach_the_pause_sign_alone() {
    let mut stage = Stage::sized(64, 64);
    with_repainted_skin(&mut stage);
    let frame = stage.frame();
    assert_eq!(frame.pixel(3, 3), FRAME_PIECES);
    assert_eq!(frame.pixel(30, 30), BACK);
    assert_eq!(frame.pixel(28, 52), GREEN);
}

#[test]
fn assigning_the_width_it_has_builds_background_and_frame_anew() {
    let mut stage = Stage::sized(64, 64);
    let (id, skin) = with_repainted_skin(&mut stage);
    // The same skin again builds nothing.
    call(&mut stage, id, WindowCall::Windowskin(Some(skin)));
    assert_eq!(stage.frame().pixel(30, 30), BACK);
    call(&mut stage, id, WindowCall::Width(64));
    let frame = stage.frame();
    assert_eq!(frame.pixel(3, 3), GREEN);
    assert_eq!(frame.pixel(30, 30), GREEN);
}

// A window of RGSS1 through the calls.

#[test]
fn a_window_of_rgss1_takes_the_calls_as_plain_assignments() {
    let mut stage = Stage::sized(64, 64);
    let id = stage.scene.create_window(None).unwrap();
    call(&mut stage, id, WindowCall::Width(40));
    call(&mut stage, id, WindowCall::Openness(100));
    call(&mut stage, id, WindowCall::Active(false));
    call(&mut stage, id, WindowCall::Update);
    let window = stage.scene.window(id).unwrap();
    assert!(!window.is_rgss2());
    assert_eq!((window.width, window.openness()), (40, 255));
    // The blink of RGSS1 stands at its twelfth step.
    assert_eq!(window.cursor_opacity(), 255 - 8 * 12);
}
