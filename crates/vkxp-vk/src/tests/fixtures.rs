//! Every committed fixture stream of the reference compositor, rendered by both backends.

use vkxp_compose_ref::fixtures;

use super::differential::{compare_stream, compare_stream_timed};

#[test]
fn every_fixture_stream_is_byte_identical() {
    let all = fixtures::all();
    assert!(all.len() >= 11, "the fixture list shrank to {}", all.len());
    let failures: Vec<String> = all
        .iter()
        .filter_map(|fixture| {
            assert!(
                fixture
                    .messages
                    .iter()
                    .any(|message| matches!(message, vkxp_proto::GuestToHost::Frame(_))),
                "fixture `{}` renders nothing",
                fixture.name
            );
            compare_stream(&fixture.messages)
                .err()
                .map(|difference| format!("{}: {difference}", fixture.name))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "fixtures differ:\n{}",
        failures.join("\n")
    );
}

/// Measuring changes no frame: the same streams with frame timing on, under the validation
/// layer like every test.
#[test]
fn every_fixture_stream_is_byte_identical_with_frame_timing_on() {
    let failures: Vec<String> = fixtures::all()
        .iter()
        .filter_map(|fixture| {
            compare_stream_timed(&fixture.messages)
                .err()
                .map(|difference| format!("{}: {difference}", fixture.name))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "fixtures differ with frame timing on:\n{}",
        failures.join("\n")
    );
}
