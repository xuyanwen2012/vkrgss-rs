//! The collection stress test: every Ruby snippet of the binding's tests gives the same
//! result when a garbage collection runs at every allocation.
//!
//! `GC.stress = true` makes the interpreter collect whenever it allocates an object, which
//! is every place where a value the binding holds only in Rust memory would be lost, and
//! where a free function could run while a method is at work.

mod cases;

use vkxp_binding::testing;

/// The snippet with stress on while it runs, and off again whatever happens.
fn stressed(source: &str) -> String {
    format!("GC.stress = true\nbegin\n{source}\nensure\nGC.stress = false\nend")
}

#[test]
fn the_interpreter_supports_stress() {
    assert_eq!(
        testing::eval("GC.stress = true; on = GC.stress; GC.stress = false; [on, GC.stress]")
            .as_deref(),
        Ok("[true, false]")
    );
}

#[test]
fn every_snippet_gives_the_same_result_under_stress() {
    let mut wrong = Vec::new();
    for (group, cases) in cases::GROUPS {
        // Without stress first: a snippet that fails here is not a finding of this test.
        let plain = cases::failures(cases, str::to_owned);
        assert!(plain.is_empty(), "{group}: {}", plain.join("\n"));
        wrong.extend(
            cases::failures(cases, stressed)
                .into_iter()
                .map(|failure| format!("{group}: {failure}")),
        );
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn unreachable_bitmaps_are_freed_under_stress() {
    assert_eq!(
        testing::eval(&stressed(cases::UNREACHABLE_BITMAPS)).as_deref(),
        Ok("nil")
    );
    let remaining = testing::bitmap_count();
    assert!(remaining < 500, "{remaining} bitmaps are left in the arena");
}
