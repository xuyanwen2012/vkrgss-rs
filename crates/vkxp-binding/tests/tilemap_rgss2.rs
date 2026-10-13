//! The class `Tilemap` in a game of RGSS2: the scenarios of "Tilemap class of RGSS2" of
//! `rgss-scene-classes`, and that what the class is given reaches the frame. A binary of its
//! own, because the classes are defined once per process, as the runtime of the run has them.
//!
//! A snippet is one evaluation and disposes what it drew before it ends, as in `scene.rs`.

mod scene_support;

use vkxp_binding::testing;
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
/// `try { ... }` is the class of the exception the block raises.
fn eval_with(objects: &str, source: &str) -> String {
    eval(&format!(
        "def try; yield; rescue Exception; $!.class; end; \
         made = []; begin; {objects}; {source}; ensure; made.each {{ |o| o.dispose }}; end"
    ))
}

const TILEMAP: &str = "made << (t = Tilemap.new) << (b = solid(32, 32, 255, 0, 0))";
const RED: &str = "[255, 0, 0, 255]";
const GREEN: &str = "[0, 255, 0, 255]";
const BLACK: &str = "[0, 0, 0, 255]";

/// Scenario "Slots, measured on the original".
#[test]
fn the_bitmap_array_has_nine_slots_that_keep_their_object() {
    assert_eq!(
        eval_with(
            TILEMAP,
            "t.bitmaps[9] = b; \
             [t.bitmaps[9], t.bitmaps[-1], (t.bitmaps[1] = b; t.bitmaps[1].equal?(b)), \
              t.bitmaps.equal?(t.bitmaps)]"
        ),
        "[nil, nil, true, true]"
    );
    assert_eq!(
        eval_with(
            TILEMAP,
            "a = t.bitmaps; fresh = (0..8).map { |i| a[i] }; \
             (0..8).each { |i| a[i] = b }; a[100] = b; a[4] = nil; \
             [a.class, fresh.compact, (0..8).map { |i| a[i].equal?(b) }, a[100], \
              (a[8] = b).equal?(b), (a[-1] = b).equal?(b), a[1.5].equal?(b)]"
        ),
        "[Tilemap::BitmapArray, [], \
         [true, true, true, true, false, true, true, true, true], nil, true, true, true]"
    );
}

#[test]
fn a_slot_takes_nil_or_a_bitmap_and_an_index_is_an_integer() {
    for (source, expected) in [
        ("t.bitmaps[0] = 1", "TypeError"),
        ("t.bitmaps[0] = Table.new(1)", "TypeError"),
        ("t.bitmaps[nil]", "TypeError"),
        ("t.bitmaps['1'] = b", "TypeError"),
    ] {
        let found = eval_with(TILEMAP, source);
        assert!(found.starts_with(expected), "{source}: {found}");
    }
    // A disposed bitmap is accepted and read back.
    assert_eq!(
        eval_with(
            TILEMAP,
            "d = Bitmap.new(8, 8); d.dispose; t.bitmaps[2] = d; t.bitmaps[2].equal?(d)"
        ),
        "true"
    );
    // An array a script made itself has no slots.
    assert_eq!(
        eval(
            "[Tilemap::BitmapArray.new.class, (Tilemap::BitmapArray.allocate[0] rescue $!.class)]"
        ),
        "[Tilemap::BitmapArray, TypeError]"
    );
}

/// Scenario "Tables, measured on the original".
#[test]
fn map_data_and_passages_cannot_be_taken_away_and_flash_data_can() {
    assert_eq!(
        eval_with(
            TILEMAP,
            "m = Table.new(4, 3, 3); p = Table.new(8192); f = Table.new(4, 3); \
             t.map_data = nil; t.passages = nil; before = [t.map_data, t.passages]; \
             t.map_data = m; t.passages = p; t.flash_data = f; \
             a = try { t.map_data = nil }; b = try { t.passages = nil }; \
             kept = [t.map_data.equal?(m), t.passages.equal?(p), t.flash_data.equal?(f)]; \
             t.flash_data = nil; \
             m2 = Table.new(1); t.map_data = m2; t.passages = m2; \
             [before, a, b, kept, t.flash_data, t.map_data.equal?(m2), t.passages.equal?(m2)]"
        ),
        "[[nil, nil], TypeError, TypeError, [true, true, true], nil, true, true]"
    );
    assert_eq!(
        eval_with(TILEMAP, "t.map_data = Table.new(1); t.map_data = nil"),
        "TypeError: can't convert NilClass into Table"
    );
    let found = eval_with(TILEMAP, "t.passages = 5");
    assert!(found.starts_with("TypeError"), "{found}");
}

/// Scenario "Disposed, measured on the original".
#[test]
fn the_bitmap_array_outlives_its_tilemap() {
    assert_eq!(
        eval_with(
            TILEMAP,
            "a = t.bitmaps; t.dispose; \
             [(t.bitmaps[1] = b; t.bitmaps[1].equal?(b)), t.bitmaps.equal?(a), \
              try { t.passages }, try { t.passages = nil }, try { t.map_data }, \
              try { t.ox }, (t.update rescue $!.message)]"
        ),
        "[true, true, RGSSError, RGSSError, RGSSError, RGSSError, \"disposed tilemap\"]"
    );
}

/// Scenario "Names of RGSS1, measured on the original".
#[test]
fn the_names_of_rgss1_are_not_defined() {
    assert_eq!(
        eval_with(
            TILEMAP,
            "[t.respond_to?(:tileset), Object.const_defined?(:TilemapAutotiles), \
              t.respond_to?(:autotiles), t.respond_to?(:priorities), t.update]"
        ),
        "[false, false, false, false, nil]"
    );
}

/// What the class is given reaches the frame: a tile of sheet B, the upper plane over a
/// sprite, an invisible tilemap, a scrolled one.
#[test]
fn a_tilemap_draws_what_it_was_given() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new) << (b = solid(512, 512, 255, 0, 0)) \
             << (g = solid(64, 64, 0, 255, 0)) << (s = Sprite.new); \
             s.bitmap = g; s.z = 100; m = Table.new(4, 3, 3); p = Table.new(8192)",
            "t.bitmaps[5] = b; t.passages = p; t.map_data = m; \
             m[0, 0, 0] = 1; m[1, 0, 2] = 2; r = [pixel(5, 5), pixel(40, 5)]; \
             p[2] = 0x10; r << pixel(40, 5); \
             t.visible = false; r << pixel(40, 5); \
             t.ox = 32; r << pixel(5, 5) << pixel(40, 5); \
             t.bitmaps[5] = nil; r << pixel(5, 5); r"
        ),
        format!("[{GREEN}, {GREEN}, {RED}, {RED}, {RED}, {GREEN}, {GREEN}]")
    );
    // Without a sprite: the tile, and black where the map is empty.
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new) << (b = solid(512, 512, 255, 0, 0)); \
             m = Table.new(4, 3, 3)",
            "t.bitmaps[5] = b; t.map_data = m; m[1, 1, 0] = 9; \
             [pixel(40, 40), pixel(5, 5), pixel(168, 40)]"
        ),
        format!("[{RED}, {BLACK}, {RED}]")
    );
}

/// Scenario "Other viewport, measured on the original" of `core-scene`, through the class.
#[test]
fn another_viewport_moves_the_planes_and_not_the_origin() {
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new(old = Viewport.new(0, 0, 544, 416))) \
             << (b = solid(512, 512, 255, 0, 0)) << old \
             << (v = Viewport.new(0, 0, 544, 416)); v.ox = 16; m = Table.new(40, 3, 3)",
            "t.bitmaps[5] = b; t.map_data = m; m[1, 0, 0] = 1; \
             r = [pixel(31, 5), pixel(32, 5)]; t.viewport = v; r << pixel(16, 5) << pixel(32, 5); \
             t.ox = t.ox; r << pixel(15, 5) << pixel(16, 5) << pixel(47, 5) << pixel(48, 5); \
             v.ox = 0; r << pixel(31, 5) << pixel(32, 5); r"
        ),
        format!("[{BLACK}, {RED}, {BLACK}, {RED}, {BLACK}, {RED}, {RED}, {BLACK}, {BLACK}, {RED}]")
    );
    // A change of the new viewport that is taken back before a frame is a change: the
    // tilemap has the origin of the new viewport from then on.
    assert_eq!(
        eval_with(
            "made << (t = Tilemap.new(old = Viewport.new(0, 0, 544, 416))) \
             << (b = solid(512, 512, 255, 0, 0)) << old \
             << (v = Viewport.new(0, 0, 544, 416)); m = Table.new(40, 3, 3)",
            "t.bitmaps[5] = b; t.map_data = m; m[1, 0, 0] = 1; old.ox = 16; \
             t.viewport = v; r = [pixel(15, 5), pixel(16, 5)]; \
             v.ox = 5; v.ox = 0; r << pixel(31, 5) << pixel(32, 5); r"
        ),
        format!("[{BLACK}, {RED}, {BLACK}, {RED}]")
    );
}
