//! The allow-list of snippet cases over Ruby 1.9.2: a listed case counts while it fails and
//! is reported when it passes.
#![cfg(not(vkxp_interpreter_absent))]
extern crate vkxp_binding_rgss3 as vkxp_binding;

#[path = "../../vkxp-binding/tests/cases/mod.rs"]
mod cases;

use cases::{Case, Expect};

#[test]
fn a_listed_case_is_reported_only_when_it_passes() {
    let cases = [
        Case {
            name: "listed and failing",
            source: "1 + 1",
            expect: Expect::Value("3"),
        },
        Case {
            name: "listed and passing",
            source: "1 + 1",
            expect: Expect::Value("2"),
        },
        Case {
            name: "not listed and failing",
            source: "1 + 1",
            expect: Expect::Value("4"),
        },
    ];
    let listed = |name: &str| name.starts_with("listed");
    assert_eq!(
        cases::failures_with(&cases, str::to_owned, listed),
        [
            "listed and passing: passes over Ruby 1.9.2, remove `case listed and passing` from \
             tests/ruby192.allow",
            "not listed and failing: expected Value(\"4\"), got the value 2",
        ]
    );
    // The committed list names none of them.
    assert_eq!(cases::failures(&cases, str::to_owned).len(), 2);
}
