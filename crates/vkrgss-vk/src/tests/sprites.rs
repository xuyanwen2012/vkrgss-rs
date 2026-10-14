//! Scenes of sprites of RGSS2 and RGSS3, extracted by the scene of the guest and composed
//! by both backends: the cases and the random scenes of the sprite measurements of
//! 2026-10-12 on the original runtime of VX and of 2026-10-13 on that of VX Ace, whose
//! pictures are committed as digests beside the scenes in `vkrgss-core`.

use vkrgss_core::test_support::sprite_cases::{cases, digest, frames_by};
use vkrgss_core::{Bitmaps, FontDefaults, Rect, Rules, Scene};
use vkrgss_proto::{GuestToHost, Size};

use super::differential::Pair;

/// Composes every frame of every scene of `text` with both backends. The two must agree
/// byte for byte, and the frame must be the picture its digest names.
fn compare(text: &str, scenes: usize) {
    compare_by(Rules::Rgss2, text, scenes);
}

/// [`compare`] with the scenes extracted from a scene of `rules`.
fn compare_by(rules: Rules, text: &str, scenes: usize) {
    let cases = cases(text);
    assert_eq!(cases.len(), scenes);
    let mut wrong = Vec::new();
    for case in cases {
        let mut pair = Pair::new();
        for (index, (packet, expected)) in frames_by(rules, &case).into_iter().enumerate() {
            let name = format!("{}, frame {index}", case.name);
            match pair.step(&GuestToHost::Frame(packet)) {
                Ok(Ok(Some(frame))) if digest(frame.size, &frame.rgba) == expected => {}
                Ok(Ok(Some(_))) => wrong.push(format!("{name}: not the expected picture")),
                Ok(Ok(None)) => wrong.push(format!("{name}: no frame")),
                Ok(Err(error)) => wrong.push(format!("{name}: {error}")),
                Err(difference) => wrong.push(format!("{name}: {difference}")),
            }
        }
    }
    assert!(wrong.is_empty(), "frames differ:\n{}", wrong.join("\n"));
}

/// Scenario "Random scenes" of "Whole sprites of RGSS2 as measured".
#[test]
fn the_random_scenes_of_sprites_of_rgss2_are_the_expected_pictures() {
    let text = include_str!("../../../vkrgss-core/tests/data/sprite-rgss2-random.txt");
    compare(text, 20);
}

/// The cases of the measurement: place and size, bush rows, waves.
#[test]
fn the_cases_of_sprites_of_rgss2_are_the_expected_pictures() {
    compare(
        include_str!("../../../vkrgss-core/tests/data/sprite-rgss2-place.txt"),
        14,
    );
    compare(
        include_str!("../../../vkrgss-core/tests/data/sprite-rgss2-bush.txt"),
        5,
    );
    compare(
        include_str!("../../../vkrgss-core/tests/data/sprite-rgss2-wave.txt"),
        98,
    );
}

/// Scenario "Both backends" of "Whole scenes of sprites of RGSS3 as measured": the topics
/// `atvp`, `turnedvp`, `aplanevp` and `aheldx`.
#[test]
fn the_scenes_of_sprites_of_rgss3_are_the_expected_pictures() {
    compare_by(
        Rules::Rgss3,
        include_str!("../../../vkrgss-core/tests/data/sprite-rgss3-turned.txt"),
        12,
    );
    compare_by(
        Rules::Rgss3,
        include_str!("../../../vkrgss-core/tests/data/sprite-rgss3-moved.txt"),
        2,
    );
    compare_by(
        Rules::Rgss3,
        include_str!("../../../vkrgss-core/tests/data/sprite-rgss3-random.txt"),
        20,
    );
}

/// "Sprites at which the original of RGSS3 faults": the sprite that ends the original
/// runtime inside `Graphics.update` is extracted into a packet both compositors accept and
/// draw alike.
#[test]
fn the_sprite_that_ends_the_original_of_rgss3_is_composed_by_both_backends() {
    let mut scene = Scene::with_rules(Rules::Rgss3, Size::new(544, 416));
    let mut bitmaps = Bitmaps::new();
    let defaults = FontDefaults::new(Vec::new());
    let white = vec![255; 20 * 90 * 4];
    let bitmap = bitmaps.create_from_rgba(20, 90, white, &defaults).unwrap();
    let id = scene.create_sprite(None).unwrap();
    let sprite = scene.sprite_mut(id).unwrap();
    sprite.set_bitmap(Some(bitmap), &bitmaps);
    sprite.src_rect = Rect::new(8, 38, 9, 26);
    (sprite.x, sprite.y) = (434, 160);
    (sprite.zoom_x, sprite.zoom_y) = (2.5, 0.7);
    sprite.angle = 180.0;
    sprite.mirror = true;
    sprite.bush_depth = 20;
    let packet = scene.extract(&mut bitmaps);
    let mut pair = Pair::new();
    let frame = pair.step(&GuestToHost::Frame(packet));
    let frame = frame.expect("the backends agree").expect("no error");
    let frame = frame.expect("a frame");
    let drawn = frame.rgba.as_chunks::<4>().0.iter();
    assert_eq!(drawn.filter(|pixel| pixel[..3] != [0, 0, 0]).count(), 360);
}
