//! Scenes of sprites of RGSS2, extracted by the scene of the guest and composed by both
//! backends: the cases and the random scenes of the sprite measurement of 2026-10-12 on the
//! original runtime of VX, whose pictures are committed as digests beside the scenes in
//! `vkxp-core`.

use vkxp_core::test_support::sprite_cases::{cases, digest, frames};
use vkxp_proto::GuestToHost;

use super::differential::Pair;

/// Composes every frame of every scene of `text` with both backends. The two must agree
/// byte for byte, and the frame must be the picture its digest names.
fn compare(text: &str, scenes: usize) {
    let cases = cases(text);
    assert_eq!(cases.len(), scenes);
    let mut wrong = Vec::new();
    for case in cases {
        let mut pair = Pair::new();
        for (index, (packet, expected)) in frames(&case).into_iter().enumerate() {
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
    let text = include_str!("../../../vkxp-core/tests/data/sprite-rgss2-random.txt");
    compare(text, 20);
}

/// The cases of the measurement: place and size, bush rows, waves.
#[test]
fn the_cases_of_sprites_of_rgss2_are_the_expected_pictures() {
    compare(
        include_str!("../../../vkxp-core/tests/data/sprite-rgss2-place.txt"),
        14,
    );
    compare(
        include_str!("../../../vkxp-core/tests/data/sprite-rgss2-bush.txt"),
        5,
    );
    compare(
        include_str!("../../../vkxp-core/tests/data/sprite-rgss2-wave.txt"),
        98,
    );
}
