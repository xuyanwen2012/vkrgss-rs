//! The class `Tilemap` in a game of RGSS3 over Ruby 1.9.2: the scenarios of "Tilemap class
//! of RGSS3", "Tables of a tilemap of RGSS3", "A tilemap of RGSS3 is drawn by the rules of
//! RGSS3" and "Disposed tilemap of RGSS3" of `rgss-scene-classes`. A file of this package
//! alone, so that it never runs as RGSS1 or RGSS2 over Ruby 1.8.7.
//!
//! The expected values are those measured on the original runtime of VX Ace (DLL versions
//! 3.0.0.1 and 3.0.1.1, 2026-10-13, `ace-v1-tilemap-REPORT.md` sections 4, 5, 7, 8 and 10,
//! kept outside the repository), but for the write to a slot of a disposed tilemap, which
//! ends the original and which vkrgss keeps.
//!
//! A snippet is one evaluation and disposes what it drew before it ends, as in `scene.rs`.
#![cfg(not(vkrgss_interpreter_absent))]
// The support module of `vkrgss-binding` names the binding by that name.
extern crate vkrgss_binding_rgss3 as vkrgss_binding;

#[path = "../../vkrgss-binding/tests/scene_support/mod.rs"]
mod scene_support;

use vkrgss_binding::testing;
use vkrgss_vfs::Runtime;

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
/// A tilemap with a red sheet of 64 by 32 in slot 5, so that tile 1, drawn from (32, 0) of
/// that sheet, is red, and a map `m` of 1 by 1 by 4 with tile 1 in layer 0.
const RED_TILE: &str = "made << (t = Tilemap.new) << (b = solid(64, 32, 255, 0, 0)); \
                        t.bitmaps[5] = b; m = Table.new(1, 1, 4); m[0, 0, 0] = 1; \
                        t.map_data = m";
const RED: &str = "[255, 0, 0, 255]";
const GREEN: &str = "[0, 255, 0, 255]";
const BLACK: &str = "[0, 0, 0, 255]";

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

/// Scenarios "Hidden, measured on the original" and "Star tile of the lowest layer,
/// measured on the original".
#[test]
fn a_hidden_tilemap_draws_nothing_and_a_star_tile_of_layer_0_is_drawn() {
    assert_eq!(
        eval_with(
            RED_TILE,
            "r = [pixel(0, 0)]; t.visible = false; r << pixel(0, 0); \
             t.visible = true; r << pixel(0, 0); \
             f = Table.new(8192); f[1] = 0x10; t.flags = f; r << pixel(0, 0); r"
        ),
        format!("[{RED}, {BLACK}, {RED}, {RED}]")
    );
}

/// Scenario "Star tile of the third layer, measured on the original": a change through the
/// table object shows without `update`.
#[test]
fn a_star_tile_of_layer_2_is_drawn_above_a_sprite() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new) << (b = solid(64, 32, 255, 0, 0)) \
             << (g = solid(32, 32, 0, 255, 0)) << (s = Sprite.new); \
             s.bitmap = g; s.z = 100",
            "t.bitmaps[5] = b; m = Table.new(1, 1, 4); m[0, 0, 2] = 1; t.map_data = m; \
             f = Table.new(8192); t.flags = f; r = [pixel(0, 0)]; \
             f[1] = 0x10; r << pixel(0, 0); r"
        ),
        format!("[{GREEN}, {RED}]")
    );
}

/// Scenario "Shadow of the fourth layer, measured on the original": the shadow of bit 0
/// darkens the upper left quarter of the cell alone.
#[test]
fn the_fourth_layer_is_the_shadow() {
    assert_eq!(
        eval_with(
            RED_TILE,
            "m[0, 0, 3] = 1; [pixel(0, 0), pixel(16, 0), pixel(0, 16)]"
        ),
        format!("[[126, 0, 0, 255], {RED}, {RED}]")
    );
}

/// Scenarios "A map of three layers, measured on the original" and "A map of fewer layers".
#[test]
fn a_map_of_fewer_layers_is_drawn() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new) << (b = solid(64, 32, 255, 0, 0))",
            "t.bitmaps[5] = b; m = Table.new(1, 1, 3); m[0, 0, 0] = 1; t.map_data = m; \
             r = [pixel(0, 0), pixel(16, 16)]; \
             m = Table.new(1, 1); m[0, 0] = 1; t.map_data = m; r << pixel(0, 0); \
             m = Table.new(1); m[0] = 1; t.map_data = m; r << pixel(0, 0); r"
        ),
        format!("[{RED}, {RED}, {RED}, {RED}]")
    );
}

/// Scenario "No flags, measured on the original".
#[test]
fn a_tilemap_without_flags_is_drawn() {
    assert_eq!(
        eval_with(
            RED_TILE,
            "r = [pixel(0, 0)]; t.flags = Table.new(1); r << pixel(0, 0); r"
        ),
        format!("[{RED}, {RED}]")
    );
}

/// Scenarios "Disposed, measured on the original", "The array of a disposed tilemap,
/// measured on the original" and "A write after dispose": the write that ends the original
/// is kept, and the frame shows nothing of the tilemap.
#[test]
fn a_disposed_tilemap_keeps_its_array() {
    assert_eq!(
        eval_with(
            RED_TILE,
            "t.bitmaps[1] = b; r = [t.disposed?, t.dispose, t.disposed?, t.dispose]; \
             e = [error { t.ox }, error { t.ox = 1 }, error { t.update }, \
                  error { t.map_data }, error { t.flags }, error { t.flags = Table.new(1) }, \
                  error { t.flash_data }, error { t.visible }, error { t.viewport }, \
                  error { t.viewport = nil }].uniq; \
             a = [t.bitmaps.class, t.bitmaps[1].equal?(b), t.bitmaps[9]]; \
             t.bitmaps[2] = b; t.bitmaps[1] = nil; \
             [r, e, a, [t.bitmaps[2].equal?(b), t.bitmaps[1]], pixel(0, 0)]"
        ),
        format!(
            "[[false, nil, true, nil], [\"RGSSError: disposed tilemap\"], \
             [Tilemap::BitmapArray, true, nil], [true, nil], {BLACK}]"
        )
    );
}

/// Scenario "Not drawn any more".
#[test]
fn a_disposed_tilemap_is_not_drawn() {
    assert_eq!(
        eval_with(
            RED_TILE,
            "r = [pixel(0, 0)]; t.dispose; r << pixel(0, 0); r"
        ),
        format!("[{RED}, {BLACK}]")
    );
}
