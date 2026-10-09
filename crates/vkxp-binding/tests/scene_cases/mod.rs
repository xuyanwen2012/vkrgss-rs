//! The Ruby test snippets of the scene classes and what each evaluates to.
//!
//! `tests/scene.rs` evaluates them as they are; `tests/scene_stress.rs` evaluates the same
//! snippets with a garbage collection at every allocation and expects the same results.
//!
//! The snippets use `pixel` and `scene_count` of `scene_support`, and `solid(width, height,
//! red, green, blue)`, an opaque bitmap of one colour. All tests of a binary share one
//! scene, so a snippet that draws disposes what it made before it ends, and what a snippet
//! leaves to the collector is invisible.

#![allow(dead_code)]

use crate::cases::{Case, Expect};

const fn value(name: &'static str, source: &'static str, inspect: &'static str) -> Case {
    Case {
        name,
        source,
        expect: Expect::Value(inspect),
    }
}

const fn raises(name: &'static str, source: &'static str, class: &'static str) -> Case {
    Case {
        name,
        source,
        expect: Expect::Raises(class),
    }
}

const fn raises_with(
    name: &'static str,
    source: &'static str,
    class: &'static str,
    message: &'static str,
) -> Case {
    Case {
        name,
        source,
        expect: Expect::RaisesWith(class, message),
    }
}

/// Every group of cases, by name.
pub const GROUPS: &[(&str, &[Case])] = &[
    ("viewport", VIEWPORT),
    ("sprite construction and plain properties", SPRITE_PLAIN),
    ("sprite object properties", SPRITE_OBJECTS),
    ("sprite bitmap", SPRITE_BITMAP),
    ("sprite disposal", SPRITE_DISPOSAL),
    ("sprite flash", SPRITE_FLASH),
    ("plane", PLANE),
];

// --- Viewport ---------------------------------------------------------------------------------

pub const VIEWPORT: &[Case] = &[
    value(
        "from four numbers",
        "v = Viewport.new(1, 2, 3, 4); r = v.rect; v.dispose; [r.x, r.y, r.width, r.height]",
        "[1, 2, 3, 4]",
    ),
    value(
        "from a rectangle",
        "v = Viewport.new(Rect.new(1, 2, 3, 4)); r = v.rect == Rect.new(1, 2, 3, 4); v.dispose; r",
        "true",
    ),
    raises_with(
        "two arguments",
        "Viewport.new(1, 2)",
        "ArgumentError",
        "wrong number of arguments (2 for 4)",
    ),
    raises("no rectangle", r#"Viewport.new("rect")"#, "TypeError"),
    value(
        "plain properties",
        "v = Viewport.new(0, 0, 10, 10); d = [v.visible, v.z, v.ox, v.oy]; \
         v.visible = nil; v.z = 5; v.ox = 3; v.oy = -4; \
         r = [d, v.visible, v.z, v.ox, v.oy]; v.dispose; r",
        "[[true, 0, 0, 0], false, 5, 3, -4]",
    ),
    value(
        "no disposed?",
        "Viewport.method_defined?(:disposed?)",
        "false",
    ),
    value(
        "the object properties are the viewport's own",
        "v = Viewport.new(0, 0, 10, 10); \
         r = [v.rect.equal?(v.rect), v.color.equal?(v.color), v.tone.equal?(v.tone)]; \
         v.dispose; r",
        "[true, true, true]",
    ),
    value(
        "the rectangle clips, and follows a change in place and an assignment",
        "b = solid(8, 8, 255, 0, 0); v = Viewport.new(10, 10, 4, 4); s = Sprite.new(v); \
         s.bitmap = b; r = [pixel(10, 10), pixel(14, 10)]; \
         v.rect.set(10, 10, 8, 8); r << pixel(14, 10); \
         given = Rect.new(20, 20, 2, 2); v.rect = given; given.x = 0; \
         r << pixel(10, 10) << pixel(20, 20) << v.rect.x; \
         s.dispose; v.dispose; b.dispose; r",
        "[[255, 0, 0, 255], [0, 0, 0, 255], [255, 0, 0, 255], [0, 0, 0, 255], [255, 0, 0, 255], 20]",
    ),
    value(
        "colour and tone follow a change in place and an assignment",
        "v = Viewport.new(30, 30, 2, 2); v.color.set(0, 255, 0, 255); r = [pixel(30, 30)]; \
         given = Color.new(0, 0, 255); v.color = given; given.blue = 0; r << pixel(31, 31); \
         v.color.alpha = 0; v.tone.set(255, 255, 255); r << pixel(30, 30); \
         v.tone = Tone.new(0, 0, 0); r << pixel(30, 30) << pixel(32, 30); \
         v.dispose; r",
        "[[0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255], [0, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    value(
        "scrolling and visibility",
        "b = solid(2, 2, 255, 0, 0); v = Viewport.new(40, 40, 4, 4); s = Sprite.new(v); \
         s.bitmap = b; r = [pixel(40, 40), pixel(42, 40)]; v.ox = -2; \
         r << pixel(40, 40) << pixel(42, 40); v.visible = false; r << pixel(42, 40); \
         s.dispose; v.dispose; b.dispose; r",
        "[[255, 0, 0, 255], [0, 0, 0, 255], [0, 0, 0, 255], [255, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    value(
        "a flash lays its colour over the rectangle until it ends",
        "v = Viewport.new(50, 50, 2, 2); v.flash(Color.new(0, 0, 255), 2); r = [pixel(50, 50)]; \
         2.times { v.update }; r << pixel(50, 50); v.dispose; r",
        "[[0, 0, 255, 255], [0, 0, 0, 255]]",
    ),
    value(
        "a flash without a colour hides the elements",
        "b = solid(2, 2, 255, 0, 0); v = Viewport.new(60, 60, 2, 2); s = Sprite.new(v); \
         s.bitmap = b; v.flash(nil, 2); r = [pixel(60, 60)]; 2.times { v.update }; \
         r << pixel(60, 60); s.dispose; v.dispose; b.dispose; r",
        "[[0, 0, 0, 255], [255, 0, 0, 255]]",
    ),
    raises_with(
        "use after dispose",
        "v = Viewport.new(0, 0, 1, 1); v.dispose; v.dispose; v.ox",
        "RGSSError",
        "disposed viewport",
    ),
    raises_with(
        "an object property after dispose",
        "v = Viewport.new(0, 0, 1, 1); v.dispose; v.rect",
        "RGSSError",
        "disposed viewport",
    ),
    value(
        "a disposed viewport is not drawn any more",
        "v = Viewport.new(70, 70, 2, 2); v.color.set(255, 0, 0); r = [pixel(70, 70)]; \
         v.dispose; r << pixel(70, 70)",
        "[[255, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    raises_with(
        "a disposed viewport given to a constructor",
        "v = Viewport.new(0, 0, 1, 1); v.dispose; Sprite.new(v)",
        "RGSSError",
        "disposed viewport",
    ),
    raises_with(
        "a viewport that was never initialized",
        "Viewport.allocate.z",
        "RGSSError",
        "disposed viewport",
    ),
];

// --- Sprite -----------------------------------------------------------------------------------

pub const SPRITE_PLAIN: &[Case] = &[
    value(
        "in a viewport",
        "v = Viewport.new(0, 0, 100, 100); s = Sprite.new(v); r = s.viewport.equal?(v); \
         s.dispose; v.dispose; r",
        "true",
    ),
    value(
        "without a viewport",
        "[Sprite.new.viewport, Sprite.new(nil).viewport]",
        "[nil, nil]",
    ),
    raises("wrong argument", r#"Sprite.new("viewport")"#, "TypeError"),
    raises_with(
        "two arguments",
        "Sprite.new(nil, nil)",
        "ArgumentError",
        "wrong number of arguments (2 for 1)",
    ),
    value(
        "defaults",
        "s = Sprite.new; [s.visible, s.x, s.y, s.z, s.ox, s.oy, s.zoom_x, s.zoom_y, s.angle, \
         s.mirror, s.bush_depth, s.opacity, s.blend_type, s.bitmap, s.disposed?]",
        "[true, 0, 0, 0, 0, 0, 1.0, 1.0, 0.0, false, 0, 255, 0, nil, false]",
    ),
    value(
        "round trip",
        "s = Sprite.new; s.x = 10; s.zoom_x = 1.5; s.mirror = true; s.opacity = 300; \
         [s.x, s.zoom_x, s.mirror, s.opacity]",
        "[10, 1.5, true, 255]",
    ),
    value(
        "round trip of the other properties",
        "s = Sprite.new; s.visible = false; s.y = -7; s.z = 200; s.ox = 3; s.oy = 4; \
         s.zoom_y = 2; s.angle = 90; s.bush_depth = 12; s.blend_type = 1; s.opacity = -5; \
         s.mirror = nil; \
         [s.visible, s.y, s.z, s.ox, s.oy, s.zoom_y, s.angle, s.bush_depth, s.blend_type, \
         s.opacity, s.mirror]",
        "[false, -7, 200, 3, 4, 2.0, 90.0, 12, 1, 0, false]",
    ),
    value(
        "a writer returns what it was given",
        "s = Sprite.new; [(s.x = 2.5), s.x, s.send(:zoom_x=, 2)]",
        "[2.5, 2, 2]",
    ),
    raises(
        "a coordinate that is no number",
        r#"Sprite.new.x = "1""#,
        "TypeError",
    ),
    value(
        "seen by the scene",
        "b = solid(1, 1, 255, 0, 0); s = Sprite.new; s.bitmap = b; s.x = 5; s.y = 6; \
         r = [pixel(5, 6), pixel(4, 6)]; s.dispose; b.dispose; r",
        "[[255, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    value(
        "the sprite with the higher z is in front",
        "red = solid(1, 1, 255, 0, 0); blue = solid(1, 1, 0, 0, 255); \
         a = Sprite.new; a.bitmap = red; b = Sprite.new; b.bitmap = blue; \
         a.x = b.x = 80; r = [pixel(80, 0)]; a.z = 1; r << pixel(80, 0); \
         a.dispose; b.dispose; red.dispose; blue.dispose; r",
        "[[0, 0, 255, 255], [255, 0, 0, 255]]",
    ),
    value(
        "a subclass defined by a script",
        "class CaseSprite < Sprite; def initialize(viewport = nil); super(viewport); \
         self.x = 3; end; end; s = CaseSprite.new; [s.x, s.class, s.disposed?]",
        "[3, CaseSprite, false]",
    ),
];

pub const SPRITE_OBJECTS: &[Case] = &[
    value(
        "changing in place",
        "b = solid(1, 1, 255, 255, 255); s = Sprite.new; s.bitmap = b; s.x = 90; \
         s.color.set(255, 0, 0, 255); r = pixel(90, 0); s.dispose; b.dispose; r",
        "[255, 0, 0, 255]",
    ),
    value(
        "identity",
        "s = Sprite.new; [s.src_rect.equal?(s.src_rect), s.color.equal?(s.color), \
         s.tone.equal?(s.tone)]",
        "[true, true, true]",
    ),
    value(
        "assignment copies",
        "s = Sprite.new; c = Color.new(0, 255, 0); s.color = c; c.green = 0; \
         [s.color.green, s.color.equal?(c)]",
        "[255.0, false]",
    ),
    value(
        "an assigned colour reaches the scene",
        "b = solid(1, 1, 255, 255, 255); s = Sprite.new; s.bitmap = b; s.x = 91; \
         s.color = Color.new(0, 255, 0); r = pixel(91, 0); s.dispose; b.dispose; r",
        "[0, 255, 0, 255]",
    ),
    value(
        "the tone follows a change in place and an assignment",
        "b = solid(1, 1, 100, 100, 100); s = Sprite.new; s.bitmap = b; s.x = 92; \
         s.tone.red = 100; r = [pixel(92, 0)]; t = Tone.new(0, 100, 0); s.tone = t; \
         t.green = 0; r << pixel(92, 0) << s.tone.green; s.dispose; b.dispose; r",
        "[[200, 100, 100, 255], [100, 200, 100, 255], 100.0]",
    ),
    value(
        "the source rectangle follows a change in place and an assignment",
        "b = Bitmap.new(2, 1); b.set_pixel(0, 0, Color.new(255, 0, 0)); \
         b.set_pixel(1, 0, Color.new(0, 255, 0)); s = Sprite.new; s.bitmap = b; s.x = 93; \
         s.src_rect.set(1, 0, 1, 1); r = [pixel(93, 0), pixel(94, 0)]; \
         given = Rect.new(0, 0, 1, 1); s.src_rect = given; given.x = 1; \
         r << pixel(93, 0) << s.src_rect.x; s.dispose; b.dispose; r",
        "[[0, 255, 0, 255], [0, 0, 0, 255], [255, 0, 0, 255], 0]",
    ),
    raises(
        "a colour of the wrong class",
        "Sprite.new.color = Tone.new(0, 0, 0)",
        "TypeError",
    ),
    raises(
        "a rectangle of the wrong class",
        "Sprite.new.src_rect = nil",
        "TypeError",
    ),
    value(
        "an object property outlives its sprite as a plain value",
        "s = Sprite.new; c = s.color; r = s.src_rect; s.dispose; c.red = 5; r.width = 7; \
         [c.red, r.width]",
        "[5.0, 7]",
    ),
    value(
        "a copy of an object property is a plain value",
        "b = solid(1, 1, 255, 255, 255); s = Sprite.new; s.bitmap = b; s.x = 95; \
         c = s.color.dup; c.set(255, 0, 0); r = [pixel(95, 0), s.color.alpha]; \
         s.dispose; b.dispose; r",
        "[[255, 255, 255, 255], 0.0]",
    ),
    value(
        "the hidden instance variables are not listed",
        "Sprite.new.instance_variables",
        "[]",
    ),
];

pub const SPRITE_BITMAP: &[Case] = &[
    value(
        "same object back",
        "b = Bitmap.new(8, 8); s = Sprite.new; s.bitmap = b; \
         [s.bitmap.equal?(b), s.src_rect == Rect.new(0, 0, 8, 8)]",
        "[true, true]",
    ),
    value(
        "nil takes the bitmap away and leaves the source rectangle",
        "b = solid(8, 8, 255, 0, 0); s = Sprite.new; s.bitmap = b; s.x = 100; s.bitmap = nil; \
         r = [s.bitmap, s.src_rect.width, pixel(100, 0)]; s.dispose; b.dispose; r",
        "[nil, 8, [0, 0, 0, 255]]",
    ),
    value(
        "a disposed bitmap is kept and draws nothing",
        "b = solid(2, 2, 255, 0, 0); b.dispose; s = Sprite.new; s.x = 101; s.bitmap = b; \
         r = [s.bitmap.equal?(b), s.src_rect.width, pixel(101, 0)]; s.dispose; r",
        "[true, 0, [0, 0, 0, 255]]",
    ),
    value(
        "a bitmap disposed after it was assigned draws nothing",
        "b = solid(2, 2, 255, 0, 0); s = Sprite.new; s.x = 102; s.bitmap = b; \
         r = [pixel(102, 0)]; b.dispose; r << pixel(102, 0) << s.bitmap.disposed?; \
         s.dispose; r",
        "[[255, 0, 0, 255], [0, 0, 0, 255], true]",
    ),
    value(
        "drawing into the bitmap changes the sprite",
        "b = solid(1, 1, 255, 0, 0); s = Sprite.new; s.x = 103; s.bitmap = b; \
         b.set_pixel(0, 0, Color.new(0, 0, 255)); r = pixel(103, 0); s.dispose; b.dispose; r",
        "[0, 0, 255, 255]",
    ),
    raises("no bitmap", "Sprite.new.bitmap = 5", "TypeError"),
];

pub const SPRITE_DISPOSAL: &[Case] = &[
    raises_with(
        "use after dispose",
        "s = Sprite.new; s.dispose; s.x",
        "RGSSError",
        "disposed sprite",
    ),
    raises_with(
        "a writer after dispose",
        r#"s = Sprite.new; s.dispose; s.x = "no number""#,
        "RGSSError",
        "disposed sprite",
    ),
    value(
        "every other method after dispose",
        "s = Sprite.new; s.dispose; \
         calls = [[:viewport], [:bitmap], [:bitmap=, nil], [:src_rect], [:color], \
         [:color=, Color.new(0, 0, 0)], [:tone], [:z], [:z=, 1], [:opacity], [:opacity=, 1], \
         [:flash, nil, 1], [:update], [:zoom_x=, 1], [:visible]]; \
         calls.map { |call| begin; s.send(*call); :none; rescue RGSSError; $!.message; end }.uniq",
        r#"["disposed sprite"]"#,
    ),
    value(
        "disposed? and disposing twice",
        "s = Sprite.new; a = s.disposed?; s.dispose; s.dispose; [a, s.disposed?, s.dispose]",
        "[false, true, nil]",
    ),
    value(
        "bitmap survives",
        "b = Bitmap.new(1, 1); s = Sprite.new; s.bitmap = b; s.dispose; b.disposed?",
        "false",
    ),
    value(
        "not drawn any more",
        "b = solid(1, 1, 255, 0, 0); s = Sprite.new; s.bitmap = b; s.x = 110; \
         r = [pixel(110, 0)]; s.dispose; r << pixel(110, 0); b.dispose; r",
        "[[255, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    raises_with(
        "a sprite that was never initialized",
        "Sprite.allocate.x",
        "RGSSError",
        "disposed sprite",
    ),
    value(
        "a sprite of a disposed viewport lives on and is not drawn",
        "b = solid(1, 1, 255, 0, 0); v = Viewport.new(111, 0, 1, 1); s = Sprite.new(v); \
         s.bitmap = b; r = [pixel(111, 0)]; v.dispose; \
         r << pixel(111, 0) << s.disposed? << s.viewport.equal?(v); s.dispose; b.dispose; r",
        "[[255, 0, 0, 255], [0, 0, 0, 255], false, true]",
    ),
];

pub const SPRITE_FLASH: &[Case] = &[
    value(
        "flash with nil",
        "b = solid(1, 1, 255, 0, 0); s = Sprite.new; s.bitmap = b; s.x = 120; \
         s.flash(nil, 2); r = [pixel(120, 0)]; 2.times { s.update }; r << pixel(120, 0); \
         s.dispose; b.dispose; r",
        "[[0, 0, 0, 255], [255, 0, 0, 255]]",
    ),
    value(
        "flash with a colour",
        "b = solid(1, 1, 255, 0, 0); s = Sprite.new; s.bitmap = b; s.x = 121; \
         s.flash(Color.new(0, 255, 0), 2); r = [pixel(121, 0), s.update]; s.update; \
         r << pixel(121, 0); s.dispose; b.dispose; r",
        "[[0, 255, 0, 255], nil, [255, 0, 0, 255]]",
    ),
    raises(
        "a flash colour of the wrong class",
        "Sprite.new.flash(1, 2)",
        "TypeError",
    ),
];

// --- Plane ------------------------------------------------------------------------------------

pub const PLANE: &[Case] = &[
    value(
        "a plane with a bitmap tiles the screen",
        "b = solid(4, 4, 255, 0, 0); b.set_pixel(0, 0, Color.new(0, 0, 255)); p = Plane.new; \
         p.bitmap = b; \
         r = [pixel(0, 0), pixel(4, 0), pixel(1, 0), pixel(636, 476), pixel(639, 479)]; \
         p.dispose; b.dispose; r << pixel(0, 0)",
        "[[0, 0, 255, 255], [0, 0, 255, 255], [255, 0, 0, 255], [0, 0, 255, 255], \
         [255, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    value(
        "construction",
        "v = Viewport.new(0, 0, 1, 1); p = Plane.new(v); q = Plane.new; \
         r = [p.viewport.equal?(v), q.viewport, q.bitmap, q.disposed?]; \
         p.dispose; q.dispose; v.dispose; r",
        "[true, nil, nil, false]",
    ),
    raises("wrong argument", "Plane.new(1)", "TypeError"),
    value(
        "plain properties",
        "p = Plane.new; d = [p.visible, p.z, p.ox, p.oy, p.zoom_x, p.zoom_y, p.opacity, \
         p.blend_type]; p.visible = false; p.z = 3; p.ox = 4; p.oy = 5; p.zoom_x = 2; \
         p.zoom_y = 0.5; p.opacity = 999; p.blend_type = 2; \
         r = [d, p.visible, p.z, p.ox, p.oy, p.zoom_x, p.zoom_y, p.opacity, p.blend_type]; \
         p.dispose; r",
        "[[true, 0, 0, 0, 1.0, 1.0, 255, 0], false, 3, 4, 5, 2.0, 0.5, 255, 2]",
    ),
    value(
        "the origin scrolls the pattern, inside a viewport",
        "b = solid(4, 4, 255, 0, 0); b.set_pixel(0, 0, Color.new(0, 0, 255)); \
         v = Viewport.new(200, 0, 8, 8); p = Plane.new(v); p.bitmap = b; \
         r = [pixel(200, 0), pixel(199, 0)]; p.ox = 1; r << pixel(200, 0) << pixel(203, 0); \
         p.dispose; v.dispose; b.dispose; r",
        "[[0, 0, 255, 255], [0, 0, 0, 255], [255, 0, 0, 255], [0, 0, 255, 255]]",
    ),
    value(
        "the object properties are the plane's own",
        "b = solid(4, 4, 255, 255, 255); v = Viewport.new(210, 0, 4, 4); p = Plane.new(v); \
         p.bitmap = b; same = [p.color.equal?(p.color), p.tone.equal?(p.tone)]; \
         p.color.set(255, 0, 0); r = [same, pixel(210, 0)]; \
         given = Color.new(0, 255, 0); p.color = given; given.green = 0; \
         r << pixel(210, 0) << p.color.green; p.color.alpha = 0; p.tone.set(-255, 0, 0); \
         r << pixel(210, 0); p.tone = Tone.new(0, -255, 0); r << pixel(210, 0); \
         p.dispose; v.dispose; b.dispose; r",
        "[[true, true], [255, 0, 0, 255], [0, 255, 0, 255], 255.0, [0, 255, 255, 255], \
         [255, 0, 255, 255]]",
    ),
    value(
        "the bitmap is kept by reference and survives the plane",
        "b = Bitmap.new(2, 2); p = Plane.new; p.bitmap = b; same = p.bitmap.equal?(b); \
         p.bitmap = nil; r = [same, p.bitmap]; p.bitmap = b; p.dispose; r << b.disposed?",
        "[true, nil, false]",
    ),
    raises_with(
        "use after dispose",
        "p = Plane.new; p.dispose; p.dispose; p.ox",
        "RGSSError",
        "disposed plane",
    ),
    value(
        "disposed?",
        "p = Plane.new; a = p.disposed?; p.dispose; [a, p.disposed?]",
        "[false, true]",
    ),
    raises_with(
        "an object property after dispose",
        "p = Plane.new; p.dispose; p.color",
        "RGSSError",
        "disposed plane",
    ),
];
