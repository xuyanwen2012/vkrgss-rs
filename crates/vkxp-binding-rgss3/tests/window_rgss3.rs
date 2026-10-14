//! The class `Window` in a game of RGSS3 over Ruby 1.9.2: the scenarios of "Window
//! construction in RGSS3", "Window methods added in RGSS3", "Window tone in RGSS3",
//! "Assignments of a window in RGSS3 are calls of the scene", "Disposal and disposed bitmaps
//! of a window in RGSS3", "Calls at which the original of RGSS3 faults, through the class"
//! and "Method surface of Window in RGSS3" of `rgss-scene-classes`. A file of this package
//! alone, so that it never runs as RGSS1 or RGSS2 over Ruby 1.8.7.
//!
//! The expected values said to be measured are those of the original runtime of VX Ace (DLL
//! versions 3.0.0.1 and 3.0.1.1, 2026-10-13, `ace-v1-window-REPORT.md` section "The class"
//! and rules 7, 9, 10, 14 and 15, kept outside the repository). Where the original ends with
//! a page fault, vkxp goes on.
//!
//! A snippet is one evaluation and disposes the windows and bitmaps it made before it ends,
//! as in `scene.rs`.
#![cfg(not(vkxp_interpreter_absent))]
// The support module of `vkxp-binding` names the binding by that name.
extern crate vkxp_binding_rgss3 as vkxp_binding;

#[path = "../../vkxp-binding/tests/scene_support/mod.rs"]
mod scene_support;

use vkxp_binding::testing;
use vkxp_vfs::Runtime;

/// `inspect` of the value of `source` in a game of RGSS3, or the class and the message of
/// the exception that left it.
fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    scene_support::install();
    match testing::eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// What `eval` returns for a snippet that makes `objects` and disposes them at its end.
/// `try { ... }` is the class of the exception the block raises, `error { ... }` its class
/// and message.
fn eval_with(objects: &str, source: &str) -> String {
    eval(&format!(
        "def try; yield; rescue Exception; $!.class; end; \
         def error; yield; rescue Exception; \"#{{$!.class}}: #{{$!.message}}\"; end; \
         made = []; begin; {objects}; {source}; ensure; made.each {{ |o| o.dispose }}; end"
    ))
}

/// A window made by `Window.new(arguments)` and its contents bitmap, both disposed at the
/// end of the snippet.
fn window(arguments: &str) -> String {
    format!("made << (w = Window.new({arguments})) << w.contents")
}

/// The grey skin of 128 by 128.
const GREY_SKIN: &str = "made << (skin = solid(128, 128, 100, 100, 100))";
/// The grey skin with the block (80, 16, 32, 32) of the four arrows opaque magenta.
const ARROW_SKIN: &str = "made << (skin = solid(128, 128, 100, 100, 100)); \
                          skin.fill_rect(80, 16, 32, 32, Color.new(255, 0, 255))";
/// The green contents of 50 by 50.
const GREEN_CONTENTS: &str = "made << (green = solid(50, 50, 0, 255, 0))";

const GREEN: &str = "[0, 255, 0, 255]";
const BLACK: &str = "[0, 0, 0, 255]";
const NIL_INTEGER: &str = "TypeError: no implicit conversion from nil to integer";

// --- Construction -----------------------------------------------------------------------------

/// Scenario "Without arguments, measured on the original".
#[test]
fn a_window_without_arguments_has_the_defaults_of_rgss3() {
    assert_eq!(
        eval_with(
            &window(""),
            "[w.x, w.y, w.width, w.height, w.z, w.padding, w.padding_bottom, \
              w.back_opacity, w.openness, w.viewport, [w.contents.width, w.contents.height]]"
        ),
        "[0, 0, 0, 0, 100, 8, 8, 192, 255, nil, [1, 1]]"
    );
}

/// Scenario "The other defaults, measured on the original".
#[test]
fn the_other_defaults_are_those_of_the_original() {
    assert_eq!(
        eval_with(
            &window(""),
            "[w.ox, w.oy, w.opacity, w.contents_opacity, w.visible, w.active, w.pause, \
              w.windowskin, w.cursor_rect.to_s, w.arrows_visible, w.open?, w.close?]"
        ),
        r#"[0, 0, 255, 255, true, true, false, nil, "(0, 0, 0, 0)", true, true, false]"#
    );
}

/// Scenario "One to four arguments, measured on the original".
#[test]
fn up_to_four_integers_place_the_window() {
    let cases = [
        ("1", "[1, 0, 0, 0]"),
        ("1, 2", "[1, 2, 0, 0]"),
        ("1, 2, 3", "[1, 2, 3, 0]"),
        ("10, 20, 30, 40", "[10, 20, 30, 40]"),
        ("10.7, 20.2, 30.9, 40.5", "[10, 20, 30, 40]"),
        ("-10, -20, -30, -40", "[-10, -20, -30, -40]"),
    ];
    for (arguments, place) in cases {
        assert_eq!(
            eval_with(
                &window(arguments),
                "[[w.x, w.y, w.width, w.height], w.z, [w.contents.width, w.contents.height]]"
            ),
            format!("[{place}, 100, [1, 1]]"),
            "Window.new({arguments})"
        );
    }
}

/// Scenario "No viewport argument, measured on the original". No window is left behind by
/// a refused construction.
#[test]
fn the_constructor_takes_no_viewport() {
    assert_eq!(
        eval_with(
            "made << (v = Viewport.new(0, 0, 10, 10)); before = scene_count(:window)",
            "[error { Window.new(v) }, error { Window.new(nil, 0, 0, 0) }, \
              error { Window.new(10, 20, 30, 40, 5) }, error { Window.new(v, 1, 2, 3, 4) }, \
              error { Window.new(1, 2, 3, 4, v) }, scene_count(:window) - before]"
        ),
        format!(
            "[\"TypeError: can't convert Viewport into Integer\", \"{NIL_INTEGER}\", \
             \"{count}\", \"{count}\", \"{count}\", 0]",
            count = "ArgumentError: wrong number of arguments (4 for 5)"
        )
    );
}

/// Scenario "Contents, measured on the original".
#[test]
fn every_window_has_contents_of_its_own() {
    assert_eq!(
        eval_with(
            "made << (a = Window.new) << a.contents << (b = Window.new(5, 6, 100, 80)) \
             << b.contents",
            "[a.contents.equal?(b.contents), a.contents.equal?(a.contents), b.contents.width, \
              b.contents.height, a.contents.get_pixel(0, 0).to_s, a.contents.disposed?]"
        ),
        r#"[false, true, 1, 1, "(0.000000, 0.000000, 0.000000, 0.000000)", false]"#
    );
}

// --- Methods added in RGSS3 -------------------------------------------------------------------

/// Scenario "Paddings, measured on the original".
#[test]
fn the_padding_sets_both_paddings() {
    assert_eq!(
        eval_with(
            &window(""),
            "both = -> { [w.padding, w.padding_bottom] }; \
             [(w.padding = 20; both.()), (w.padding_bottom = 5; both.()), \
              (w.padding = 7; both.()), (w.padding_bottom = 30; w.padding = 7; both.())]"
        ),
        "[[20, 20], [20, 5], [7, 7], [7, 7]]"
    );
}

/// Scenario "Values of the paddings, measured on the original".
#[test]
fn the_paddings_are_truncated_and_not_limited() {
    for property in ["padding", "padding_bottom"] {
        assert_eq!(
            eval_with(
                &window(""),
                &format!(
                    "[[-5, 100000, 10.7, -10.7].map {{ |v| w.{property} = v; w.{property} }}, \
                      error {{ w.{property} = nil }}]"
                )
            ),
            format!("[[-5, 100000, 10, -10], \"{NIL_INTEGER}\"]"),
            "{property}"
        );
    }
}

/// Scenario "Arrows flag, measured on the original".
#[test]
fn the_arrows_flag_is_ruby_truth() {
    assert_eq!(
        eval_with(
            &window(""),
            "[nil, 0, 1, 'x'].map { |v| w.arrows_visible = v; w.arrows_visible }"
        ),
        "[false, true, true, true]"
    );
}

/// Scenario "Move, measured on the original".
#[test]
fn move_assigns_the_four_and_returns_the_window() {
    assert_eq!(
        eval_with(
            &window(""),
            "place = ->(*a) { [w.move(*a).equal?(w), w.x, w.y, w.width, w.height] }; \
             [place.(1, 2, 3, 4), place.(-1, -2, -3, -4), place.(1.9, 2.9, 3.9, 4.9), \
              error { w.move(1, 2, 3) }, error { w.move(nil, 2, 3, 4) }]"
        ),
        format!(
            "[[true, 1, 2, 3, 4], [true, -1, -2, -3, -4], [true, 1, 2, 3, 4], \
             \"ArgumentError: wrong number of arguments(3 for 4)\", \"{NIL_INTEGER}\"]"
        )
    );
}

/// Scenario "Open and closed, measured on the original".
#[test]
fn open_and_closed_are_the_ends_of_the_openness() {
    assert_eq!(
        eval_with(
            &window(""),
            "[255, 254, 128, 1, 0].map { |v| w.openness = v; [w.open?, w.close?] }"
        ),
        "[[true, false], [false, false], [false, false], [false, false], [false, true]]"
    );
}

/// Scenario "Paddings move the contents".
#[test]
fn the_paddings_move_and_cut_the_contents() {
    assert_eq!(
        eval_with(
            &format!(
                "{GREEN_CONTENTS}; {}; w.contents = green",
                window("0, 0, 100, 100")
            ),
            "first = [pixel(8, 8), pixel(7, 7)]; w.padding = 20; \
             second = [pixel(8, 8), pixel(20, 20), pixel(30, 60)]; w.padding_bottom = 50; \
             [first, second, [pixel(30, 49), pixel(30, 50)], w.padding]"
        ),
        format!("[[{GREEN}, {BLACK}], [{BLACK}, {GREEN}, {GREEN}], [{GREEN}, {BLACK}], 20]")
    );
}

/// Scenario "Arrows hidden and shown": the down arrow of a window of 100 by 100 lies at
/// (42, 88).
#[test]
fn the_arrows_flag_hides_and_shows_the_arrows() {
    assert_eq!(
        eval_with(
            &format!(
                "{ARROW_SKIN}; {}; made << (c = Bitmap.new(200, 200)); w.windowskin = skin; \
                 w.contents = c",
                window("0, 0, 100, 100")
            ),
            "shown = pixel(50, 92); w.arrows_visible = false; hidden = pixel(50, 92); \
             w.arrows_visible = true; [hidden != shown, pixel(50, 92) == shown]"
        ),
        "[true, true]"
    );
}

/// Scenario "Move reaches the scene".
#[test]
fn move_reaches_the_scene() {
    assert_eq!(
        eval_with(
            &format!(
                "{GREY_SKIN}; {}; w.windowskin = skin; w.back_opacity = 255",
                window("0, 0, 20, 20")
            ),
            &format!(
                "before = [pixel(70, 70) == {BLACK}, pixel(10, 10) == {BLACK}]; \
                 w.move(40, 30, 68, 68); [before, [pixel(70, 70) == {BLACK}, \
                 pixel(10, 10) == {BLACK}]]"
            )
        ),
        "[[true, false], [false, true]]"
    );
}
