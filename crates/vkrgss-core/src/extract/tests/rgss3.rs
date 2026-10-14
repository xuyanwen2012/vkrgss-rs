//! The scene of RGSS3: what it shares with RGSS2, and the rules measured on the original
//! runtime of VX Ace on 2026-10-13 (versions 3.0.0.1 and 3.0.1.1, `ace-v1-sprite-REPORT.md`).
//! The expected numbers are those the model of the measurement gives.

use vkrgss_compose_ref::{Compositor, Frame};
use vkrgss_proto::{Op, Sampling};

use super::*;
use crate::extract::rgss2::wave_offset;
use crate::test_support::sprite_cases::{cases, digest, frames_by};
use crate::{ElementKind, SceneError};

/// Composes every frame of every scene of `text`, extracted from a scene of `rules`, with
/// the reference compositor, and returns the names of the frames that are not the picture
/// their digest names.
fn wrong_frames(text: &str, rules: Rules, scenes: usize) -> Vec<String> {
    let cases = cases(text);
    assert_eq!(cases.len(), scenes);
    let mut wrong = Vec::new();
    for case in cases {
        let mut compositor = Compositor::new();
        for (index, (packet, expected)) in frames_by(rules, &case).into_iter().enumerate() {
            let frame = compositor.render(&packet).unwrap();
            if digest(frame.size, &frame.rgba) != expected {
                wrong.push(format!("{}, frame {index}", case.name));
            }
        }
    }
    wrong
}

/// Scenario "Scenes that RGSS3 shares with RGSS2, measured on the original": the files of
/// sprites. The topics of `rgss2-scene-cases.txt` are run in `scene_cases`.
#[test]
fn the_scenes_of_sprites_of_rgss2_are_the_same_pictures_in_rgss3() {
    for (text, scenes) in [
        (
            include_str!("../../../tests/data/sprite-rgss2-place.txt"),
            14,
        ),
        (include_str!("../../../tests/data/sprite-rgss2-bush.txt"), 5),
        (
            include_str!("../../../tests/data/sprite-rgss2-wave.txt"),
            98,
        ),
    ] {
        assert_eq!(wrong_frames(text, Rules::Rgss3, scenes), [""; 0]);
    }
}

/// Scenario "y orders sprites of equal z".
#[test]
fn in_rgss3_the_larger_y_is_in_front_among_equal_z() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 8, 8);
    red_with_y_then_green(&mut stage, 9);
    assert_eq!(stage.frame().pixel(1, 1), RED);
}

/// Scenario "Flash without a colour".
#[test]
fn in_rgss3_a_viewport_flash_without_a_colour_leaves_the_viewport_undone() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 8, 8);
    let viewport = tinted_half(&mut stage);
    let white = stage.solid(1, 1, WHITE);
    sprite_in(&mut stage, Some(viewport), white, 1, 1);
    stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(-100.0, -100.0, 0.0, 0.0);
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
    // The flash is over: the sprite is drawn and the tone applied.
    assert_eq!(stage.frame().pixel(1, 1), [155, 155, 255, 255]);
}

/// Scenario "Plane tile and sprite width": the sprite. The plane is in `plane::tests`.
#[test]
fn in_rgss3_the_width_of_a_sprite_depends_on_the_place() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 544, 16);
    let bitmap = stage.solid(90, 8, WHITE);
    let id = sprite(&mut stage, bitmap, 272, 0);
    stage.scene.sprite_mut(id).unwrap().zoom_x = 0.7;
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let widths: Vec<u32> = packet
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Quad(quad) => match quad.sampling {
                Sampling::Stretch { dest, .. } => Some(dest.width),
                Sampling::Affine(_) => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(widths, [63]);
}

/// Scenario "Kept by a clear".
#[test]
fn a_cleared_scene_of_rgss3_keeps_its_rules_and_gives_them_to_its_sprites() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 320, 240);
    stage.scene.clear();
    assert_eq!(stage.scene.rules(), Rules::Rgss3);
    half_turn(&mut stage, Rect::new(100, 50, 60, 40), (10, 4), 180.0);
    assert_eq!(drawn(&stage.frame()), Some((120, 105, 58, 119, 65)));
}

/// An opaque bitmap whose texel column c is (c, 255 - c, 128).
fn colx(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
    let pixels: Vec<Rgba> = (0..height)
        .flat_map(|_| (0..width).map(|x| [x as u8, 255 - x as u8, 128, 255]))
        .collect();
    stage.pixels(width, height, &pixels)
}

/// The number of pixels of `frame` that are not black, and the first and last column and
/// row that hold one: (count, left, top, right, bottom). `None` when all are black.
fn drawn(frame: &Frame) -> Option<(usize, u32, u32, u32, u32)> {
    let mut found: Option<(usize, u32, u32, u32, u32)> = None;
    for y in 0..frame.size.height {
        for x in 0..frame.size.width {
            if frame.pixel(x, y) == BLACK {
                continue;
            }
            let (count, left, top, right, bottom) = found.unwrap_or((0, x, y, x, y));
            found = Some((
                count + 1,
                left.min(x),
                top.min(y),
                right.max(x),
                bottom.max(y),
            ));
        }
    }
    found
}

/// The sprite of the scenarios of turned sprites: the bitmap [`colx`] of 16 by 8 at
/// (30, 20) with `angle`, in a viewport of `rect` with the origin `origin`.
fn half_turn(stage: &mut Stage, rect: Rect, origin: (i32, i32), angle: f64) -> SpriteId {
    let viewport = stage.scene.create_viewport(rect).unwrap();
    let inside = stage.scene.viewport_mut(viewport).unwrap();
    (inside.ox, inside.oy) = origin;
    let bitmap = colx(stage, 16, 8);
    let id = sprite_in(stage, Some(viewport), bitmap, 30, 20);
    stage.scene.sprite_mut(id).unwrap().angle = angle;
    id
}

/// Scenarios "Half turn, measured rule" and "Quarter turn, measured rule".
#[test]
fn in_rgss3_a_turned_sprite_in_a_viewport_stands_at_the_sum_of_the_positions() {
    let rect = Rect::new(100, 50, 60, 40);
    let mut stage = Stage::with_rules(Rules::Rgss3, 320, 240);
    half_turn(&mut stage, rect, (10, 4), 180.0);
    let frame = stage.frame();
    assert_eq!(drawn(&frame), Some((120, 105, 58, 119, 65)));
    for y in 58..=65 {
        assert_eq!(frame.pixel(105, y), [15, 240, 128, 255]);
        assert_eq!(frame.pixel(119, y), [1, 254, 128, 255]);
    }

    let mut stage = Stage::with_rules(Rules::Rgss3, 320, 240);
    half_turn(&mut stage, rect, (10, 4), 90.0);
    assert_eq!(drawn(&stage.frame()), Some((105, 121, 51, 127, 65)));
}

/// Scenario "As on the screen".
#[test]
fn in_rgss3_a_turned_sprite_in_a_viewport_is_the_sprite_on_the_screen() {
    for angle in [180.0, 90.0] {
        let mut inside = Stage::with_rules(Rules::Rgss3, 320, 240);
        half_turn(&mut inside, Rect::new(100, 50, 60, 40), (10, 4), angle);
        let mut free = Stage::with_rules(Rules::Rgss3, 320, 240);
        let bitmap = colx(&mut free, 16, 8);
        let id = sprite(&mut free, bitmap, 120, 66);
        free.scene.sprite_mut(id).unwrap().angle = angle;
        assert_eq!(inside.frame().rgba, free.frame().rgba, "angle {angle}");
    }
}

/// Scenario "Clipped by the viewport".
#[test]
fn in_rgss3_a_turned_sprite_is_clipped_by_its_viewport() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 320, 240);
    half_turn(&mut stage, Rect::new(100, 50, 15, 10), (10, 4), 180.0);
    assert_eq!(drawn(&stage.frame()), Some((20, 105, 58, 114, 59)));
}

/// Scenario "A large origin only moves the sprite".
#[test]
fn in_rgss3_a_large_origin_only_moves_a_turned_sprite() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 320, 240);
    half_turn(&mut stage, Rect::new(100, 50, 200, 150), (-50, -30), 180.0);
    assert_eq!(drawn(&stage.frame()), Some((120, 165, 92, 179, 99)));
}

/// Scenario "Source rectangle leaving the bitmap" of "Turned sprites in viewports of
/// RGSS3".
#[test]
fn in_rgss3_a_turned_sprite_with_a_cut_source_rectangle_is_not_drawn() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 320, 240);
    let viewport = Rect::new(10, 10, 200, 200);
    let viewport = stage.scene.create_viewport(viewport).unwrap();
    let bitmap = stage.solid(24, 16, WHITE);
    let id = sprite_in(&mut stage, Some(viewport), bitmap, 60, 60);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.angle = 30.0;
    assert!(drawn(&stage.frame()).is_some());
    stage.scene.sprite_mut(id).unwrap().src_rect = Rect::new(-1, 0, 24, 16);
    assert_eq!(drawn(&stage.frame()), None);
}

/// Scenario "The shift stays in RGSS1 and RGSS2".
#[test]
fn in_rgss1_and_rgss2_a_turned_sprite_in_a_viewport_keeps_its_shift() {
    for rules in [Rules::Rgss1, Rules::Rgss2] {
        let rect = Rect::new(100, 50, 60, 40);
        let mut stage = Stage::with_rules(rules, 320, 240);
        half_turn(&mut stage, rect, (10, 4), 180.0);
        let frame = stage.frame();
        assert_eq!(drawn(&frame), Some((20, 115, 62, 119, 65)), "{rules:?}");
        for y in 62..=65 {
            let reds: Vec<u8> = (115..=119).map(|x| frame.pixel(x, y)[0]).collect();
            assert_eq!(reds, [15, 14, 13, 12, 11], "{rules:?}");
        }

        let mut stage = Stage::with_rules(rules, 320, 240);
        half_turn(&mut stage, rect, (10, 4), 90.0);
        assert_eq!(drawn(&stage.frame()), None, "{rules:?}");
    }
}

/// Every pixel of `frame` is white inside the columns and rows given, both inclusive, and
/// black outside.
fn covers(frame: &Frame, columns: (u32, u32), rows: (u32, u32)) -> bool {
    (0..frame.size.height).all(|y| {
        (0..frame.size.width).all(|x| {
            let inside = (columns.0..=columns.1).contains(&x) && (rows.0..=rows.1).contains(&y);
            frame.pixel(x, y) == if inside { WHITE } else { BLACK }
        })
    })
}

/// A scene of RGSS3 of 544 by 416 with a white plane: in a viewport of the screen, in one
/// of (100, 100, 600, 500), or without a viewport.
fn white_plane(place: usize) -> Stage {
    let mut stage = Stage::with_rules(Rules::Rgss3, 544, 416);
    let viewport = match place {
        0 => Some(stage.scene.create_screen_viewport().unwrap()),
        1 => {
            let rect = Rect::new(100, 100, 600, 500);
            Some(stage.scene.create_viewport(rect).unwrap())
        }
        _ => None,
    };
    let white = stage.solid(32, 32, WHITE);
    let plane = stage.scene.create_plane(viewport).unwrap();
    stage.scene.plane_mut(plane).unwrap().bitmap = Some(white);
    stage
}

/// Scenarios "Larger screen, measured on the original of RGSS3" and "Smaller screen,
/// measured on the original of RGSS3".
#[test]
fn a_change_of_the_size_changes_no_viewport_and_frees_the_screen() {
    let larger = [
        ((0, 543), (0, 415)),
        ((100, 639), (100, 479)),
        ((0, 639), (0, 479)),
    ];
    let smaller = [((0, 319), (0, 239)), ((100, 319), (100, 239))];
    for (place, (columns, rows)) in larger.into_iter().enumerate() {
        let mut stage = white_plane(place);
        stage.scene.set_logical_size(Size::new(640, 480));
        let frame = stage.frame();
        assert_eq!(frame.size, Size::new(640, 480));
        assert!(covers(&frame, columns, rows), "larger, plane {place}");
        if let Some((columns, rows)) = smaller.get(place) {
            stage.scene.set_logical_size(Size::new(320, 240));
            let frame = stage.frame();
            assert_eq!(frame.size, Size::new(320, 240));
            assert!(covers(&frame, *columns, *rows), "smaller, plane {place}");
        }
    }
}

/// The red of the pixel at (`x`, `y`), or `None` where nothing is drawn.
fn red(frame: &Frame, x: u32, y: u32) -> Option<u8> {
    let pixel = frame.pixel(x, y);
    (pixel != BLACK).then_some(pixel[0])
}

/// Scenarios "A plane follows its viewport, measured on the original of RGSS3" and "To the
/// screen", by every rule set.
#[test]
fn a_moved_plane_is_drawn_by_its_new_viewport() {
    for rules in [Rules::Rgss1, Rules::Rgss2, Rules::Rgss3] {
        let mut stage = Stage::with_rules(rules, 544, 416);
        let first = Rect::new(4, 4, 60, 50);
        let first = stage.scene.create_viewport(first).unwrap();
        let second = Rect::new(70, 20, 80, 90);
        let second = stage.scene.create_viewport(second).unwrap();
        let origin = stage.scene.viewport_mut(second).unwrap();
        (origin.ox, origin.oy) = (7, 3);
        let bitmap = colx(&mut stage, 24, 16);
        let plane = stage.scene.create_plane(Some(first)).unwrap();
        stage.scene.plane_mut(plane).unwrap().bitmap = Some(bitmap);
        stage.scene.plane_mut(plane).unwrap().ox = 5;
        let frame = stage.frame();
        assert_eq!(red(&frame, 4, 4), Some(5), "{rules:?}");
        assert_eq!(red(&frame, 64, 54), None, "{rules:?}");

        stage.scene.set_plane_viewport(plane, Some(second)).unwrap();
        let frame = stage.frame();
        assert_eq!(red(&frame, 4, 4), None, "{rules:?}");
        assert_eq!(red(&frame, 70, 20), Some(12), "{rules:?}");
        assert_eq!(red(&frame, 149, 109), Some(19), "{rules:?}");
        assert_eq!(red(&frame, 150, 110), None, "{rules:?}");

        stage.scene.set_plane_viewport(plane, None).unwrap();
        let frame = stage.frame();
        assert_eq!(red(&frame, 0, 0), Some(5), "{rules:?}");
        assert_eq!(red(&frame, 543, 415), Some(20), "{rules:?}");
    }
}

/// Scenario "The rank stays, measured on the original of RGSS3".
#[test]
fn a_moved_plane_keeps_the_rank_of_its_creation() {
    for rules in [Rules::Rgss1, Rules::Rgss2, Rules::Rgss3] {
        for (plane_first, front) in [(true, GREEN), (false, RED)] {
            let mut stage = Stage::with_rules(rules, 16, 16);
            let rect = Rect::new(2, 2, 8, 8);
            let of_plane = stage.scene.create_viewport(rect).unwrap();
            let of_sprite = stage.scene.create_viewport(rect).unwrap();
            let (red, green) = (stage.solid(4, 4, RED), stage.solid(4, 4, GREEN));
            let plane = |stage: &mut Stage| {
                let plane = stage.scene.create_plane(Some(of_plane)).unwrap();
                stage.scene.plane_mut(plane).unwrap().bitmap = Some(red);
                plane
            };
            let plane = if plane_first {
                let plane = plane(&mut stage);
                sprite_in(&mut stage, Some(of_sprite), green, 0, 0);
                plane
            } else {
                sprite_in(&mut stage, Some(of_sprite), green, 0, 0);
                plane(&mut stage)
            };
            let moved = stage.scene.set_plane_viewport(plane, Some(of_sprite));
            assert_eq!(moved, Ok(()));
            let frame = stage.frame();
            assert_eq!(
                frame.pixel(3, 3),
                front,
                "{rules:?}, plane first {plane_first}"
            );
            assert_eq!(frame.pixel(8, 8), RED, "{rules:?}, beside the sprite");
        }
    }
}

/// Scenario "Disposed viewport": the original runtime of RGSS3 ends here.
#[test]
fn a_move_into_a_disposed_viewport_fails_and_moves_nothing() {
    for rules in [Rules::Rgss1, Rules::Rgss2, Rules::Rgss3] {
        let mut stage = Stage::with_rules(rules, 16, 16);
        let home = stage.scene.create_viewport(Rect::new(0, 0, 8, 8)).unwrap();
        let gone = stage.scene.create_viewport(Rect::new(8, 8, 8, 8)).unwrap();
        stage.scene.dispose_viewport(gone, &mut stage.bitmaps);
        let (red, green) = (stage.solid(2, 2, RED), stage.solid(2, 2, GREEN));
        let sprite = sprite_in(&mut stage, Some(home), red, 0, 0);
        let plane = stage.scene.create_plane(None).unwrap();
        stage.scene.plane_mut(plane).unwrap().bitmap = Some(green);
        stage.scene.set_plane_z(plane, -1).unwrap();
        let disposed = Err(SceneError::Disposed(ElementKind::Viewport));
        assert_eq!(
            stage.scene.set_sprite_viewport(sprite, Some(gone)),
            disposed
        );
        assert_eq!(stage.scene.set_plane_viewport(plane, Some(gone)), disposed);
        assert_eq!(stage.scene.sprite(sprite).unwrap().viewport, Some(home));
        assert_eq!(stage.scene.plane(plane).unwrap().viewport, None);
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), RED, "{rules:?}");
        assert_eq!(frame.pixel(15, 15), GREEN, "{rules:?}");
    }
}

/// Scenario "The sprite that ends the original": the original runtime of RGSS3 ends with a
/// page fault inside `Graphics.update` when it draws this sprite, so there is no picture to
/// compare with; it is drawn here by the rules of a turned sprite.
#[test]
fn the_sprite_that_ends_the_original_of_rgss3_is_drawn() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 544, 416);
    let bitmap = stage.solid(20, 90, WHITE);
    let id = sprite(&mut stage, bitmap, 434, 160);
    let sprite = stage.scene.sprite_mut(id).unwrap();
    sprite.src_rect = Rect::new(8, 38, 9, 26);
    (sprite.zoom_x, sprite.zoom_y) = (2.5, 0.7);
    sprite.angle = 180.0;
    sprite.mirror = true;
    sprite.bush_depth = 20;
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let frame = stage.render(&packet).unwrap();
    assert_eq!(drawn(&frame), Some((360, 414, 142, 433, 159)));
}

/// Scenario "Negative wave length": the original runtime of RGSS3 reads outside its sine
/// table there.
#[test]
fn a_negative_wave_length_takes_the_angle_modulo_360() {
    // 8 rows of 360 / -50 degrees are -57.6, whose floor is 302 modulo 360.
    assert_eq!(wave_offset(10, -50, 0.0, 8, 8, 0), -8);
}

const TURNED: &str = include_str!("../../../tests/data/sprite-rgss3-turned.txt");
const MOVED: &str = include_str!("../../../tests/data/sprite-rgss3-moved.txt");
const RANDOM: &str = include_str!("../../../tests/data/sprite-rgss3-random.txt");

/// The number of `frame` statements of `text`.
fn frame_statements(text: &str) -> usize {
    let frames = text.lines().filter(|line| line.starts_with("frame "));
    frames.count()
}

/// Scenario "Turned sprites in viewports" of "Whole scenes of sprites of RGSS3 as
/// measured": the 8 frames of `atvp` and the 4 of `turnedvp`. `vkrgss-vk` composes the same
/// scenes with both backends.
#[test]
fn the_turned_sprites_in_viewports_of_rgss3_are_the_measured_pictures() {
    assert_eq!(frame_statements(TURNED), 12);
    assert_eq!(wrong_frames(TURNED, Rules::Rgss3, 12), [""; 0]);
}

/// Scenario "Moved planes and sprites": the 6 frames of `aplanevp`.
#[test]
fn the_moved_planes_and_sprites_of_rgss3_are_the_measured_pictures() {
    assert_eq!(frame_statements(MOVED), 6);
    assert_eq!(wrong_frames(MOVED, Rules::Rgss3, 2), [""; 0]);
}

/// Scenario "Random scenes held out": the 20 scenes of `aheldx`, none left out.
#[test]
fn the_random_scenes_of_rgss3_are_the_measured_pictures() {
    assert_eq!(frame_statements(RANDOM), 20);
    assert_eq!(wrong_frames(RANDOM, Rules::Rgss3, 20), [""; 0]);
}

/// Scenario "Not the rule of RGSS2".
#[test]
fn the_turned_sprites_in_viewports_of_rgss3_are_not_those_of_rgss2() {
    let wrong = wrong_frames(TURNED, Rules::Rgss2, 12);
    assert!(
        wrong.iter().any(|name| name.starts_with("atvp-")),
        "{wrong:?}"
    );
}

/// Scenario "Nothing of the original": the files hold comments, statements of scenes with
/// numbers, names of bitmaps made by formula and digests, and their heads name the topic,
/// the model and the DLL versions.
#[test]
fn the_files_of_scenes_of_rgss3_hold_scenes_and_digests_only() {
    for (text, topics) in [
        (TURNED, &["`atvp`", "`turnedvp`"][..]),
        (MOVED, &["`aplanevp`"]),
        (RANDOM, &["`aheldx`"]),
    ] {
        let head: Vec<&str> = text
            .lines()
            .take_while(|line| line.starts_with('#'))
            .collect();
        let head = head.join("\n");
        for word in ["ace-v1-sprite-model.py", "3.0.0.1", "3.0.1.1"] {
            assert!(head.contains(word), "the head names `{word}`");
        }
        for topic in topics {
            assert!(head.contains(topic), "the head names {topic}");
        }
        let statements = text.lines().filter(|line| !line.starts_with('#'));
        for line in statements {
            let word = line.split(' ').next().unwrap();
            let known = [
                "scene", "sprite", "plane", "viewport", "set", "flash", "update", "frame", "end",
            ];
            assert!(known.contains(&word), "`{line}`");
            assert!(line.is_ascii() && line.len() < 400, "`{line}`");
        }
    }
}

/// Scenario "The scene as it is and the brightness of the call" of "Pixels of a snapshot".
#[test]
fn a_snapshot_shows_the_scene_as_it_is_at_the_brightness_of_the_call() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 8, 8);
    let bitmap = stage.solid(4, 4, RED);
    sprite(&mut stage, bitmap, 2, 2);
    assert_eq!(stage.frame().pixel(3, 3), RED);
    let blue = crate::Color::new(0.0, 0.0, 255.0, 255.0);
    stage
        .bitmaps
        .fill_rect(bitmap, Rect::new(0, 0, 4, 4), blue)
        .unwrap();
    stage.scene.set_brightness(100);
    let mut twin = Stage::with_rules(Rules::Rgss3, 8, 8);
    twin.scene = stage.scene.clone();
    twin.bitmaps = stage.bitmaps.clone();

    let packet = stage.scene.snapshot(&mut stage.bitmaps);
    let snapped = vkrgss_compose_ref::snapshot(&packet, vkrgss_compose_ref::SnapshotRule::Rgss3);
    let snapped = snapped.unwrap();
    for (x, y, expected) in [
        (2, 2, [0, 0, 99, 255]),
        (5, 5, [0, 0, 99, 255]),
        (1, 1, BLACK),
    ] {
        assert_eq!(snapped.pixel(x, y), expected);
    }
    // The next extracted packet is the one it would have been without the snapshot.
    let next = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(next, twin.scene.extract(&mut twin.bitmaps));
}
