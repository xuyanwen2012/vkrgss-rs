//! The collection stress test of the scene classes: every snippet of `scene_cases` gives
//! the same result when a garbage collection runs at every allocation.
//!
//! See `tests/stress.rs` for what `GC.stress` does.

mod cases;
mod scene_cases;
mod scene_support;

use vkrgss_binding::testing::{self, Harness};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS1.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

/// The snippet with stress on while it runs, and off again whatever happens.
fn stressed(source: &str) -> String {
    format!("GC.stress = true\nbegin\n{source}\nensure\nGC.stress = false\nend")
}

#[test]
fn every_snippet_gives_the_same_result_under_stress() {
    scene_support::install(harness());
    let mut wrong = Vec::new();
    for (group, cases) in scene_cases::GROUPS {
        // Without stress first: a snippet that fails here is not a finding of this test.
        let plain = cases::failures(harness(), cases, str::to_owned);
        assert!(plain.is_empty(), "{group}: {}", plain.join("\n"));
        wrong.extend(
            cases::failures(harness(), cases, stressed)
                .into_iter()
                .map(|failure| format!("{group}: {failure}")),
        );
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
