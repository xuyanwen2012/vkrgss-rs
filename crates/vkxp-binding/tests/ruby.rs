//! The scenarios of `rgss-values` and `rgss-bitmap`, written in Ruby and evaluated in the
//! embedded interpreter. The snippets are in `cases`.

mod cases;

use cases::Case;

fn check(cases: &[Case]) {
    let wrong = cases::failures(cases, str::to_owned);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn value_method_surface() {
    check(cases::VALUE_SURFACE);
}

#[test]
fn color_and_tone() {
    check(cases::COLOR_AND_TONE);
}

#[test]
fn rect() {
    check(cases::RECT);
}

#[test]
fn table() {
    check(cases::TABLE);
}

#[test]
fn marshal() {
    check(cases::MARSHAL);
}

#[test]
fn font() {
    check(cases::FONT);
}

#[test]
fn copies_of_values() {
    check(cases::VALUE_COPIES);
}

#[test]
fn every_group_is_tested_here() {
    // One test function per group above: a group added to the list must get one too.
    assert_eq!(cases::GROUPS.len(), 7);
}
