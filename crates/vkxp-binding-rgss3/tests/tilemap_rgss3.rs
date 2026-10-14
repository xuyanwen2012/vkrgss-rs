//! The class `Tilemap` in a game of RGSS3 over Ruby 1.9.2: the scenarios of "Tilemap class
//! of RGSS3", "Tables of a tilemap of RGSS3", "A tilemap of RGSS3 is drawn by the rules of
//! RGSS3" and "Disposed tilemap of RGSS3" of `rgss-scene-classes`. A file of this package
//! alone, so that it never runs as RGSS1 or RGSS2 over Ruby 1.8.7.
//!
//! The expected values are those measured on the original runtime of VX Ace (DLL versions
//! 3.0.0.1 and 3.0.1.1, 2026-10-13, `ace-v1-tilemap-REPORT.md` sections 4, 5, 7, 8 and 10,
//! kept outside the repository), but for the write to a slot of a disposed tilemap, which
//! ends the original and which vkxp keeps.
//!
//! A snippet is one evaluation and disposes what it drew before it ends, as in `scene.rs`.
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

const TILEMAP: &str = "made << (t = Tilemap.new) << (b = solid(32, 32, 255, 0, 0))";

/// Scenario "Methods, measured on the original".
#[test]
fn the_methods_are_those_of_the_original() {
    assert_eq!(
        eval(
            "Tilemap.instance_methods(false).sort.map { |m| \
             [m, Tilemap.instance_method(m).arity] }"
        ),
        "[[:bitmaps, 0], [:clone, 0], [:dispose, 0], [:disposed?, 0], [:dup, 0], \
         [:flags, 0], [:flags=, 1], [:flash_data, 0], [:flash_data=, 1], [:map_data, 0], \
         [:map_data=, 1], [:ox, 0], [:ox=, 1], [:oy, 0], [:oy=, 1], [:update, 0], \
         [:viewport, 0], [:viewport=, 1], [:visible, 0], [:visible=, 1]]"
    );
}

/// Scenario "Constructor and array, measured on the original".
#[test]
fn the_constructor_and_the_array_have_the_arities_of_the_original() {
    assert_eq!(
        eval(
            "[Tilemap.instance_method(:initialize).arity, \
              Tilemap::BitmapArray.instance_methods(false).sort.map { |m| \
              [m, Tilemap::BitmapArray.instance_method(m).arity] }]"
        ),
        "[-1, [[:[], 1], [:[]=, 2]]]"
    );
}

/// Scenario "A new tilemap, measured on the original".
#[test]
fn a_new_tilemap_holds_nothing() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "[t.map_data, t.flags, t.flash_data, t.viewport, t.ox, t.oy, t.visible, \
              (0..9).map { |i| t.bitmaps[i] }.compact, t.bitmaps.class, \
              t.bitmaps.equal?(t.bitmaps)]"
        ),
        "[nil, nil, nil, nil, 0, 0, true, [], Tilemap::BitmapArray, true]"
    );
}

/// Scenario "Names of the other runtimes, measured on the original".
#[test]
fn the_names_of_rgss1_and_rgss2_are_not_defined() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "[:passages, :passages=, :tileset, :autotiles, :priorities].map { |m| \
              t.respond_to?(m) } << Object.const_defined?(:TilemapAutotiles)"
        ),
        "[false, false, false, false, false, false]"
    );
}

/// Scenario "Viewport, measured on the original".
#[test]
fn the_viewport_is_kept_by_reference() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new(v = Viewport.new(1, 2, 3, 4))) << v \
             << (n = Tilemap.new(nil))",
            "[t.viewport.equal?(v), n.viewport, (t.viewport = nil), t.viewport]"
        ),
        "[true, nil, nil, nil]"
    );
}

/// Scenario "Slots, measured on the original".
#[test]
fn the_slots_keep_their_object() {
    assert_eq!(
        eval_with(
            TILEMAP,
            "t.bitmaps[9] = b; t.bitmaps[-1] = b; \
             [t.bitmaps[9], t.bitmaps[-1], (t.bitmaps[3] = b; t.bitmaps[3].equal?(b)), \
              (t.bitmaps[3] = nil; t.bitmaps[3])]"
        ),
        "[nil, nil, true, nil]"
    );
}

/// Scenario "Wrong arguments, measured on the original".
#[test]
fn wrong_arguments_raise_the_errors_of_the_original() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new) << (v = Viewport.new(0, 0, 8, 8))",
            "[try { t.bitmaps[3] = 5 }, try { t.bitmaps['a'] }, try { t.viewport = 5 }, \
              try { made << Tilemap.new(5) }, try { made << Tilemap.new(v, 1) }, \
              try { t.ox = '3' }, try { t.ox = nil }, try { t.ox = 2**31 }]"
        ),
        "[TypeError, TypeError, TypeError, TypeError, ArgumentError, TypeError, TypeError, \
         RangeError]"
    );
}

/// Scenario "Plain properties, measured on the original".
#[test]
fn the_plain_properties_take_the_values_of_the_original() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "t.ox = 1.9; t.oy = -1.9; a = [t.ox, t.oy]; t.ox = 2**30 - 1; t.oy = 1e3; \
             t.visible = nil; b = t.visible; t.visible = 5; \
             a + [t.ox, t.oy, b, t.visible, t.update]"
        ),
        "[1, -1, 1073741823, 1000, false, true, nil]"
    );
}

/// Scenario "No copies and no dump, measured on the original".
#[test]
fn a_tilemap_is_neither_copied_nor_dumped() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "[error { t.dup }, error { t.clone }, try { Marshal.dump(t) }]"
        ),
        "[\"TypeError: can't dup Tilemap\", \"TypeError: can't clone Tilemap\", TypeError]"
    );
}

/// Scenario "Nil on a fresh tilemap, measured on the original".
#[test]
fn nil_is_accepted_while_no_table_was_held() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "t.flags = nil; t.map_data = nil; t.flash_data = nil; \
             [t.flags, t.map_data, t.flash_data]"
        ),
        "[nil, nil, nil]"
    );
}

/// Scenario "Nil once a table is set, measured on the original".
#[test]
fn nil_is_refused_once_a_table_is_held() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new); f = Table.new(8192); m = Table.new(1, 1, 4)",
            "t.flags = f; t.map_data = m; \
             [error { t.flags = nil }, error { t.map_data = nil }, \
              [t.flags.equal?(f), t.map_data.equal?(m)]]"
        ),
        "[\"TypeError: can't convert NilClass into Table\", \
         \"TypeError: can't convert NilClass into Table\", [true, true]]"
    );
}

/// Scenario "Flash data, measured on the original".
#[test]
fn flash_data_can_be_taken_away() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "t.flash_data = Table.new(2, 2); t.flash_data = nil; t.flash_data"
        ),
        "nil"
    );
}

/// Scenario "Any dimensions, measured on the original".
#[test]
fn a_table_of_any_dimensions_is_taken() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "t.flags = Table.new(4, 4); t.map_data = Table.new(3); \
             [t.flags.ysize, t.map_data.xsize]"
        ),
        "[4, 3]"
    );
}

/// Scenario "Other values, measured on the original".
#[test]
fn a_value_that_is_no_table_is_refused() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "[try { t.flags = 5 }, try { t.map_data = 5 }, try { t.flash_data = 'a' }]"
        ),
        "[TypeError, TypeError, TypeError]"
    );
}

/// Scenario "Kept alive": the tilemap is the only holder of its flags table.
#[test]
fn the_flags_table_is_kept_alive_by_its_tilemap() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new)",
            "def flags_of_seven; f = Table.new(4); f[1] = 7; f; end; \
             t.flags = flags_of_seven; 3.times { GC.start }; \
             (1..2000).map { |i| Table.new(4)[1] = i }; GC.start; t.flags[1]"
        ),
        "7"
    );
}
