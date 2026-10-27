//! The scenarios of `rgss-scene-classes`, written in Ruby and evaluated in the embedded
//! interpreter. The snippets are in `scene_cases`; what they see of the scene comes from
//! the helpers of `scene_support`.

mod cases;
mod scene_cases;
mod scene_support;

use cases::Case;
use vkrgss_binding::scene_testing;
use vkrgss_binding::testing::{self, Harness};
use vkrgss_core::FontDefaults;
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS1.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

fn check(cases: &[Case]) {
    scene_support::install(harness());
    let wrong = cases::failures(harness(), cases, str::to_owned);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn viewport() {
    check(scene_cases::VIEWPORT);
}

#[test]
fn sprite_construction_and_plain_properties() {
    check(scene_cases::SPRITE_PLAIN);
}

#[test]
fn sprite_object_properties() {
    check(scene_cases::SPRITE_OBJECTS);
}

#[test]
fn sprite_bitmap() {
    check(scene_cases::SPRITE_BITMAP);
}

#[test]
fn sprite_disposal() {
    check(scene_cases::SPRITE_DISPOSAL);
}

#[test]
fn sprite_flash() {
    check(scene_cases::SPRITE_FLASH);
}

#[test]
fn plane() {
    check(scene_cases::PLANE);
}

#[test]
fn window() {
    check(scene_cases::WINDOW);
}

#[test]
fn tilemap() {
    check(scene_cases::TILEMAP);
}

#[test]
fn no_copies() {
    check(scene_cases::NO_COPIES);
}

#[test]
fn lifetime() {
    check(scene_cases::LIFETIME);
}

/// The scenarios "XP" of "Moving an element to another viewport" and of "Small differences
/// of RGSS2": the classes of a game of RGSS1 have none of the names RGSS2 adds.
#[test]
fn the_classes_of_rgss1_lack_what_rgss2_adds() {
    scene_support::install(harness());
    assert_eq!(
        harness()
            .eval(
                "s = Sprite.new; pl = Plane.new; w = Window.new; t = Tilemap.new; \
             r = [s.respond_to?(:viewport=), w.respond_to?(:viewport=), \
                  t.respond_to?(:viewport=), pl.respond_to?(:viewport=), pl.viewport, \
                  w.contents, s.respond_to?(:width), s.respond_to?(:height), \
                  w.respond_to?(:stretch), w.stretch]; \
             pl.visible = false; r << pl.visible; [s, pl, w, t].each { |o| o.dispose }; r"
            )
            .as_deref(),
        Ok("[false, false, false, false, nil, nil, false, false, true, true, false]")
    );
    // Scenario "XP and VX" of the tilemap of RGSS3: a tilemap of RGSS1 has no `flags`.
    assert_eq!(
        harness()
            .eval("t = Tilemap.new; r = t.respond_to?(:flags); t.dispose; r")
            .as_deref(),
        Ok("false")
    );
}

/// Scenario "XP" of "Sprite properties of RGSS2": a sprite of RGSS1 has none of the five,
/// and `bitmap=` with the bitmap it has sets its source rectangle again.
#[test]
fn a_sprite_of_rgss1_has_no_wave_and_no_bush_opacity() {
    scene_support::install(harness());
    assert_eq!(
        harness().eval(
            "s = Sprite.new; b = Bitmap.new(8, 8);              r = [:wave_amp, :wave_length, :wave_speed, :wave_phase, :bush_opacity].map { |m|                s.respond_to?(m) || s.respond_to?(\"#{m}=\") };              r << Sprite.new.respond_to?(:wave_amp);              s.bitmap = b; s.src_rect.set(1, 2, 3, 4); s.bitmap = b;              r << [s.src_rect.x, s.src_rect.y, s.src_rect.width, s.src_rect.height];              s.dispose; b.dispose; r"
        )
        .as_deref(),
        Ok("[false, false, false, false, false, false, [0, 0, 8, 8]]")
    );
}

#[test]
fn every_group_is_tested_here() {
    // One test function per group above: a group added to the list must get one too.
    assert_eq!(scene_cases::GROUPS.len(), 11);
}

#[test]
fn ruby_reads_the_pixel_of_a_sprite_made_in_rust() {
    scene_support::install(harness());
    // In a corner of the screen that no snippet draws to.
    let (sprite, dot) = harness().run(|ruby| {
        scene_testing::with_scene(ruby, |scene, bitmaps| {
            let defaults = FontDefaults::new(Vec::new());
            let dot = bitmaps
                .create_from_rgba(1, 1, vec![10, 20, 30, 255], &defaults)
                .unwrap();
            let sprite = scene.create_sprite(None).unwrap();
            let element = scene.sprite_mut(sprite).unwrap();
            element.set_bitmap(Some(dot), bitmaps);
            element.x = 639;
            element.y = 479;
            (sprite, dot)
        })
    });
    assert_eq!(
        harness()
            .eval("[pixel(639, 479), pixel(640, 479), pixel(0, 480)]")
            .as_deref(),
        Ok("[[10, 20, 30, 255], nil, nil]")
    );
    harness().run(move |ruby| {
        scene_testing::with_scene(ruby, |scene, bitmaps| {
            scene.dispose_sprite(sprite);
            bitmaps.dispose(dot);
        });
    });
    assert_eq!(
        harness().eval("pixel(639, 479)").as_deref(),
        Ok("[0, 0, 0, 255]")
    );
}

#[test]
fn elements_are_counted_by_kind() {
    scene_support::install(harness());
    assert_eq!(
        harness()
            .eval("[scene_count(:sprite).class, scene_count(:tilemap).class, scene_count(:x)]")
            .as_deref(),
        Ok("[Fixnum, Fixnum, nil]")
    );
    // A sprite that is held counts, and stops counting when it is disposed.
    assert_eq!(
        harness()
            .eval(
                "s = Sprite.new; held = scene_count(:sprite); s.dispose; \
             [held >= 1, scene_count(:sprite) < held]"
            )
            .as_deref(),
        Ok("[true, true]")
    );
}

/// Scenario "XP" of "Small rules of RGSS2 in the modules and elements": in RGSS1
/// `blend_type=` takes any value.
#[test]
fn blend_type_takes_any_value_in_rgss1() {
    scene_support::install(harness());
    for class in ["Sprite", "Plane"] {
        let source = format!(
            "s = {class}.new; begin; s.blend_type = 3; three = s.blend_type; \
             s.blend_type = -1; [three, s.blend_type]; ensure; s.dispose; end"
        );
        assert_eq!(harness().eval(&source).unwrap(), "[3, -1]", "{class}");
    }
}

/// Scenarios "XP and VX" of "Window construction in RGSS3" and "Window methods added in
/// RGSS3", for a game of RGSS1: `Window.new` takes a viewport and not four Integers, and the
/// window has none of the methods RGSS3 adds.
#[test]
fn a_window_of_rgss1_is_not_the_window_of_rgss3() {
    scene_support::install(harness());
    assert_eq!(
        harness()
            .eval(
                "v = Viewport.new(0, 0, 10, 10); w = Window.new(v); \
             r = [w.viewport.nil?, (Window.new(1, 2, 3, 4) rescue $!.class)]; \
             r << [:padding, :padding_bottom, :arrows_visible, :move, :open?, :close?, :tone] \
             .map { |name| w.respond_to?(name) }; w.dispose; v.dispose; r"
            )
            .as_deref(),
        Ok("[false, ArgumentError, [false, false, false, false, false, false, false]]")
    );
}

/// Scenario "XP and VX" of "The scene classes of RGSS1 and RGSS2 are unchanged by RGSS3",
/// for a game of RGSS1: no `Plane#viewport=`, no `Viewport.new` without arguments, and a
/// viewport takes its elements along.
#[test]
fn the_scene_classes_of_rgss1_are_not_those_of_rgss3() {
    scene_support::install(harness());
    assert_eq!(
        harness()
            .eval(
                "p = Plane.new; a = p.respond_to?(:viewport=); p.dispose; \
             b = begin; Viewport.new; rescue ArgumentError; ArgumentError; end; \
             v = Viewport.new(0, 0, 8, 8); s = Sprite.new(v); v.dispose; [a, b, s.disposed?]"
            )
            .as_deref(),
        Ok("[false, ArgumentError, true]")
    );
}
