use vkxp_compose_ref::Frame;
use vkxp_pixel::Rgba;

use super::*;
use crate::bitmap::testing::{color, defaults};
use crate::extract::testing::*;
use crate::{Tone, WindowId};

const BACK: Rgba = [0, 0, 200, 255];
const FRAME_PIECES: Rgba = [90, 10, 10, 255];
const ARROW_UP: Rgba = [1, 200, 200, 255];
const ARROW_DOWN: Rgba = [2, 200, 200, 255];
const ARROW_LEFT: Rgba = [3, 200, 200, 255];
const ARROW_RIGHT: Rgba = [4, 200, 200, 255];
/// The first image of the pause sign.
const PAUSE_FIRST: Rgba = [60, 0, 0, 255];
/// Flat opaque contents.
const INK: Rgba = [10, 250, 10, 255];

fn fill(stage: &mut Stage, id: BitmapId, rect: (i32, i32, i32, i32), pixel: Rgba) {
    let (x, y, width, height) = rect;
    stage
        .bitmaps
        .fill_rect(id, Rect::new(x, y, width, height), color(pixel))
        .unwrap();
}

/// The piece skin of the design: 128 by 128, one opaque colour for each piece and
/// transparent elsewhere.
fn piece_skin(stage: &mut Stage) -> BitmapId {
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
        fill(stage, id, at, [60 * (image as u8 + 1), 0, 0, 255]);
    }
    id
}

/// The readable skin of the design: pixel (x, y) is (x, y, 100, 255), the pattern quadrant
/// transparent.
fn readable_skin(stage: &mut Stage) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..128u8 {
        for x in 0..128u8 {
            let pattern = x < 64 && y >= 64;
            rgba.extend_from_slice(&if pattern { CLEAR } else { [x, y, 100, 255] });
        }
    }
    stage.bitmap(128, 128, rgba)
}

fn call(stage: &mut Stage, id: WindowId, call: WindowCall) {
    let window = stage.scene.window_mut(id).unwrap();
    window.call(&mut stage.bitmaps, call);
}

fn calls(stage: &mut Stage, id: WindowId, list: &[WindowCall]) {
    for assignment in list {
        call(stage, id, *assignment);
    }
}

fn tone(stage: &mut Stage, id: WindowId, tone: (f64, f64, f64, f64)) {
    let window = stage.scene.window_mut(id).unwrap();
    window.set_tone(Tone::new(tone.0, tone.1, tone.2, tone.3));
}

/// A window of RGSS3 of `width` by `height` at (`x`, `y`) with `skin` and transparent
/// contents of 1 by 1.
fn window(
    stage: &mut Stage,
    skin: Option<BitmapId>,
    (x, y, width, height): (i32, i32, i32, i32),
) -> WindowId {
    let id = stage.scene.create_window_rgss3(None).unwrap();
    let contents = stage.bitmaps.create(1, 1, &defaults()).unwrap();
    calls(
        stage,
        id,
        &[
            WindowCall::Contents(Some(contents)),
            WindowCall::Windowskin(skin),
            WindowCall::Move(x, y, width, height),
        ],
    );
    id
}

/// Gives the window flat opaque contents of `side` by `side`.
fn contents(stage: &mut Stage, id: WindowId, side: u32) -> BitmapId {
    let bitmap = stage.solid(side, side, INK);
    call(stage, id, WindowCall::Contents(Some(bitmap)));
    bitmap
}

fn sprite(stage: &mut Stage, bitmap: BitmapId, z: i32) {
    let id = stage.scene.create_sprite(None).unwrap();
    stage.scene.set_sprite_z(id, z).unwrap();
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> WindowCall {
    WindowCall::CursorRect(Rect::new(x, y, width, height))
}

/// The number of pixels of `frame` that are `pixel`.
fn count(frame: &Frame, pixel: Rgba) -> usize {
    let pixels = frame.rgba.as_chunks::<4>().0;
    pixels.iter().filter(|found| **found == pixel).count()
}

/// Asserts that the pixels of `frame` that are `pixel` are those of the rectangle
/// (`left` .. `right`, `top` .. `bottom`).
#[track_caller]
fn assert_fills(frame: &Frame, pixel: Rgba, (left, right, top, bottom): (u32, u32, u32, u32)) {
    assert_eq!(
        count(frame, pixel),
        ((right - left) * (bottom - top)) as usize
    );
    for y in top..bottom {
        for x in left..right {
            assert_eq!(frame.pixel(x, y), pixel, "({x}, {y})");
        }
    }
}

// Requirement "Window of RGSS3".

#[test]
fn a_new_window_has_the_defaults_of_rgss3() {
    let mut stage = Stage::rgss2(8, 8);
    let id = stage.scene.create_window_rgss3(None).unwrap();
    let window = stage.scene.window(id).unwrap();
    assert!(window.is_rgss3());
    assert_eq!(window.z(), 100);
    assert_eq!(window.back_opacity(), 192);
    assert_eq!((window.padding(), window.padding_bottom()), (8, 8));
    assert!(window.arrows_visible());
    assert_eq!(window.tone(), Tone::new(0.0, 0.0, 0.0, 0.0));
    assert_eq!(window.openness(), 255);
}

#[test]
fn the_three_kinds_of_window_are_told_apart() {
    let mut stage = Stage::rgss2(8, 8);
    let first = stage.scene.create_window(None).unwrap();
    let second = stage.scene.create_window_rgss2(None).unwrap();
    let third = stage.scene.create_window_rgss3(None).unwrap();
    let kind = |id| {
        let window = stage.scene.window(id).unwrap();
        (window.is_rgss2(), window.is_rgss3())
    };
    assert_eq!(kind(first), (false, false));
    assert_eq!(kind(second), (true, false));
    assert_eq!(kind(third), (true, true));
}

#[test]
fn a_window_stands_among_sprites_at_its_default_z() {
    let mut stage = Stage::rgss2(120, 120);
    let below = stage.solid(100, 100, GREEN);
    let above = stage.solid(32, 32, RED);
    sprite(&mut stage, below, 99);
    sprite(&mut stage, above, 101);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    stage.scene.window_mut(id).unwrap().set_back_opacity(255);
    let frame = stage.frame();
    assert_eq!(count(&frame, GREEN), 0);
    assert_eq!(count(&frame, RED), 1024);
}

#[test]
fn padding_sets_both_paddings_and_neither_is_clamped() {
    let mut stage = Stage::rgss2(8, 8);
    let id = stage.scene.create_window_rgss3(None).unwrap();
    let mut read = |assignment| {
        call(&mut stage, id, assignment);
        let window = stage.scene.window(id).unwrap();
        (window.padding(), window.padding_bottom())
    };
    assert_eq!(read(WindowCall::Padding(12)), (12, 12));
    assert_eq!(read(WindowCall::PaddingBottom(20)), (12, 20));
    assert_eq!(read(WindowCall::Padding(-5)), (-5, -5));
    assert_eq!(read(WindowCall::PaddingBottom(100_000)), (-5, 100_000));
}

#[test]
fn windows_of_rgss1_and_rgss2_ignore_the_new_calls() {
    for rgss2 in [false, true] {
        let mut stage = Stage::rgss2(120, 120);
        let skin = stage.bitmap(192, 128, crate::bitmap::testing::pattern(192, 128, 7));
        let ink = stage.solid(200, 200, INK);
        let id = if rgss2 {
            stage.scene.create_window_rgss2(None).unwrap()
        } else {
            stage.scene.create_window(None).unwrap()
        };
        calls(
            &mut stage,
            id,
            &[
                WindowCall::Windowskin(Some(skin)),
                WindowCall::Contents(Some(ink)),
                WindowCall::Move(2, 3, 100, 90),
                WindowCall::Ox(10),
                rect(0, 0, 32, 32),
            ],
        );
        let before = stage.frame();
        calls(
            &mut stage,
            id,
            &[
                WindowCall::Padding(3),
                WindowCall::PaddingBottom(40),
                WindowCall::ArrowsVisible(false),
            ],
        );
        let window = stage.scene.window_mut(id).unwrap();
        window.set_tone(Tone::new(50.0, 0.0, 0.0, 0.0));
        assert_eq!((window.padding(), window.padding_bottom()), (16, 16));
        assert!(window.arrows_visible());
        assert_eq!(window.tone(), Tone::default());
        assert_eq!(
            (window.x, window.y, window.width, window.height),
            (2, 3, 100, 90)
        );
        assert!(count(&before, INK) > 0);
        assert_eq!(stage.frame().rgba, before.rgba, "rgss2: {rgss2}");
    }
}

// Requirement "Background of a window of RGSS3".

/// A window of 68 by 68 with the readable skin over a flat sprite of (200, 200, 200).
fn over_grey(stage: &mut Stage) -> WindowId {
    let grey = stage.solid(120, 120, [200, 200, 200, 255]);
    sprite(stage, grey, 0);
    let skin = readable_skin(stage);
    window(stage, Some(skin), (0, 0, 68, 68))
}

#[test]
fn the_background_is_blended_normally_at_openness_255() {
    let mut stage = Stage::rgss2(120, 120);
    over_grey(&mut stage);
    assert_eq!(stage.frame().pixel(30, 40), [70, 77, 124, 255]);
}

#[test]
fn the_tone_changes_the_background_with_its_components_truncated() {
    let mut stage = Stage::rgss2(120, 120);
    let id = over_grey(&mut stage);
    for value in [(40.0, -30.0, 0.0, 128.0), (40.9, -30.9, 0.9, 128.9)] {
        tone(&mut stage, id, value);
        assert_eq!(stage.frame().pixel(30, 40), [105, 56, 101, 255]);
    }
}

#[test]
fn the_tone_greys_and_clamps() {
    let mut stage = Stage::rgss2(120, 120);
    let id = over_grey(&mut stage);
    stage.scene.window_mut(id).unwrap().set_back_opacity(255);
    tone(&mut stage, id, (0.0, 0.0, 0.0, 255.0));
    assert_eq!(stage.frame().pixel(30, 40), [42, 42, 42, 255]);
    tone(&mut stage, id, (-255.0, 255.0, 20.0, 0.0));
    assert_eq!(stage.frame().pixel(30, 40), [0, 255, 120, 255]);
}

#[test]
fn only_the_background_is_toned() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    stage.scene.window_mut(id).unwrap().set_back_opacity(255);
    tone(&mut stage, id, (-255.0, -255.0, -255.0, 0.0));
    call(&mut stage, id, rect(20, 20, 16, 16));
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), FRAME_PIECES);
    assert_eq!(frame.pixel(60, 60), BLACK);
    assert_eq!(count(&frame, WHITE), 256);
}

#[test]
fn an_opening_window_shows_the_toned_background_in_its_rows() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 68, 100));
    tone(&mut stage, id, (40.0, -30.0, 0.0, 128.0));
    assert_eq!(stage.frame().pixel(30, 50), [38, 0, 82, 255]);
    call(&mut stage, id, WindowCall::Openness(128));
    let frame = stage.frame();
    assert_eq!(frame.pixel(30, 50), [38, 0, 82, 255]);
    assert_eq!(frame.pixel(30, 23), BLACK);
    assert_eq!(frame.pixel(30, 74), BLACK);
    assert_eq!(frame.pixel(0, 24), FRAME_PIECES);
    assert_eq!(frame.pixel(0, 73), FRAME_PIECES);
}

// Requirement "Cursor of a window of RGSS3".

#[test]
fn the_cursor_has_a_border_of_4() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = readable_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    call(&mut stage, id, rect(10, 6, 16, 16));
    let frame = stage.frame();
    for ((x, y), (skin_x, skin_y)) in [
        ((18, 14), (64, 64)),
        ((21, 14), (67, 64)),
        ((22, 14), (68, 64)),
        ((22, 18), (68, 68)),
        ((23, 18), (71, 68)),
        ((33, 29), (95, 95)),
    ] {
        assert_eq!(frame.pixel(x, y), [skin_x, skin_y, 100, 255], "({x}, {y})");
    }
}

#[test]
fn the_centre_of_a_large_cursor_is_stretched_bilinearly() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = readable_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    call(&mut stage, id, rect(10, 6, 40, 40));
    assert_eq!(stage.frame().pixel(40, 30), [80, 76, 100, 255]);
}

#[test]
fn the_cursor_lies_at_the_absolute_difference_from_the_origin() {
    let mut stage = Stage::rgss2(320, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 300, 100));
    calls(&mut stage, id, &[rect(10, 0, 16, 16), WindowCall::Ox(30)]);
    assert_fills(&stage.frame(), WHITE, (28, 44, 8, 24));
    calls(&mut stage, id, &[rect(-250, 0, 16, 16), WindowCall::Ox(0)]);
    assert_fills(&stage.frame(), WHITE, (258, 274, 8, 24));
    calls(&mut stage, id, &[rect(10, 20, 16, 16), WindowCall::Oy(50)]);
    assert_fills(&stage.frame(), WHITE, (18, 34, 38, 54));
}

// Requirement "Cursor placement of a window of RGSS3".

/// A window of `side` by `side` with the piece skin and a cursor of 100 by 100.
fn with_large_cursor(stage: &mut Stage, side: i32) -> (WindowId, BitmapId) {
    let skin = piece_skin(stage);
    let id = window(stage, Some(skin), (0, 0, side, side));
    call(stage, id, rect(0, 0, 100, 100));
    (id, skin)
}

#[test]
fn the_placed_cursor_is_kept_while_the_window_grows() {
    let mut stage = Stage::rgss2(160, 160);
    let (id, skin) = with_large_cursor(&mut stage, 40);
    assert_fills(&stage.frame(), WHITE, (8, 32, 8, 32));
    calls(
        &mut stage,
        id,
        &[
            WindowCall::Width(120),
            WindowCall::Height(120),
            WindowCall::X(0),
            WindowCall::Y(3),
            WindowCall::Y(0),
        ],
    );
    stage.scene.set_window_z(id, 5).unwrap();
    stage.scene.window_mut(id).unwrap().set_opacity(255);
    calls(
        &mut stage,
        id,
        &[
            WindowCall::ContentsOpacity(255),
            WindowCall::Openness(255),
            WindowCall::ArrowsVisible(true),
        ],
    );
    tone(&mut stage, id, (10.0, 20.0, 30.0, 40.0));
    call(&mut stage, id, WindowCall::Windowskin(Some(skin)));
    assert_fills(&stage.frame(), WHITE, (8, 32, 8, 32));
}

#[test]
fn move_does_not_place_the_cursor() {
    let mut stage = Stage::rgss2(160, 160);
    let (id, _) = with_large_cursor(&mut stage, 40);
    call(&mut stage, id, WindowCall::Move(0, 0, 120, 120));
    assert_fills(&stage.frame(), WHITE, (8, 32, 8, 32));
}

#[test]
fn the_calls_of_its_list_place_the_cursor() {
    let placing: [&[WindowCall]; 6] = [
        &[WindowCall::Ox(0)],
        &[WindowCall::Oy(0)],
        &[WindowCall::Padding(8)],
        &[WindowCall::PaddingBottom(8)],
        &[rect(0, 0, 100, 100)],
        &[WindowCall::Windowskin(None)],
    ];
    for list in placing {
        let mut stage = Stage::rgss2(160, 160);
        let (id, skin) = with_large_cursor(&mut stage, 40);
        call(&mut stage, id, WindowCall::Move(0, 0, 120, 120));
        calls(&mut stage, id, list);
        // The skin again, where it was taken away.
        call(&mut stage, id, WindowCall::Windowskin(Some(skin)));
        assert_fills(&stage.frame(), WHITE, (8, 108, 8, 108));
    }
}

#[test]
fn the_kept_part_is_cut_by_the_window_that_shrinks() {
    let mut stage = Stage::rgss2(160, 160);
    let (id, _) = with_large_cursor(&mut stage, 120);
    assert_fills(&stage.frame(), WHITE, (8, 108, 8, 108));
    calls(
        &mut stage,
        id,
        &[WindowCall::Width(40), WindowCall::Height(40)],
    );
    assert_fills(&stage.frame(), WHITE, (8, 40, 8, 40));
    call(&mut stage, id, rect(0, 0, 100, 100));
    assert_fills(&stage.frame(), WHITE, (8, 32, 8, 32));
}

#[test]
fn the_placed_cursor_moves_with_the_window() {
    let mut stage = Stage::rgss2(200, 160);
    let (id, _) = with_large_cursor(&mut stage, 120);
    calls(&mut stage, id, &[WindowCall::X(50), WindowCall::Y(20)]);
    assert_fills(&stage.frame(), WHITE, (58, 158, 28, 128));
}

// Requirement "Contents of a window of RGSS3".

#[test]
fn the_paddings_clip_the_contents() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 60, 50));
    contents(&mut stage, id, 100);
    assert_fills(&stage.frame(), INK, (8, 52, 8, 42));
    call(&mut stage, id, WindowCall::PaddingBottom(20));
    assert_fills(&stage.frame(), INK, (8, 52, 8, 30));
    call(&mut stage, id, WindowCall::Padding(12));
    assert_fills(&stage.frame(), INK, (12, 48, 12, 38));
    call(&mut stage, id, WindowCall::Padding(30));
    assert_eq!(count(&stage.frame(), INK), 0);
    call(&mut stage, id, WindowCall::Padding(-3));
    assert_fills(&stage.frame(), INK, (0, 60, 0, 50));
}

#[test]
fn the_contents_are_drawn_without_a_skin() {
    let mut stage = Stage::rgss2(120, 120);
    let id = window(&mut stage, None, (0, 0, 60, 50));
    contents(&mut stage, id, 100);
    calls(&mut stage, id, &[WindowCall::Ox(5), WindowCall::Oy(7)]);
    assert_fills(&stage.frame(), INK, (8, 52, 8, 42));
}

#[test]
fn the_paddings_show_the_contents_again() {
    let showing = [
        (None, false),
        (Some(WindowCall::ArrowsVisible(true)), false),
        (Some(WindowCall::Padding(8)), true),
        (Some(WindowCall::PaddingBottom(8)), true),
        (Some(WindowCall::Update), true),
        (Some(WindowCall::Move(0, 0, 100, 100)), true),
    ];
    for (assignment, shows) in showing {
        let mut stage = Stage::rgss2(120, 120);
        let id = window(&mut stage, None, (0, 0, 100, 100));
        stage.scene.window_mut(id).unwrap().visible = false;
        contents(&mut stage, id, 100);
        stage.scene.window_mut(id).unwrap().visible = true;
        assert_eq!(count(&stage.frame(), INK), 0);
        match assignment {
            Some(assignment) => call(&mut stage, id, assignment),
            None => tone(&mut stage, id, (10.0, 0.0, 0.0, 0.0)),
        }
        let expected = if shows { 7056 } else { 0 };
        assert_eq!(count(&stage.frame(), INK), expected, "{assignment:?}");
    }
}

// Requirement "Cached parts and move of a window of RGSS3".

/// A window of 100 by 100 at (5, 6) whose piece skin got other colours after it was
/// assigned.
fn repainted(stage: &mut Stage) -> WindowId {
    let skin = piece_skin(stage);
    let id = window(stage, Some(skin), (5, 6, 100, 100));
    stage.scene.window_mut(id).unwrap().set_back_opacity(255);
    fill(stage, skin, (0, 0, 64, 64), GREEN);
    fill(stage, skin, (64, 0, 64, 64), RED);
    id
}

#[test]
fn move_builds_background_and_frame() {
    let mut stage = Stage::rgss2(120, 120);
    let id = repainted(&mut stage);
    call(&mut stage, id, WindowCall::Move(5, 6, 100, 100));
    let frame = stage.frame();
    assert_eq!(frame.pixel(5, 6), RED);
    assert_eq!(frame.pixel(55, 56), GREEN);
}

#[test]
fn the_new_properties_build_nothing() {
    let mut stage = Stage::rgss2(120, 120);
    let id = repainted(&mut stage);
    tone(&mut stage, id, (0.0, 0.0, 0.0, 0.0));
    calls(
        &mut stage,
        id,
        &[
            WindowCall::Padding(10),
            WindowCall::PaddingBottom(12),
            WindowCall::ArrowsVisible(false),
        ],
    );
    let frame = stage.frame();
    assert_eq!(frame.pixel(5, 6), FRAME_PIECES);
    assert_eq!(frame.pixel(55, 56), BACK);
}

#[test]
fn move_reads_back() {
    let mut stage = Stage::rgss2(8, 8);
    let id = stage.scene.create_window_rgss3(None).unwrap();
    call(&mut stage, id, WindowCall::Move(5, 6, 100, 80));
    let window = stage.scene.window(id).unwrap();
    assert_eq!(
        (window.x, window.y, window.width, window.height),
        (5, 6, 100, 80)
    );
}

// Requirement "Scroll arrows of a window of RGSS3".

/// The number of pixels of the arrows down, left, right and up.
fn arrows(frame: &Frame) -> [usize; 4] {
    [ARROW_DOWN, ARROW_LEFT, ARROW_RIGHT, ARROW_UP].map(|pixel| count(frame, pixel))
}

/// A window of 100 by 100 with the piece skin and contents of `side` by `side` that are
/// not seen.
fn scrolling(stage: &mut Stage, side: u32) -> WindowId {
    let skin = piece_skin(stage);
    let id = window(stage, Some(skin), (0, 0, 100, 100));
    call(stage, id, WindowCall::ContentsOpacity(0));
    contents(stage, id, side);
    id
}

#[test]
fn the_arrows_stand_at_their_places() {
    let mut stage = Stage::rgss2(120, 120);
    let id = scrolling(&mut stage, 200);
    call(&mut stage, id, WindowCall::Ox(10));
    let frame = stage.frame();
    assert_fills(&frame, ARROW_DOWN, (42, 58, 88, 96));
    assert_fills(&frame, ARROW_LEFT, (4, 12, 42, 58));
    assert_fills(&frame, ARROW_RIGHT, (88, 96, 42, 58));
    assert_eq!(count(&frame, ARROW_UP), 0);
    call(&mut stage, id, WindowCall::Oy(5));
    assert_fills(&stage.frame(), ARROW_UP, (42, 58, 4, 12));
}

/// The window of [`scrolling`] with contents of 80 by 80 after `padding = 16` and the calls
/// that do not decide the arrows.
fn padded(stage: &mut Stage) -> WindowId {
    let id = scrolling(stage, 80);
    assert_eq!(arrows(&stage.frame()), [0; 4]);
    calls(
        stage,
        id,
        &[
            WindowCall::Padding(16),
            WindowCall::Update,
            WindowCall::X(5),
            WindowCall::Active(false),
            WindowCall::Pause(true),
            rect(1, 2, 3, 4),
            WindowCall::ContentsOpacity(0),
            WindowCall::Openness(255),
        ],
    );
    assert_eq!(arrows(&stage.frame()), [0; 4]);
    id
}

#[test]
fn the_paddings_count_but_do_not_decide_the_arrows() {
    for deciding in [
        WindowCall::Ox(0),
        WindowCall::Width(100),
        WindowCall::Move(0, 0, 100, 100),
    ] {
        let mut stage = Stage::rgss2(120, 120);
        let id = padded(&mut stage);
        call(&mut stage, id, deciding);
        assert_eq!(arrows(&stage.frame()), [128, 0, 128, 0], "{deciding:?}");
    }
}

#[test]
fn the_bottom_padding_counts_for_the_down_arrow() {
    let mut stage = Stage::rgss2(120, 120);
    let id = scrolling(&mut stage, 80);
    call(&mut stage, id, WindowCall::PaddingBottom(40));
    assert_eq!(arrows(&stage.frame()), [0; 4]);
    call(&mut stage, id, WindowCall::Height(100));
    assert_eq!(arrows(&stage.frame()), [128, 0, 0, 0]);
}

#[test]
fn arrows_visible_takes_the_arrows_away_and_brings_them_back() {
    let mut stage = Stage::rgss2(120, 120);
    let id = padded(&mut stage);
    call(&mut stage, id, WindowCall::Ox(0));
    call(&mut stage, id, WindowCall::ArrowsVisible(false));
    assert_eq!(arrows(&stage.frame()), [0; 4]);
    call(&mut stage, id, WindowCall::ArrowsVisible(true));
    assert_eq!(arrows(&stage.frame()), [128, 0, 128, 0]);
}

#[test]
fn without_live_contents_the_arrows_stay() {
    for disposed in [false, true] {
        let mut stage = Stage::rgss2(120, 120);
        let skin = piece_skin(&mut stage);
        let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
        let bitmap = contents(&mut stage, id, 200);
        call(&mut stage, id, WindowCall::Ox(10));
        if disposed {
            stage.bitmaps.dispose(bitmap);
        } else {
            call(&mut stage, id, WindowCall::Contents(None));
        }
        call(&mut stage, id, WindowCall::Ox(0));
        let frame = stage.frame();
        assert_eq!(arrows(&frame), [128, 128, 128, 0], "disposed: {disposed}");
        assert_eq!(count(&frame, INK), 0);
    }
}

// Requirement "Order of the parts of a window of RGSS3".

#[test]
fn the_contents_lie_over_the_arrows() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    contents(&mut stage, id, 200);
    call(&mut stage, id, WindowCall::Ox(10));
    let frame = stage.frame();
    assert_eq!(count(&frame, INK), 7056);
    assert_fills(&frame, ARROW_DOWN, (42, 58, 92, 96));
    assert_fills(&frame, ARROW_LEFT, (4, 8, 42, 58));
    assert_fills(&frame, ARROW_RIGHT, (92, 96, 42, 58));
    call(&mut stage, id, WindowCall::Padding(0));
    let frame = stage.frame();
    assert_eq!(count(&frame, INK), 10000);
    assert_eq!(arrows(&frame), [0; 4]);
}

#[test]
fn the_contents_lie_over_the_pause_sign() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    call(&mut stage, id, WindowCall::Pause(true));
    calls(&mut stage, id, &[WindowCall::Update; 8]);
    assert_fills(&stage.frame(), PAUSE_FIRST, (42, 58, 84, 100));
    contents(&mut stage, id, 200);
    call(&mut stage, id, WindowCall::Padding(0));
    assert_eq!(count(&stage.frame(), PAUSE_FIRST), 0);
}

#[test]
fn the_contents_lie_over_the_cursor() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    call(&mut stage, id, rect(0, 0, 16, 16));
    assert_eq!(count(&stage.frame(), WHITE), 256);
    contents(&mut stage, id, 100);
    assert_eq!(count(&stage.frame(), WHITE), 0);
}

// Requirement "Window of RGSS3 without area".

/// Asserts that (100, 100) is `pixel` and that every other pixel of the frame is black.
#[track_caller]
fn assert_one_pixel(frame: &Frame, pixel: Rgba) {
    assert_eq!(frame.pixel(100, 100), pixel);
    let pixels = (frame.size.width * frame.size.height) as usize;
    assert_eq!(count(frame, BLACK), pixels - 1);
}

#[test]
fn a_window_without_area_shows_one_pixel_of_its_contents() {
    let mut stage = Stage::rgss2(160, 160);
    let id = window(&mut stage, None, (100, 100, 0, 50));
    contents(&mut stage, id, 100);
    // The default padding leaves the pixel out.
    assert!(all(&stage.frame(), BLACK));
    call(&mut stage, id, WindowCall::Padding(-3));
    assert_one_pixel(&stage.frame(), INK);
}

#[test]
fn a_window_without_area_has_no_background_and_no_frame() {
    let mut stage = Stage::rgss2(160, 160);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (100, 100, 50, 0));
    contents(&mut stage, id, 100);
    call(&mut stage, id, WindowCall::Padding(-3));
    assert_one_pixel(&stage.frame(), INK);
}

#[test]
fn an_arrow_shows_in_the_pixel_of_a_window_without_area() {
    let mut stage = Stage::rgss2(160, 160);
    let skin = readable_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (100, 100, 5, -3));
    call(&mut stage, id, WindowCall::ContentsOpacity(0));
    contents(&mut stage, id, 200);
    call(&mut stage, id, WindowCall::Width(5));
    // The right arrow at (-7, -9).
    assert_one_pixel(&stage.frame(), [111, 33, 100, 255]);
}

#[test]
fn the_pause_sign_shows_in_the_pixel_of_a_window_without_area() {
    let mut stage = Stage::rgss2(160, 160);
    let skin = readable_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (100, 100, -3, 10));
    call(&mut stage, id, WindowCall::Pause(true));
    calls(&mut stage, id, &[WindowCall::Update; 8]);
    // The pause sign at (-9, -6).
    assert_one_pixel(&stage.frame(), [105, 70, 100, 255]);
}

// Requirement "Disposed skin and contents of a window of RGSS3".

#[test]
fn a_disposed_skin_draws_nothing_of_the_skin() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    contents(&mut stage, id, 100);
    call(&mut stage, id, rect(0, 0, 16, 16));
    stage.bitmaps.dispose(skin);
    let pixels = 120 * 120;
    let frame = stage.frame();
    assert_eq!(count(&frame, INK), 7056);
    assert_eq!(count(&frame, BLACK), pixels - 7056);
    calls(
        &mut stage,
        id,
        &[WindowCall::Width(100), WindowCall::Update],
    );
    let frame = stage.frame();
    assert_eq!(count(&frame, INK), 7056);
    assert_eq!(count(&frame, BLACK), pixels - 7056);
}

#[test]
fn disposed_contents_are_not_drawn() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    let bitmap = contents(&mut stage, id, 200);
    call(&mut stage, id, WindowCall::Ox(10));
    stage.bitmaps.dispose(bitmap);
    let frame = stage.frame();
    assert_eq!(count(&frame, INK), 0);
    assert_eq!(arrows(&frame), [128, 128, 128, 0]);
}

// Requirement "Calls at which the original of RGSS3 faults".

#[test]
fn the_cursor_is_placed_in_a_window_that_never_had_a_skin() {
    let mut stage = Stage::rgss2(120, 120);
    let id = stage.scene.create_window_rgss3(None).unwrap();
    calls(
        &mut stage,
        id,
        &[
            WindowCall::Move(0, 0, 100, 100),
            WindowCall::Ox(0),
            WindowCall::Oy(0),
            WindowCall::Padding(8),
            WindowCall::PaddingBottom(8),
            rect(0, 0, 0, 0),
        ],
    );
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn sizes_too_large_for_a_bitmap_are_drawn_without_the_built_parts() {
    let mut stage = Stage::rgss2(120, 120);
    let skin = piece_skin(&mut stage);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    for assignment in [
        WindowCall::Move(0, 0, 20_000, 20_000),
        WindowCall::Move(0, 0, 100_000, 100_000),
        rect(0, 0, 100_000, 100_000),
    ] {
        call(&mut stage, id, assignment);
        let frame = stage.frame();
        assert_eq!(count(&frame, WHITE), 0);
        assert_eq!(count(&frame, FRAME_PIECES), 0);
    }
}

// Requirement "Skins smaller than the layout".

#[test]
fn the_background_of_a_skin_smaller_than_its_block_is_shortened() {
    for ((skin_width, skin_height), (width, height), filled) in [
        ((32, 32), (76, 132), Some((2, 37, 2, 64))),
        ((63, 128), (76, 68), Some((2, 72, 2, 66))),
        ((40, 128), (24, 24), Some((2, 14, 2, 22))),
        ((40, 128), (68, 68), Some((2, 42, 2, 66))),
        ((32, 32), (24, 24), Some((2, 11, 2, 11))),
        ((1, 1), (76, 76), None),
    ] {
        let mut stage = Stage::rgss2(140, 140);
        let skin = stage.solid(skin_width, skin_height, RED);
        let id = window(&mut stage, Some(skin), (0, 0, width, height));
        stage.scene.window_mut(id).unwrap().set_back_opacity(255);
        let frame = stage.frame();
        let filled = filled.unwrap_or_default();
        assert_fills(&frame, RED, filled);
        let pixels = ((filled.1 - filled.0) * (filled.3 - filled.2)) as usize;
        assert_eq!(count(&frame, BLACK), 140 * 140 - pixels);
    }
}

#[test]
fn centre_and_edges_of_the_cursor_are_shortened_by_a_skin_that_cuts_them() {
    let mut stage = Stage::rgss2(120, 120);
    let whole = piece_skin(&mut stage);
    let skin = stage.bitmaps.create(80, 128, &defaults()).unwrap();
    let left = Rect::new(0, 0, 80, 128);
    stage.bitmaps.blt(skin, 0, 0, whole, left, 255).unwrap();
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    call(&mut stage, id, rect(0, 0, 32, 32));
    assert_fills(&stage.frame(), WHITE, (8, 24, 8, 40));
    call(&mut stage, id, rect(0, 0, 16, 16));
    assert_fills(&stage.frame(), WHITE, (8, 15, 8, 24));
    call(&mut stage, id, rect(0, 0, 40, 40));
    assert_fills(&stage.frame(), WHITE, (8, 27, 8, 48));
}

/// An arrow that the skin cuts to an odd width blends its last column by the normal
/// formula: the cut width counts in the rule of the odd column.
#[test]
fn a_part_cut_by_the_skin_counts_its_cut_width() {
    let mut stage = Stage::rgss2(120, 120);
    let grey = stage.solid(120, 120, [200, 200, 200, 255]);
    sprite(&mut stage, grey, 0);
    // The down arrow at (88, 40, 16, 8), half transparent, of which the skin has 7 columns.
    let skin = stage.bitmaps.create(95, 128, &defaults()).unwrap();
    fill(&mut stage, skin, (88, 40, 7, 8), [40, 40, 40, 128]);
    let id = window(&mut stage, Some(skin), (0, 0, 100, 100));
    call(&mut stage, id, WindowCall::ContentsOpacity(0));
    contents(&mut stage, id, 200);
    let frame = stage.frame();
    // The window formula, and the normal one in the seventh column.
    assert_eq!(frame.pixel(47, 90), [120, 120, 120, 255]);
    assert_eq!(frame.pixel(48, 90), [119, 119, 119, 255]);
    assert_eq!(frame.pixel(49, 90), [200, 200, 200, 255]);
}
