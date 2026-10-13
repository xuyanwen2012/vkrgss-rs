//! The scene classes in a game of RGSS2: the scenarios "Moving an element to another
//! viewport" and "Small differences of RGSS2" of `rgss-scene-classes`. A binary of its own,
//! because the classes are defined once per process, as the runtime of the run has them.
//!
//! A snippet is one evaluation and disposes what it drew before it ends, as in `scene.rs`.

mod scene_support;

use vkxp_binding::testing;
use vkxp_binding::testing::host::{self, Script};
use vkxp_vfs::Runtime;

/// `inspect` of the value of `source` in the game of RGSS2, or the class and the message of
/// the exception that left it.
fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss2);
    scene_support::install();
    match testing::eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// What `eval` returns for a snippet that makes `objects` and disposes them at its end.
fn eval_with(objects: &str, source: &str) -> String {
    eval(&format!(
        "made = []; begin; {objects}; {source}; ensure; made.each {{ |o| o.dispose }}; end"
    ))
}

const RED: &str = "[255, 0, 0, 255]";
const BLACK: &str = "[0, 0, 0, 255]";

/// Scenario "Plane, measured on the original".
#[test]
fn a_plane_moves_into_a_viewport_and_out_of_it() {
    assert_eq!(
        eval_with(
            "made << (b = solid(32, 32, 255, 0, 0)) << (plane = Plane.new) \
             << (v = Viewport.new(0, 0, 50, 50)); plane.bitmap = b",
            "before = [pixel(10, 10), pixel(100, 100)]; \
             given = plane.viewport(v); inside = [pixel(10, 10), pixel(100, 100)]; \
             none = plane.viewport(nil); \
             [before, given.equal?(v), inside, none, [pixel(10, 10), pixel(100, 100)]]"
        ),
        format!("[[{RED}, {RED}], true, [{RED}, {BLACK}], nil, [{RED}, {RED}]]")
    );
}

/// Scenario "Seen by the scene", through the classes.
#[test]
fn a_moved_sprite_is_drawn_in_its_new_viewport() {
    assert_eq!(
        eval_with(
            "made << (b = solid(8, 8, 255, 0, 0)) << (s = Sprite.new) \
             << (v = Viewport.new(20, 30, 16, 16)); s.bitmap = b",
            "before = [pixel(0, 0), pixel(20, 30)]; s.viewport = v; \
             [before, [pixel(0, 0), pixel(20, 30)]]"
        ),
        format!("[[{RED}, {BLACK}], [{BLACK}, {RED}]]")
    );
}

/// A window and a tilemap move as well. A window without a skin shows its contents, 16
/// pixels inside its corner.
#[test]
fn a_window_and_a_tilemap_move() {
    assert_eq!(
        eval_with(
            "made << (b = solid(8, 8, 255, 0, 0)) << (w = Window.new) \
             << (v = Viewport.new(100, 100, 64, 64)); w.width = 40; w.height = 40; w.contents = b",
            "before = [pixel(16, 16), pixel(116, 116)]; w.viewport = v; \
             [before, [pixel(16, 16), pixel(116, 116)], w.viewport.equal?(v)]"
        ),
        format!("[[{RED}, {BLACK}], [{BLACK}, {RED}], true]")
    );
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new) << (v = Viewport.new(0, 0, 8, 8))",
            "a = t.viewport; t.viewport = v; b = t.viewport.equal?(v); t.viewport = nil; \
             [a, b, t.viewport]"
        ),
        "[nil, true, nil]"
    );
}

/// Scenario "Readers and errors, measured on the original".
#[test]
fn the_readers_return_the_object_given_and_other_objects_are_refused() {
    assert_eq!(
        eval_with(
            "made << (s = Sprite.new) << (v = Viewport.new(0, 0, 8, 8))",
            "s.viewport = v; a = s.viewport.equal?(v); s.viewport = nil; [a, s.viewport]"
        ),
        "[true, nil]"
    );
    let objects = "made << (s = Sprite.new) << (w = Window.new) << (t = Tilemap.new) \
                   << (plane = Plane.new)";
    for (source, expected) in [
        (
            "s.viewport = 5",
            "TypeError: can't convert Fixnum into Viewport",
        ),
        (
            "w.viewport = 'v'",
            "TypeError: can't convert String into Viewport",
        ),
        (
            "t.viewport = s",
            "TypeError: can't convert Sprite into Viewport",
        ),
        (
            "plane.viewport(5)",
            "TypeError: can't convert Fixnum into Viewport",
        ),
        (
            "plane.viewport",
            "ArgumentError: wrong number of arguments (0 for 1)",
        ),
        (
            "[plane.respond_to?(:viewport=), Plane.instance_method(:viewport).arity]",
            "[false, 1]",
        ),
        (
            "[Sprite, Window, Tilemap].map { |c| \
               [c.instance_method(:viewport).arity, c.instance_method(:viewport=).arity] }",
            "[[0, 1], [0, 1], [0, 1]]",
        ),
    ] {
        assert_eq!(eval_with(objects, source), expected, "{source}");
    }
}

/// Scenario "Disposed".
#[test]
fn a_disposed_viewport_and_a_disposed_element_raise() {
    let objects = "made << (s = Sprite.new) << (w = Window.new) << (t = Tilemap.new) \
                   << (plane = Plane.new) << (keep = Viewport.new(0, 0, 8, 8)); \
                   v = Viewport.new(0, 0, 8, 8); v.dispose";
    for source in [
        "s.viewport = v",
        "w.viewport = v",
        "t.viewport = v",
        "plane.viewport(v)",
    ] {
        assert_eq!(
            eval_with(objects, source),
            "RGSSError: disposed viewport",
            "{source}"
        );
    }
    // The refused viewport changed nothing.
    assert_eq!(
        eval_with(
            objects,
            "s.viewport = keep; begin; s.viewport = v; rescue RGSSError; end; \
             s.viewport.equal?(keep)"
        ),
        "true"
    );
    for (source, expected) in [
        ("d = Sprite.new; d.dispose; d.viewport = nil", "sprite"),
        ("d = Window.new; d.dispose; d.viewport = nil", "window"),
        ("d = Tilemap.new; d.dispose; d.viewport = nil", "tilemap"),
        ("d = Plane.new; d.dispose; d.viewport(nil)", "plane"),
        // A disposed element raises before its argument is looked at, as every writer.
        ("d = Sprite.new; d.dispose; d.viewport = 5", "sprite"),
    ] {
        assert_eq!(
            eval(source),
            format!("RGSSError: disposed {expected}"),
            "{source}"
        );
    }
}

/// An element that was moved belongs to its new viewport: that one takes it along.
#[test]
fn a_moved_element_is_disposed_with_its_new_viewport() {
    assert_eq!(
        eval_with(
            "made << (old = Viewport.new(0, 0, 8, 8)) << (s = Sprite.new(old)) \
             << (plane = Plane.new(old))",
            "new = Viewport.new(0, 0, 8, 8); s.viewport = new; plane.viewport(nil); \
             old.dispose; a = [s.disposed?, plane.disposed?]; new.dispose; \
             a + [s.disposed?, plane.disposed?]"
        ),
        "[false, false, true, false]"
    );
}

/// Scenario "Size of a sprite, measured on the original".
#[test]
fn a_sprite_tells_the_size_of_its_source_rectangle() {
    assert_eq!(
        eval_with(
            "made << (s = Sprite.new) << (b = Bitmap.new(7, 9))",
            "a = [s.width, s.height]; s.src_rect = Rect.new(1, 2, 100, -12); \
             c = [s.width, s.height]; s.bitmap = b; s.src_rect.width = 3; \
             [a, c, [s.width, s.height]]"
        ),
        "[[0, 0], [100, -12], [3, 9]]"
    );
    assert_eq!(
        eval("d = Sprite.new; d.dispose; d.width"),
        "RGSSError: disposed sprite"
    );
}

/// Scenario "Plane that is hidden, measured on the original".
#[test]
fn a_hidden_plane_reads_visible() {
    assert_eq!(
        eval_with(
            "made << (b = solid(32, 32, 255, 0, 0)) << (plane = Plane.new); plane.bitmap = b",
            "shown = pixel(10, 10); plane.visible = false; [shown, plane.visible, pixel(10, 10)]"
        ),
        format!("[{RED}, true, {BLACK}]")
    );
}

/// Scenario "Contents of a new window, measured on the original".
#[test]
fn a_new_window_has_contents_of_one_pixel_that_outlive_it() {
    assert_eq!(
        eval(
            "w = Window.new; c = w.contents; a = [c.class, c.width, c.height, c.equal?(w.contents)]; \
             w.dispose; d = c.disposed?; c.dispose; [a, d]"
        ),
        "[[Bitmap, 1, 1, true], false]"
    );
    // The bitmap is an ordinary one, and another can take its place.
    assert_eq!(
        eval_with(
            "made << (w = Window.new) << (c = w.contents) << (b = Bitmap.new(4, 4))",
            "c.set_pixel(0, 0, Color.new(1, 2, 3)); f = c.font.class; w.contents = b; \
             [c.get_pixel(0, 0).red, f, w.contents.equal?(b), c.disposed?]"
        ),
        "[1.0, Font, true, false]"
    );
}

/// Scenario "Names that RGSS2 lacks, measured on the original". The constant is asked for
/// with the prelude in place.
#[test]
fn stretch_and_hangup_are_not_defined() {
    assert_eq!(
        eval(
            "w = Window.new; r = [w.respond_to?(:stretch), w.respond_to?(:stretch=)]; w.dispose; r"
        ),
        "[false, false]"
    );
    let (value, _) = host::play(
        "[Object.const_defined?(:Hangup), Object.const_defined?(:Reset)]",
        Script::new(),
    );
    assert_eq!(value.as_deref(), Ok("[false, true]"));
}
