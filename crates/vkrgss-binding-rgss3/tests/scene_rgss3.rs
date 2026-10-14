//! `Sprite`, `Plane` and `Viewport` in a game of RGSS3 over Ruby 1.9.2: the scenarios of
//! "Viewport disposal" for RGSS3, "The scene of a game of RGSS3", "Viewport of RGSS3",
//! "Plane viewport of RGSS3" and "Constructors and object arguments of RGSS3" of
//! `rgss-scene-classes`. A file of this package alone, so that it never runs as RGSS1 or
//! RGSS2 over Ruby 1.8.7.
//!
//! The expected values are those measured on the original runtime of VX Ace (DLL versions
//! 3.0.0.1 and 3.0.1.1, 2026-10-13, `ace-v1-sprite-REPORT.md` sections 1 to 5 with its
//! exploration log, `ace-v1-window-REPORT.md` and `ace-v1-tilemap-REPORT.md`, kept outside
//! the repository), but where a scenario says vkrgss decided without a measurement: what an
//! element of a disposed viewport draws, and a disposed viewport given to `viewport=`.
//!
//! A snippet is one evaluation and disposes what it drew before it ends, as in `scene.rs`;
//! one that resizes the screen sets it back to 544 by 416.
#![cfg(not(vkrgss_interpreter_absent))]
// The support module of `vkrgss-binding` names the binding by that name.
extern crate vkrgss_binding_rgss3 as vkrgss_binding;

#[path = "../../vkrgss-binding/tests/scene_support/mod.rs"]
mod scene_support;

use vkrgss_binding::testing;
use vkrgss_binding::testing::host::{self, Script};
use vkrgss_vfs::Runtime;

/// Ruby that `eval_with` defines: `try { ... }` is the class of the exception the block
/// raises, `error { ... }` its class and message.
const HELPERS: &str = "def try; yield; rescue Exception; $!.class; end; \
     def error; yield; rescue Exception; \"#{$!.class}: #{$!.message}\"; end";

/// Starts the harness as a game of RGSS3.
fn start() {
    testing::start_as(Runtime::Rgss3);
    scene_support::install();
}

/// `inspect` of the value of `source` in a game of RGSS3, or the class and the message of
/// the exception that left it.
fn eval(source: &str) -> String {
    start();
    match testing::eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// What `eval` returns for a snippet that makes `objects` and disposes them at its end.
fn eval_with(objects: &str, source: &str) -> String {
    eval(&format!(
        "{HELPERS}; made = []; begin; {objects}; {source}; \
         ensure; made.each {{ |o| o.dispose }}; end"
    ))
}

const RED: &str = "[255, 0, 0, 255]";
const BLACK: &str = "[0, 0, 0, 255]";

/// The four elements of the scenario "RGSS3 keeps the elements" in a viewport
/// (0, 0, 50, 50): a sprite `s`, a plane `p`, a window `w` and a tilemap `t`.
const FOUR: &str = "made << (v = Viewport.new(0, 0, 50, 50)) << (s = Sprite.new(v)) \
     << (p = Plane.new(v)) << (w = Window.new) << (t = Tilemap.new(v)); w.viewport = v";

// Requirement "Viewport disposal".

/// Scenario "RGSS3 keeps the elements, measured on the original".
#[test]
fn a_disposed_viewport_keeps_its_elements() {
    start();
    let (value, record) = host::play(
        &format!(
            "made = []; begin; {FOUR}; v.dispose; Graphics.update; s.x = 3; \
             [s.disposed?, p.disposed?, w.disposed?, t.disposed?, s.x, s.viewport.equal?(v), \
              p.viewport.equal?(v), w.viewport.equal?(v), t.viewport.equal?(v)]; \
             ensure; made.each {{ |o| o.dispose }}; end"
        ),
        Script::new(),
    );
    assert_eq!(
        value.as_deref(),
        Ok("[false, false, false, false, 3, true, true, true, true]")
    );
    assert_eq!(record.ticks, 1);
}

/// Scenario "Usable after the viewport, measured on the original".
#[test]
fn the_elements_of_a_disposed_viewport_are_usable() {
    assert_eq!(
        eval_with(
            FOUR,
            "v.dispose; w.ox = 4; t.ox = 4; s.x = 3; p.z = 2; s.update; w.update; t.update; \
             read = [w.ox, t.ox, s.x, p.z]; \
             disposed = [s, p, w, t].map { |o| [o.dispose, o.disposed?] }; [read, disposed]"
        ),
        "[[4, 4, 3, 2], [[nil, true], [nil, true], [nil, true], [nil, true]]]"
    );
}

/// Scenarios "Drawn nowhere" and "Moved out of the disposed viewport".
#[test]
fn a_sprite_of_a_disposed_viewport_is_drawn_nowhere_until_it_is_moved_out() {
    assert_eq!(
        eval_with(
            "made << (b = solid(8, 8, 255, 0, 0)) << (v = Viewport.new(0, 0, 50, 50)) \
             << (s = Sprite.new(v)); s.bitmap = b; s.x = 10; s.y = 10",
            "shown = pixel(10, 10); v.dispose; gone = [pixel(10, 10), s.disposed?]; \
             s.viewport = nil; [shown, gone, pixel(10, 10), s.viewport]"
        ),
        format!("[{RED}, [{BLACK}, false], {RED}, nil]")
    );
}

// Requirement "The scene of a game of RGSS3".

/// A bitmap `b` of 16 by 8 whose texel column c is (c, 255 - c, 128).
const COLUMNS: &str = "made << (b = Bitmap.new(16, 8)); \
     16.times { |c| b.fill_rect(c, 0, 1, 8, Color.new(c, 255 - c, 128)) }";

/// Scenario "A turned sprite in a viewport, measured rule".
#[test]
fn a_turned_sprite_in_a_viewport_is_drawn_by_the_rule_of_rgss3() {
    assert_eq!(
        eval_with(
            &format!(
                "{COLUMNS}; made << (v = Viewport.new(100, 50, 60, 40)) << (s = Sprite.new(v)); \
                 v.ox = 10; v.oy = 4; s.bitmap = b; s.x = 30; s.y = 20; s.angle = 180"
            ),
            "[pixel(105, 58)[0], pixel(119, 65)[0], \
              [[104, 58], [120, 58], [105, 57], [105, 66]].map { |x, y| pixel(x, y) }.uniq]"
        ),
        format!("[15, 1, [{BLACK}]]")
    );
}

/// Scenario "The order of RGSS2".
#[test]
fn the_y_of_a_sprite_orders_as_in_rgss2() {
    assert_eq!(
        eval_with(
            "made << (r = solid(20, 20, 255, 0, 0)) << (g = solid(20, 20, 0, 255, 0)) \
             << (a = Sprite.new) << (c = Sprite.new); a.bitmap = r; a.y = 9; c.bitmap = g",
            "pixel(5, 12)"
        ),
        RED
    );
}

// Requirement "Viewport of RGSS3".

/// Scenario "Without arguments, measured on the original".
#[test]
fn a_viewport_without_arguments_covers_the_screen_of_the_moment() {
    assert_eq!(
        eval(
            "begin; \
               rect = lambda { v = Viewport.new; r = v.rect; v.dispose; \
                               [r.x, r.y, r.width, r.height] }; \
               all = [rect.call]; \
               [[640, 480], [320, 240], [200, 300], [1, 1]].each { |w, h| \
                 Graphics.resize_screen(w, h); all << rect.call }; \
               v = Viewport.new; all << [v.z, v.ox, v.oy, v.visible]; v.dispose; all; \
             ensure; Graphics.resize_screen(544, 416); end"
        ),
        "[[0, 0, 544, 416], [0, 0, 640, 480], [0, 0, 320, 240], [0, 0, 200, 300], \
         [0, 0, 1, 1], [0, 0, 0, true]]"
    );
}

/// Scenario "Forms, measured on the original".
#[test]
fn a_viewport_copies_a_rect_and_converts_four_values() {
    assert_eq!(
        eval_with(
            "made << (a = Viewport.new(given = Rect.new(3, 4, 5, 6))) \
             << (f = Viewport.new(1.9, 2.9, 3.9, 4.9)) << (n = Viewport.new(-5, -6, -7, -8))",
            "read = lambda { |v| r = v.rect; [r.x, r.y, r.width, r.height] }; \
             all = [a, f, n].map(&read); given.set(9, 9, 9, 9); \
             all + [read.call(a), a.rect.equal?(given)]"
        ),
        "[[3, 4, 5, 6], [1, 2, 3, 4], [-5, -6, -7, -8], [3, 4, 5, 6], false]"
    );
}

/// Scenario "Wrong arguments, measured on the original".
#[test]
fn a_viewport_refuses_other_counts_with_the_measured_message() {
    for (source, expected) in [
        ("Viewport.new(1)", "(4 for 1)"),
        ("Viewport.new(nil)", "(4 for 1)"),
        ("Viewport.new('a')", "(4 for 1)"),
        ("Viewport.new(o)", "(4 for 1)"),
        ("Viewport.new(1, 2)", "(4 for 2)"),
        ("Viewport.new(1, 2, 3)", "(4 for 3)"),
        ("Viewport.new(1, 2, 3, 4, 5)", "(4 for 5)"),
        ("Viewport.new(Rect.new(0, 0, 1, 1), 1)", "(1 for 2)"),
    ] {
        assert_eq!(
            eval_with(
                "made << (o = Viewport.new); count = scene_count(:viewport)",
                &format!("[error {{ {source} }}, scene_count(:viewport) == count]")
            ),
            format!("[\"ArgumentError: wrong number of arguments {expected}\", true]"),
            "{source}"
        );
    }
    assert_eq!(
        eval("Viewport.new(nil, 2, 3, 4)"),
        "TypeError: no implicit conversion from nil to integer"
    );
}

/// The colour (red, green, blue, alpha) of the pixel (x, y) of the bitmap `s` as Integers.
const READ: &str = "read = lambda { |s, x, y| c = s.get_pixel(x, y); \
     [c.red, c.green, c.blue, c.alpha].map(&:to_i) }";

/// Scenario "Kept across a resize, measured on the original".
#[test]
fn a_viewport_of_the_screen_keeps_its_rectangle_across_a_resize() {
    assert_eq!(
        eval_with(
            "made << (white = solid(8, 8, 255, 255, 255)) << (v = Viewport.new) \
             << (inside = Plane.new(v)) << (outside = Plane.new); \
             inside.bitmap = white; outside.bitmap = white; outside.visible = false",
            &format!(
                "{READ}; begin; Graphics.resize_screen(640, 480); \
                   r = v.rect; rect = [r.x, r.y, r.width, r.height]; \
                   made << (first = Graphics.snap_to_bitmap); \
                   outside.visible = true; made << (second = Graphics.snap_to_bitmap); \
                   [rect, [[543, 415], [544, 0], [0, 416]].map {{ |x, y| read.call(first, x, y) }}, \
                    read.call(second, 639, 479)]; \
                 ensure; Graphics.resize_screen(544, 416); end"
            )
        ),
        "[[0, 0, 544, 416], [[255, 255, 255, 255], [0, 0, 0, 255], [0, 0, 0, 255]], \
         [255, 255, 255, 255]]"
    );
}

/// Scenario "Disposed viewport, measured on the original".
#[test]
fn a_disposed_viewport_raises_but_for_dispose() {
    assert_eq!(
        eval_with(
            "v = Viewport.new(0, 0, 8, 8); v.dispose",
            "[error { v.rect }, error { v.z }, error { v.z = 1 }, error { v.update }, \
              error { v.flash(Color.new(1, 2, 3), 2) }, v.dispose, try { v.disposed? }].uniq"
        ),
        "[\"RGSSError: disposed viewport\", nil, NoMethodError]"
    );
}

/// Scenario "Properties, measured on the original".
#[test]
fn the_plain_properties_of_a_viewport_are_converted() {
    assert_eq!(
        eval(
            "[[:z=, 5.9, :z], [:z=, -5.9, :z], [:ox=, 5.9, :ox], [:oy=, 5.9, :oy], \
              [:visible=, nil, :visible], [:visible=, 5, :visible]].map { |writer, value, reader| \
               v = Viewport.new; v.send(writer, value); r = v.send(reader); v.dispose; r }"
        ),
        "[5, -5, 5, 5, false, true]"
    );
}

// Requirement "Plane viewport of RGSS3".

/// Scenario "Reader and writer, measured on the original".
#[test]
fn a_plane_has_a_viewport_reader_and_writer() {
    assert_eq!(
        eval_with(
            "made << (p = Plane.new) << (v = Viewport.new(0, 0, 9, 9))",
            "[Plane.instance_method(:viewport).arity, Plane.instance_method(:viewport=).arity, \
              p.viewport, (p.viewport = v; p.viewport.equal?(v)), (p.viewport = nil; p.viewport)]"
        ),
        "[0, 1, nil, true, nil]"
    );
}

/// Scenario "Wrong arguments, measured on the original" of the plane.
#[test]
fn a_plane_refuses_another_object_and_an_argument_of_the_reader() {
    assert_eq!(
        eval_with(
            "made << (p = Plane.new)",
            "[error { p.viewport = 5 }, try { p.viewport(nil) }]"
        ),
        "[\"TypeError: can't convert Fixnum into Viewport\", ArgumentError]"
    );
}

/// Scenario "Seen by the scene, measured on the original" of the plane.
#[test]
fn a_moved_plane_is_drawn_in_its_new_viewport() {
    assert_eq!(
        eval_with(
            "made << (b = Bitmap.new(24, 16)) << (v = Viewport.new(4, 4, 60, 50)) \
             << (u = Viewport.new(70, 20, 80, 90)) << (p = Plane.new(v)); \
             24.times { |c| b.fill_rect(c, 0, 1, 16, Color.new(c, 255 - c, 128)) }; \
             p.bitmap = b; p.ox = 5; u.ox = 7; u.oy = 3",
            "first = [pixel(4, 4)[0], pixel(64, 54)]; p.viewport = u; \
             [first, [pixel(4, 4), pixel(70, 20)[0], pixel(149, 109)[0], pixel(150, 110)]]"
        ),
        format!("[[5, {BLACK}], [{BLACK}, 12, 19, {BLACK}]]")
    );
}

/// Scenario "Disposed viewport" of the plane: vkrgss raises where the original ends.
#[test]
fn a_disposed_viewport_is_refused_by_a_plane_and_a_sprite() {
    assert_eq!(
        eval_with(
            "made << (keep = Viewport.new(0, 0, 8, 8)) << (p = Plane.new(keep)) \
             << (s = Sprite.new(keep)); v = Viewport.new(0, 0, 8, 8); v.dispose",
            "[error { p.viewport = v }, error { s.viewport = v }, \
              p.viewport.equal?(keep), s.viewport.equal?(keep)]"
        ),
        "[\"RGSSError: disposed viewport\", \"RGSSError: disposed viewport\", true, true]"
    );
}

// Requirement "Constructors and object arguments of RGSS3".

/// Scenario "Constructors, measured on the original".
#[test]
fn the_constructors_take_nil_or_a_viewport_with_the_measured_messages() {
    assert_eq!(
        eval_with(
            "made << (v = Viewport.new)",
            "[(s = Sprite.new; r = s.viewport; s.dispose; r), \
              (s = Sprite.new(nil); r = s.viewport; s.dispose; r), \
              error { Sprite.new(5) }, error { Sprite.new(v, 1) }, error { Plane.new(5) }, \
              error { Plane.new(v, 1) }, error { Tilemap.new(v, 1) }]"
        ),
        "[nil, nil, \"TypeError: can't convert Fixnum into Viewport\", \
         \"ArgumentError: wrong number of arguments (1 for 2)\", \
         \"TypeError: can't convert Fixnum into Viewport\", \
         \"ArgumentError: wrong number of arguments (1 for 2)\", \
         \"ArgumentError: wrong number of arguments (1 for 2)\"]"
    );
}

/// Scenarios "Object writers, measured on the original" and "Plane writers".
#[test]
fn the_object_arguments_are_worded_as_the_original_words_them() {
    let objects = "made << (s = Sprite.new) << (v = Viewport.new) << (p = Plane.new)";
    for (source, expected) in [
        ("s.color = nil", "NilClass into Color"),
        ("s.color = 5", "Fixnum into Color"),
        ("s.tone = nil", "NilClass into Tone"),
        ("s.src_rect = nil", "NilClass into Rect"),
        ("s.src_rect = 5", "Fixnum into Rect"),
        ("s.bitmap = 5", "Fixnum into Bitmap"),
        ("v.rect = nil", "NilClass into Rect"),
        ("v.rect = 5", "Fixnum into Rect"),
        ("v.color = nil", "NilClass into Color"),
        ("v.tone = nil", "NilClass into Tone"),
        ("s.flash(5, 3)", "Fixnum into Color"),
        ("v.flash(5, 3)", "Fixnum into Color"),
        ("p.color = nil", "NilClass into Color"),
        ("p.tone = 5", "Fixnum into Tone"),
        ("p.bitmap = 5", "Fixnum into Bitmap"),
    ] {
        assert_eq!(
            eval_with(objects, source),
            format!("TypeError: can't convert {expected}"),
            "{source}"
        );
    }
    // What the writers take is still taken.
    assert_eq!(
        eval_with(
            &format!("{objects} << (b = Bitmap.new(4, 4))"),
            "s.bitmap = b; s.bitmap = nil; p.bitmap = b; s.flash(nil, 3); \
             v.flash(Color.new(1, 2, 3), 2); s.color = Color.new(1, 2, 3); \
             [s.bitmap, p.bitmap.equal?(b), s.color.red]"
        ),
        "[nil, true, 1.0]"
    );
}

/// Scenario "Plain values of a sprite, measured on the original".
#[test]
fn the_plain_values_of_a_sprite_are_converted_as_measured() {
    assert_eq!(
        eval_with(
            "made << (s = Sprite.new)",
            "[[:x, 5.9], [:x, -5.9], [:opacity, 256], [:opacity, -0.5], [:bush_opacity, 300], \
              [:bush_depth, -3], [:wave_amp, 5.7], [:wave_phase, 725.5], [:wave_phase, -30], \
              [:zoom_x, -1.5], [:angle, 725.5], [:mirror, 0], [:visible, nil], \
              [:blend_type, 1.9]].map { |name, value| s.send(\"#{name}=\", value); s.send(name) }"
        ),
        "[5, -5, 255, 0, 300, -3, 5, 5.5, 330.0, -1.5, 725.5, true, false, 1]"
    );
}

/// Scenario "Errors of plain values, measured on the original".
#[test]
fn the_plain_values_of_a_sprite_raise_as_measured() {
    assert_eq!(
        eval_with(
            "made << (s = Sprite.new)",
            "[error { s.x = nil }, error { s.x = '3' }, error { s.zoom_x = nil }, \
              error { s.blend_type = 3 }, error { s.blend_type = -1 }, \
              error { s.flash(Color.new(1, 2, 3), nil) }]"
        ),
        "[\"TypeError: no implicit conversion from nil to integer\", \
         \"TypeError: can't convert String into Integer\", \
         \"TypeError: no implicit conversion to float from nil\", \
         \"ArgumentError: bad value for blend_type\", \
         \"ArgumentError: bad value for blend_type\", \
         \"TypeError: no implicit conversion from nil to integer\"]"
    );
}

/// Scenario "Plane values, measured on the original".
#[test]
fn the_values_of_a_plane_are_those_measured() {
    assert_eq!(
        eval_with(
            "made << (p = Plane.new)",
            "defaults = [p.bitmap, p.viewport, p.z, p.ox, p.oy, p.zoom_x, p.opacity, \
                         p.blend_type, p.visible]; \
             read = [[:opacity, 300], [:opacity, 77.9], [:ox, -5.9], [:zoom_x, 0], \
                     [:blend_type, 1.9]].map { |name, value| \
                       p.send(\"#{name}=\", value); p.send(name) }; \
             [defaults, read, error { p.blend_type = 3 }]"
        ),
        "[[nil, nil, 0, 0, 0, 1.0, 255, 0, true], [255, 77, -5, 0.0, 1], \
         \"ArgumentError: bad value for blend_type\"]"
    );
}

/// Scenario "Disposed sprite and plane, measured on the original".
#[test]
fn a_disposed_sprite_and_plane_raise_as_measured() {
    assert_eq!(
        eval_with(
            "s = Sprite.new; s.dispose; p = Plane.new; p.dispose",
            "[[error { s.x }, error { s.x = 1 }, error { s.update }, error { s.width }, \
               error { s.viewport }].uniq, s.bitmap, s.dispose, s.disposed?, \
              [error { p.ox }, error { p.viewport }].uniq]"
        ),
        "[[\"RGSSError: disposed sprite\"], nil, nil, true, [\"RGSSError: disposed plane\"]]"
    );
}
