//! Scenes of windows of RGSS2 and RGSS3, extracted by the scene of the guest and composed by
//! both backends: the cases and the random scenes of the window measurements of 2026-10-12 on
//! the original runtime of VX and of 2026-10-13 on that of VX Ace, whose pictures are
//! committed as digests beside the scenes in `vkrgss-core`.

use vkrgss_core::test_support::window_cases::{cases, digest};
use vkrgss_proto::GuestToHost;

use super::differential::Pair;

/// Composes every frame of every scene of `text` with both backends. The two must agree
/// byte for byte, and the frame must be the picture its digest names.
fn compare(text: &str, scenes: usize) {
    let cases = cases(text);
    assert_eq!(cases.len(), scenes);
    let mut wrong = Vec::new();
    for case in cases {
        let mut pair = Pair::new();
        for (index, (packet, expected)) in case.frames().into_iter().enumerate() {
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

/// Scenario "Random scenes" of "Whole windows of RGSS2 as measured".
#[test]
fn the_random_scenes_of_windows_of_rgss2_are_the_expected_pictures() {
    let text = include_str!("../../../vkrgss-core/tests/data/window-rgss2-random.txt");
    compare(text, 30);
}

/// The cases of the measurement, at least one for each rule of the window of RGSS2.
#[test]
fn the_cases_of_windows_of_rgss2_are_the_expected_pictures() {
    let text = include_str!("../../../vkrgss-core/tests/data/window-rgss2-cases.txt");
    compare(text, 27);
}

/// Scenario "Both backends" of "Whole windows of RGSS3 as measured", the random scenes.
#[test]
fn the_random_scenes_of_windows_of_rgss3_are_the_expected_pictures() {
    let text = include_str!("../../../vkrgss-core/tests/data/window-rgss3-random.txt");
    compare(text, 30);
}

/// Scenario "Both backends" of "Whole windows of RGSS3 as measured", the cases of the
/// measurement.
#[test]
fn the_cases_of_windows_of_rgss3_are_the_expected_pictures() {
    let text = include_str!("../../../vkrgss-core/tests/data/window-rgss3-cases.txt");
    compare(text, 28);
}
