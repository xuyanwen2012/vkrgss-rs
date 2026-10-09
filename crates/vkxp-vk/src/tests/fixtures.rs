//! Every committed fixture stream of the reference compositor, rendered by both backends.

use vkxp_compose_ref::fixtures;

use super::differential::compare_stream;

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
