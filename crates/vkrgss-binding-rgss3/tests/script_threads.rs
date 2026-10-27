//! The RGSS classes called from script threads in a game of RGSS3 over Ruby 1.9.2, whose
//! threads are native threads: the scenarios of "RGSS classes from script threads" of
//! `binding-over-two-interpreters`. A file of this package alone, because over Ruby 1.8.7
//! every Ruby thread is the interpreter's one thread.
//!
//! Each call runs in a fresh `Thread.new { ... }.value`, as in the probe of the change
//! `call-rgss-from-script-threads`. The original runtime of VX Ace (DLL versions 3.0.0.1 and
//! 3.0.1.1, 2026-10-13, `ace-v0-REPORT.md`, kept outside the repository) answered
//! `Graphics.frame_count`, `Audio.se_stop` and `Bitmap.new` with `dispose` from a thread as
//! from the main script. Each snippet is evaluated with a scripted host that presses no
//! button.
//!
//! A snippet that counts bitmaps or scene elements is one evaluation, so that no snippet of
//! another test of this binary runs between its counts.
#![cfg(not(vkrgss_interpreter_absent))]
// The support module of `vkrgss-binding` names the binding by that name.
extern crate vkrgss_binding_rgss3 as vkrgss_binding;

#[path = "../../vkrgss-binding/tests/scene_support/mod.rs"]
mod scene_support;

use std::sync::OnceLock;

use vkrgss_binding::Guest;
use vkrgss_binding::testing::host::{self, Script};
use vkrgss_binding::testing::{self, Harness};
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{Arity, HostResult, Ruby};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS3, with `scene_count(kind)` and `bitmap_count`.
fn harness() -> &'static Harness {
    let harness = testing::start_as(Runtime::Rgss3);
    scene_support::install(harness);
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        harness.run(|ruby| {
            ruby.define_global_function_with_arity(c"bitmap_count", Arity::Fixed(0), bitmap_count)
                .expect("a new function");
        });
    });
    harness
}

/// The number of live bitmaps in the arena.
fn bitmap_count(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    let count = Guest::with(ruby, |guest| guest.bitmaps().len());
    Ok(ffi::long2fix(count.try_into().expect("a count that fits")))
}

/// `inspect` of the value of `source`, or the class and the message of the exception that
/// left it.
fn eval(source: &str) -> String {
    match host::play(harness(), source, Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// What [`eval`] returns for `source` evaluated in a script thread.
fn in_thread(source: &str) -> String {
    eval(&format!("Thread.new {{ {source} }}.value"))
}

/// Scenario "A bitmap and a sprite in a thread", and each call of the probe.
#[test]
fn rgss_classes_work_from_a_script_thread() {
    let cases = [
        (
            "b = Bitmap.new(8, 8); b.fill_rect(0, 0, 4, 4, Color.new(255, 0, 0)); \
             s = Sprite.new; s.bitmap = b; r = [b.width, Graphics.brightness]; \
             s.dispose; b.dispose; r",
            "[8, 255]",
        ),
        (
            "b = Bitmap.new(8, 8); b.fill_rect(0, 0, 4, 4, Color.new(255, 0, 0)); \
             c = b.get_pixel(1, 1); \
             r = [b.width, b.height, c.red, c.green, c.alpha, b.get_pixel(5, 5).alpha]; \
             b.dispose; r",
            "[8, 8, 255.0, 0.0, 255.0, 0.0]",
        ),
        (
            "b = Bitmap.new(8, 8); s = Sprite.new; s.bitmap = b; s.x = 3; \
             r = [s.bitmap.width, s.x, s.disposed?]; s.dispose; b.dispose; r",
            "[8, 3, false]",
        ),
        ("Graphics.brightness", "255"),
        (
            "c = Graphics.frame_count; Graphics.update; Graphics.frame_count - c",
            "1",
        ),
        ("Input.update; Input.press?(:C)", "false"),
        ("Audio.se_stop", "nil"),
        ("b = Bitmap.new(8, 8); b = nil; GC.start; :done", ":done"),
        (
            "v = Viewport.new(0, 0, 32, 32); p = Plane.new(v); w = Window.new; \
             t = Tilemap.new(v); f = Font.new; tb = Table.new(2, 2); tb[1, 1] = 5; \
             r = [Rect.new(1, 2, 3, 4).width, Tone.new(1, 2, 3).green, f.size > 0, tb[1, 1], \
             v.rect.width, p.disposed?, w.disposed?, t.disposed?]; \
             [t, w, p, v].each { |o| o.dispose }; r",
            "[3, 2.0, true, 5, 32, false, false, false]",
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(in_thread(source), expected, "{source}");
    }
}

/// Scenario "Collection in a thread": a hundred bitmaps and sprites a thread made and
/// dropped, collected on that thread, are gone after the next `Graphics.update` of the main
/// script. Garbage of earlier snippets is collected first, so the counts of after are at
/// most those of before.
#[test]
fn collection_on_a_script_thread_frees_bitmaps_and_elements() {
    let counts = eval(
        "GC.start; Graphics.update
         before = [bitmap_count, scene_count(:sprite)]
         during = Thread.new {
           100.times { s = Sprite.new; s.bitmap = Bitmap.new(4, 4) }
           made = [bitmap_count, scene_count(:sprite)]
           GC.start
           made
         }.value
         Graphics.update
         after = [bitmap_count, scene_count(:sprite)]
         [during.zip(before).map { |d, b| d - b }, after.zip(before).map { |a, b| a <= b }]",
    );
    assert_eq!(counts, "[[100, 100], [true, true]]");
}

/// The main script goes on as before once a thread that used the classes has ended.
#[test]
fn the_main_script_works_after_a_script_thread_ended() {
    assert_eq!(
        in_thread("b = Bitmap.new(2, 2); s = Sprite.new; s.bitmap = b; :done"),
        ":done"
    );
    assert_eq!(
        eval(
            "b = Bitmap.new(2, 2); s = Sprite.new; s.bitmap = b; \
             r = [s.bitmap.width, Graphics.brightness]; s.dispose; b.dispose; Graphics.update; r"
        ),
        "[2, 255]"
    );
}
