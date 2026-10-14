//! The scenarios of `rgss-values` and `rgss-bitmap`, written in Ruby and evaluated in the
//! embedded interpreter. The snippets are in `cases`.

mod cases;

use cases::Case;
use vkrgss_binding::testing;

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
fn bitmap_creation_from_a_size() {
    check(cases::BITMAP_FROM_SIZE);
}

#[test]
fn bitmap_creation_from_a_file() {
    check(cases::BITMAP_FROM_FILE);
}

#[test]
fn bitmap_disposal() {
    check(cases::BITMAP_DISPOSAL);
}

#[test]
fn bitmap_drawing() {
    check(cases::BITMAP_DRAWING);
}

#[test]
fn bitmap_text() {
    check(cases::BITMAP_TEXT);
}

#[test]
fn bitmap_copying() {
    check(cases::BITMAP_COPYING);
}

#[test]
fn unreachable_bitmaps_are_freed() {
    assert_eq!(
        testing::eval(cases::UNREACHABLE_BITMAPS).as_deref(),
        Ok("nil")
    );
    // The other tests of this binary hold a few bitmaps, and the conservative collector may
    // keep a few of the ten thousand alive.
    let remaining = testing::bitmap_count();
    assert!(remaining < 500, "{remaining} bitmaps are left in the arena");
}

#[test]
fn every_group_is_tested_here() {
    // One test function per group above: a group added to the list must get one too.
    assert_eq!(cases::GROUPS.len(), 13);
}

/// Scenarios "XP and VX" of "Color and Tone of RGSS3" and "Rect of RGSS3", for a game of
/// RGSS1: the forms that RGSS3 adds raise `ArgumentError`, and `Rect#initialize` keeps its
/// arity of 4.
#[test]
fn the_forms_of_rgss3_are_not_taken() {
    for source in [
        "Color.new",
        "Tone.new",
        "Color.new(1, 2, 3).set(Color.new(1, 2, 3))",
        "Rect.new",
        "Rect.new(1, 2, 3, 4).set(Rect.new(1, 2, 3, 4))",
    ] {
        let error = testing::eval(source).expect_err(source);
        assert_eq!(error.class_name, "ArgumentError", "{source}");
    }
    assert_eq!(
        testing::eval("Rect.instance_method(:initialize).arity").as_deref(),
        Ok("4")
    );
}

/// Scenario "XP" of "Font and Bitmap of RGSS1 and RGSS2 are unchanged by RGSS3": a game of
/// RGSS1 has no outline.
#[test]
fn the_font_of_rgss3_does_not_reach_rgss1() {
    assert_eq!(
        testing::eval("[Font.new.respond_to?(:outline), Font.respond_to?(:default_outline)]")
            .as_deref(),
        Ok("[false, false]")
    );
}
