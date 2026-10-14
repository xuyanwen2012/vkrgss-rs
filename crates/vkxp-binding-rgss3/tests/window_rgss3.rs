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
/// The grey skin with the cursor block (64, 64, 32, 32) opaque red.
const CURSOR_SKIN: &str = "made << (skin = solid(128, 128, 100, 100, 100)); \
                           skin.fill_rect(64, 64, 32, 32, Color.new(255, 0, 0))";
/// The grey skin with the block (80, 16, 32, 32) of the four arrows opaque magenta.
const ARROW_SKIN: &str = "made << (skin = solid(128, 128, 100, 100, 100)); \
                          skin.fill_rect(80, 16, 32, 32, Color.new(255, 0, 255))";
/// The green contents of 50 by 50.
const GREEN_CONTENTS: &str = "made << (green = solid(50, 50, 0, 255, 0))";

const RED: &str = "[255, 0, 0, 255]";
const GREEN: &str = "[0, 255, 0, 255]";
const BLACK: &str = "[0, 0, 0, 255]";
const DISPOSED: &str = "RGSSError: disposed window";
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
              w.windowskin, w.cursor_rect.to_s, w.arrows_visible, w.tone.to_s, w.open?, \
              w.close?]"
        ),
        concat!(
            r#"[0, 0, 255, 255, true, true, false, nil, "(0, 0, 0, 0)", true, "#,
            r#""(0.000000, 0.000000, 0.000000, 0.000000)", true, false]"#
        )
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

// --- Tone -------------------------------------------------------------------------------------

/// Scenario "The object, measured on the original".
#[test]
fn the_tone_is_an_object_of_the_window() {
    assert_eq!(
        eval_with(
            &window(""),
            "given = ->(t) { w.tone = t; r = w.tone; [r.red, r.green, r.blue, r.gray, \
             r.equal?(t)] }; [w.tone.equal?(w.tone), given.(Tone.new(1, 2, 3, 4)), \
             given.(Tone.new(300, -300, 255.5, 300))]"
        ),
        "[true, [1.0, 2.0, 3.0, 4.0, false], [255.0, -255.0, 255.0, 255.0, false]]"
    );
}

/// Scenario "Not a tone, measured on the original".
#[test]
fn the_tone_takes_a_tone_only() {
    assert_eq!(
        eval_with(
            &window(""),
            "w.tone = Tone.new(1, 2, 3, 4); before = w.tone.to_s; \
             [error { w.tone = nil }, error { w.tone = 5 }, \
              error { w.tone = Color.new(1, 2, 3, 4) }, w.tone.to_s == before]"
        ),
        "[\"TypeError: can't convert NilClass into Tone\", \
         \"TypeError: can't convert Fixnum into Tone\", \
         \"TypeError: can't convert Color into Tone\", true]"
    );
}

/// Scenario "Changed through its own writer, measured on the original".
#[test]
fn the_tone_changes_through_its_own_writer() {
    assert_eq!(
        eval_with(&window(""), "t = w.tone; t.red = 50; w.tone.red"),
        "50.0"
    );
}

/// Scenario "Seen in the frame": the background of the grey skin, toned.
#[test]
fn the_tone_is_seen_on_the_background() {
    assert_eq!(
        eval_with(
            &format!(
                "{GREY_SKIN}; {}; w.windowskin = skin; w.back_opacity = 255",
                window("0, 0, 68, 68")
            ),
            "first = pixel(30, 40); w.tone.set(Tone.new(50, -30, 0, 0)); second = pixel(30, 40); \
             w.tone.red = -255; third = pixel(30, 40); w.tone = Tone.new; \
             [first, second, third, pixel(30, 40) == first]"
        ),
        "[[100, 100, 100, 255], [150, 70, 100, 255], [0, 70, 100, 255], true]"
    );
}

/// Scenario "The given object stays unconnected".
#[test]
fn the_given_tone_stays_unconnected() {
    assert_eq!(
        eval_with(
            &window(""),
            "t = Tone.new(1, 2, 3, 4); w.tone = t; t.red = 99; w.tone.red"
        ),
        "1.0"
    );
}

// --- Assignments as calls ---------------------------------------------------------------------

/// A window of 40 by 40 at (0, 0) with the cursor skin and a cursor rectangle of 100 by 100,
/// placed and cut to the inner area of that size; `cursor` is the colour of the cursor at
/// (20, 20). The window is then grown to 120 by 120, which does not place the cursor.
fn grown() -> String {
    format!(
        "{CURSOR_SKIN}; {}; w.windowskin = skin; w.cursor_rect = Rect.new(0, 0, 100, 100); \
         cursor = pixel(20, 20); w.width = 120; w.height = 120",
        window("0, 0, 40, 40")
    )
}

/// Scenario "The cursor is placed through the Rect, measured on the original".
#[test]
fn the_cursor_is_placed_through_its_rect() {
    assert_eq!(
        eval_with(
            &grown(),
            "grown = pixel(50, 50) == cursor; w.cursor_rect.x = 0; \
             [cursor, grown, pixel(50, 50) == cursor]"
        ),
        format!("[{RED}, false, true]")
    );
}

/// Scenario "Every way of changing the Rect, measured on the original".
#[test]
fn every_way_of_changing_the_rect_places_the_cursor() {
    for call in [
        "w.cursor_rect.set(0, 0, 100, 100)",
        "w.cursor_rect.set(Rect.new(0, 0, 100, 100))",
        "w.cursor_rect = Rect.new(0, 0, 100, 100)",
        "w.ox = 0",
        "w.padding = 8",
    ] {
        assert_eq!(
            eval_with(&grown(), &format!("{call}; pixel(50, 50) == cursor")),
            "true",
            "{call}"
        );
    }
}

/// Scenario "Calls that do not place it, measured on the original".
#[test]
fn other_calls_do_not_place_the_cursor() {
    for call in ["w.move(0, 0, 120, 120)", "w.x = 0", "w.update", "w.z = 100"] {
        assert_eq!(
            eval_with(&grown(), &format!("{call}; pixel(50, 50) == cursor")),
            "false",
            "{call}"
        );
    }
}

/// Scenario "The cursor rectangle is the window's own, measured on the original".
#[test]
fn the_cursor_rectangle_is_the_windows_own() {
    assert_eq!(
        eval_with(
            &window(""),
            "r = Rect.new(1, 2, 3, 4); w.cursor_rect = r; c = w.cursor_rect; \
             [c.x, c.y, c.width, c.height, c.equal?(r), c.equal?(w.cursor_rect), \
              try { w.cursor_rect = nil }, try { w.cursor_rect = 5 }]"
        ),
        "[1, 2, 3, 4, false, true, TypeError, TypeError]"
    );
}

/// Scenario "Limits of the opacities, measured on the original".
#[test]
fn the_opacities_are_truncated_and_clamped() {
    for property in ["opacity", "back_opacity", "contents_opacity", "openness"] {
        assert_eq!(
            eval_with(
                &window(""),
                &format!(
                    "[300, 256, -5, 100.7, 254.9, -0.5].map {{ |v| w.{property} = v; \
                     w.{property} }}"
                )
            ),
            "[255, 255, 0, 100, 254, 0]",
            "{property}"
        );
    }
}

/// Scenario "Positions and flags, measured on the original".
#[test]
fn the_positions_and_the_flags_take_values_as_in_rgss2() {
    for property in ["x", "y", "z", "width", "height", "ox", "oy"] {
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
    for property in ["visible", "active", "pause"] {
        assert_eq!(
            eval_with(
                &window(""),
                &format!("[nil, 0, 'x'].map {{ |v| w.{property} = v; w.{property} }}")
            ),
            "[false, true, true]",
            "{property}"
        );
    }
}

/// Scenario "Bitmaps and update, measured on the original".
#[test]
fn the_bitmaps_are_kept_and_update_returns_nil() {
    assert_eq!(
        eval_with(
            &format!("{}; made << (b = Bitmap.new(8, 8))", window("")),
            "[(w.windowskin = b; w.contents = b; [w.windowskin.equal?(b), w.contents.equal?(b)]), \
              (w.windowskin = nil; w.contents = nil; [w.windowskin, w.contents]), \
              try { w.contents = 5 }, try { w.windowskin = 5 }, w.update]"
        ),
        "[[true, true], [nil, nil], TypeError, TypeError, nil]"
    );
}

/// Scenario "Viewport, measured on the original".
#[test]
fn the_viewport_is_assigned_as_in_rgss2() {
    assert_eq!(
        eval_with(
            &format!("{}; made << (v = Viewport.new(0, 0, 10, 10))", window("")),
            "w.viewport = v; a = w.viewport.equal?(v); w.viewport = nil; \
             [[a, w.viewport], error { w.viewport = 5 }]"
        ),
        "[[true, nil], \"TypeError: can't convert Fixnum into Viewport\"]"
    );
}

// --- Disposal and disposed bitmaps ------------------------------------------------------------

/// Scenario "A disposed window, measured on the original".
#[test]
fn a_disposed_window_raises_and_leaves_its_bitmaps() {
    let calls = [
        "w.x",
        "w.width",
        "w.contents",
        "w.windowskin",
        "w.tone",
        "w.cursor_rect",
        "w.open?",
        "w.close?",
        "w.update",
        "w.padding",
        "w.viewport",
        "w.visible",
        "w.x = 1",
        "w.move(1, 2, 3, 4)",
    ];
    let errors: Vec<String> = calls
        .iter()
        .map(|call| format!("error {{ {call} }}"))
        .collect();
    assert_eq!(
        eval_with(
            &format!("{}; made << (c = Bitmap.new(10, 10))", window("")),
            &format!(
                "w.contents = c; w.dispose; [[c.disposed?, w.disposed?, w.dispose], [{}]]",
                errors.join(", ")
            )
        ),
        format!(
            "[[false, true, nil], [{}]]",
            vec![format!("\"{DISPOSED}\""); calls.len()].join(", ")
        )
    );
}

/// Scenario "A disposed skin, measured on the original".
#[test]
fn a_disposed_skin_is_drawn_as_none() {
    assert_eq!(
        eval_with(
            &format!(
                "{GREY_SKIN}; {}; w.windowskin = skin; w.back_opacity = 255",
                window("0, 0, 100, 100")
            ),
            &format!(
                "first = pixel(50, 50) != {BLACK}; skin.dispose; second = pixel(50, 50); \
                 w.update; w.width = 100; w.ox = 0; w.cursor_rect = Rect.new(0, 0, 8, 8); \
                 w.contents_opacity = 100; same = w.windowskin.equal?(skin); \
                 [first, second, pixel(50, 50), same]"
            )
        ),
        format!("[true, {BLACK}, {BLACK}, true]")
    );
}

/// Scenario "Disposed contents, measured on the original".
#[test]
fn disposed_contents_are_drawn_as_none() {
    assert_eq!(
        eval_with(
            &format!(
                "{GREEN_CONTENTS}; {}; w.contents = green",
                window("0, 0, 100, 100")
            ),
            "first = pixel(10, 10); green.dispose; w.update; w.contents_opacity = 100; \
             [first, pixel(10, 10)]"
        ),
        format!("[{GREEN}, {BLACK}]")
    );
}

// --- Calls at which the original faults -------------------------------------------------------

/// Scenario "No skin was ever assigned": the original ends at the first assignment here.
#[test]
fn the_cursor_is_placed_in_a_window_that_never_had_a_skin() {
    assert_eq!(
        eval_with(
            &window("0, 0, 96, 96"),
            "w.padding = 8; w.padding_bottom = 4; w.ox = 0; w.oy = 0; \
             w.cursor_rect = Rect.new(0, 0, 0, 0); w.cursor_rect.set(0, 0, 0, 0); \
             w.cursor_rect.x = 0; w.cursor_rect.empty; \
             [w.padding, w.padding_bottom, w.cursor_rect.to_s, pixel(48, 48)]"
        ),
        format!("[8, 4, \"(0, 0, 0, 0)\", {BLACK}]")
    );
}

/// Scenario "A small skin and contents".
#[test]
fn contents_follow_a_small_skin() {
    assert_eq!(
        eval_with(
            &window(""),
            "made << (w.windowskin = Bitmap.new(8, 8)) << (w.contents = Bitmap.new(8, 8)); \
             w.contents.width"
        ),
        "8"
    );
}

/// Scenario "Sizes": nothing of a window without a skin needs a bitmap of its size.
#[test]
fn a_window_and_a_cursor_of_any_size_are_taken() {
    assert_eq!(
        eval_with(
            &window("0, 0, 100000, 100000"),
            "w.cursor_rect = Rect.new(0, 0, 100000, 100000); \
             [w.width, w.cursor_rect.width, pixel(0, 0).size]"
        ),
        "[100000, 100000, 4]"
    );
}

// --- Method surface ---------------------------------------------------------------------------

/// Scenarios "Every listed method exists" and "Nothing beside the list", in part: the
/// arities named there and no `stretch`. `arity_rgss3.rs` checks the whole list.
#[test]
fn the_class_has_the_methods_of_rgss3_and_no_stretch() {
    assert_eq!(
        eval(
            "[:initialize, :move, :open?, :close?, :padding, :padding_bottom, \
              :arrows_visible, :tone, :viewport].map { |m| Window.instance_method(m).arity } \
             << [Window.method_defined?(:stretch), Window.method_defined?(:stretch=)]"
        ),
        "[-1, 4, 0, 0, 0, 0, 0, 0, 0, [false, false]]"
    );
}

/// Scenario "No copies".
#[test]
fn a_window_is_not_copied() {
    assert_eq!(
        eval_with(&window(""), "[try { w.clone }, try { w.dup }]"),
        "[TypeError, TypeError]"
    );
}
