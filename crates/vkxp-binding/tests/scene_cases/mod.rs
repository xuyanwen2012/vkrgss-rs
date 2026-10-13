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
pub(crate) const GROUPS: &[(&str, &[Case])] = &[
    ("viewport", VIEWPORT),
    ("sprite construction and plain properties", SPRITE_PLAIN),
    ("sprite object properties", SPRITE_OBJECTS),
    ("sprite bitmap", SPRITE_BITMAP),
    ("sprite disposal", SPRITE_DISPOSAL),
    ("sprite flash", SPRITE_FLASH),
    ("plane", PLANE),
    ("window", WINDOW),
    ("tilemap", TILEMAP),
    ("no copies", NO_COPIES),
    ("lifetime", LIFETIME),
];

// --- Viewport ---------------------------------------------------------------------------------

pub(crate) const VIEWPORT: &[Case] = &[
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
    value(
        "a disposed viewport given to a constructor",
        "v = Viewport.new(0, 0, 1, 1); v.dispose; n = Sprite.new(v); n.x = 5; \
         o = [Plane.new(v), Window.new(v), Tilemap.new(v)]; \
         r = [n.disposed?, n.x, n.viewport.equal?(v)] + o.map { |e| e.disposed? } + \
         o.map { |e| e.viewport.equal?(v) }; (o << n).each { |e| e.dispose }; r",
        "[false, 5, true, false, false, false, true, true, true]",
    ),
    value(
        "an element created in a disposed viewport draws nothing and outlives another dispose",
        "b = solid(1, 1, 255, 0, 0); v = Viewport.new(71, 70, 1, 1); v.dispose; \
         s = Sprite.new(v); s.bitmap = b; r = [pixel(71, 70)]; v.dispose; \
         r << s.disposed?; s.dispose; b.dispose; r",
        "[[0, 0, 0, 255], false]",
    ),
    value(
        "a viewport takes its elements along",
        "v = Viewport.new(0, 0, 1, 1); s = Sprite.new(v); p = Plane.new(v); w = Window.new(v); \
         t = Tilemap.new(v); other = Sprite.new; v.dispose; \
         r = [s, p, w, t].map { |e| e.disposed? }; \
         r << begin; s.x; rescue RGSSError; $!.message; end; \
         r << begin; s.viewport; rescue RGSSError; $!.message; end; \
         r += [s, p, w, t].map { |e| e.dispose }; r << other.disposed? << other.x; \
         other.dispose; r",
        r#"[true, true, true, true, "disposed sprite", "disposed sprite", nil, nil, nil, nil, false, 0]"#,
    ),
    value(
        "the elements a viewport took along are counted as disposed",
        "v = Viewport.new(0, 0, 1, 1); held = [Sprite.new(v), Plane.new(v), Window.new(v), \
         Tilemap.new(v)]; kinds = [:sprite, :plane, :window, :tilemap]; \
         before = kinds.map { |kind| scene_count(kind) }; v.dispose; \
         r = kinds.zip(before).map { |kind, was| scene_count(kind) < was }; held.clear; r",
        "[true, true, true, true]",
    ),
    raises_with(
        "a viewport that was never initialized",
        "Viewport.allocate.z",
        "RGSSError",
        "disposed viewport",
    ),
];

// --- Sprite -----------------------------------------------------------------------------------

pub(crate) const SPRITE_PLAIN: &[Case] = &[
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

pub(crate) const SPRITE_OBJECTS: &[Case] = &[
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

pub(crate) const SPRITE_BITMAP: &[Case] = &[
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

pub(crate) const SPRITE_DISPOSAL: &[Case] = &[
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
         calls = [[:viewport], [:bitmap=, nil], [:src_rect], [:color], \
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
        "a sprite of a disposed viewport is disposed and not drawn",
        "b = solid(1, 1, 255, 0, 0); v = Viewport.new(111, 0, 1, 1); s = Sprite.new(v); \
         s.bitmap = b; r = [pixel(111, 0)]; v.dispose; \
         r << pixel(111, 0) << s.disposed?; s.dispose; b.dispose; r",
        "[[255, 0, 0, 255], [0, 0, 0, 255], true]",
    ),
    value(
        "the bitmap of a disposed sprite",
        "b = Bitmap.new(1, 1); s = Sprite.new; s.bitmap = b; s.dispose; \
         r = [s.bitmap.equal?(b)]; s.bitmap.dispose; r << b.disposed?",
        "[true, true]",
    ),
    value(
        "the bitmap of a disposed sprite that had none",
        "s = Sprite.new; s.dispose; s.bitmap",
        "nil",
    ),
];

pub(crate) const SPRITE_FLASH: &[Case] = &[
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

pub(crate) const PLANE: &[Case] = &[
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

// --- Window -----------------------------------------------------------------------------------

/// A window skin whose background and frame are blue and whose cursor is white.
macro_rules! skin {
    () => {
        "skin = solid(192, 128, 0, 0, 255); \
         skin.fill_rect(128, 64, 32, 32, Color.new(255, 255, 255)); "
    };
}

pub(crate) const WINDOW: &[Case] = &[
    value(
        "construction and defaults",
        "v = Viewport.new(0, 0, 1, 1); w = Window.new(v); u = Window.new; \
         r = [w.viewport.equal?(v), u.viewport, u.windowskin, u.contents, u.stretch, \
         u.active, u.visible, u.pause, u.x, u.y, u.width, u.height, u.z, u.ox, u.oy, \
         u.opacity, u.back_opacity, u.contents_opacity, u.cursor_rect == Rect.new(0, 0, 0, 0), \
         u.disposed?]; w.dispose; u.dispose; v.dispose; r",
        "[true, nil, nil, nil, true, true, true, false, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, \
         true, false]",
    ),
    raises("wrong argument", "Window.new(:viewport)", "TypeError"),
    value(
        "no openness in RGSS1",
        "w = Window.new; r = [w.respond_to?(:openness), w.respond_to?(:openness=)]; w.dispose; r",
        "[false, false]",
    ),
    value(
        "flags, positions, sizes and opacities",
        "w = Window.new; w.stretch = false; w.active = nil; w.visible = false; w.pause = 1; \
         w.x = 1; w.y = 2; w.width = 3; w.height = 4; w.z = 5; w.ox = 6; w.oy = 7; \
         w.opacity = 300; w.back_opacity = -1; w.contents_opacity = 128; \
         r = [w.stretch, w.active, w.visible, w.pause, w.x, w.y, w.width, w.height, w.z, \
         w.ox, w.oy, w.opacity, w.back_opacity, w.contents_opacity]; w.dispose; r",
        "[false, false, false, true, 1, 2, 3, 4, 5, 6, 7, 255, 0, 128]",
    ),
    value(
        "the contents appear 16 pixels inside the window",
        concat!(
            skin!(),
            "c = solid(32, 32, 255, 0, 0); w = Window.new; w.windowskin = skin; \
             w.x = 300; w.y = 100; w.width = 64; w.height = 64; w.contents = c; \
             r = [pixel(316, 116), pixel(315, 116), pixel(347, 147), pixel(348, 147), \
             pixel(299, 100)]; w.dispose; r << pixel(316, 116); c.dispose; skin.dispose; r"
        ),
        "[[255, 0, 0, 255], [0, 0, 255, 255], [255, 0, 0, 255], [0, 0, 255, 255], \
         [0, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    value(
        "window skin and contents are kept by reference and survive the window",
        "skin = Bitmap.new(192, 128); c = Bitmap.new(32, 32); w = Window.new; \
         w.windowskin = skin; w.contents = c; \
         r = [w.windowskin.equal?(skin), w.contents.equal?(c)]; \
         w.contents = nil; r << w.contents; w.contents = c; w.dispose; \
         r << skin.disposed? << c.disposed?",
        "[true, true, nil, false, false]",
    ),
    raises(
        "no bitmap as the contents",
        "Window.new.contents = 1",
        "TypeError",
    ),
    value(
        "the cursor rectangle is the window's own",
        concat!(
            skin!(),
            "w = Window.new; w.windowskin = skin; w.x = 300; w.y = 200; w.width = 96; \
             w.height = 64; same = w.cursor_rect.equal?(w.cursor_rect); \
             r = [same, pixel(332, 232)]; w.cursor_rect.set(0, 0, 32, 32); r << pixel(332, 232); \
             given = Rect.new(32, 0, 32, 32); w.cursor_rect = given; given.x = 0; \
             r << pixel(332, 232) << pixel(364, 232) << w.cursor_rect.x; \
             w.dispose; skin.dispose; r"
        ),
        "[true, [0, 0, 255, 255], [255, 255, 255, 255], [0, 0, 255, 255], \
         [255, 255, 255, 255], 32]",
    ),
    value(
        "window cursor blinks",
        concat!(
            skin!(),
            "w = Window.new; w.windowskin = skin; w.x = 300; w.y = 300; w.width = 64; \
             w.height = 64; w.cursor_rect.set(0, 0, 32, 32); before = pixel(332, 332); \
             16.times { w.update }; after = pixel(332, 332); w.active = false; \
             16.times { w.update }; r = [before, after, pixel(332, 332)]; \
             w.dispose; skin.dispose; r"
        ),
        // 127 at the sixteenth update; an inactive window stands at 159.
        "[[255, 255, 255, 255], [127, 127, 255, 255], [159, 159, 255, 255]]",
    ),
    value(
        "window active off and on at once",
        concat!(
            skin!(),
            "w = Window.new; w.windowskin = skin; w.x = 300; w.y = 300; w.width = 64; \
             w.height = 64; w.cursor_rect.set(0, 0, 32, 32); 5.times { w.update }; \
             before = pixel(332, 332); w.active = false; w.active = true; \
             r = [before, pixel(332, 332)]; w.update; r << pixel(332, 332); \
             w.dispose; skin.dispose; r"
        ),
        // The blink counter goes from 5 to 12 at the assignment and on from there.
        "[[215, 215, 255, 255], [159, 159, 255, 255], [151, 151, 255, 255]]",
    ),
    value(
        "window pause off and on at once",
        concat!(
            skin!(),
            "skin.fill_rect(160, 64, 32, 32, Color.new(0, 255, 0)); \
             w = Window.new; w.windowskin = skin; w.x = 300; w.y = 300; w.width = 64; \
             w.height = 64; w.pause = true; 8.times { w.update }; before = pixel(332, 356); \
             w.pause = false; w.pause = true; r = [before, pixel(332, 356)]; \
             w.dispose; skin.dispose; r"
        ),
        // The sign starts over: it is gone until the next update.
        "[[0, 255, 0, 255], [0, 0, 255, 255]]",
    ),
    raises_with(
        "use after dispose",
        "w = Window.new; w.dispose; w.dispose; w.update",
        "RGSSError",
        "disposed window",
    ),
    raises_with(
        "an object property after dispose",
        "w = Window.new; w.dispose; w.cursor_rect",
        "RGSSError",
        "disposed window",
    ),
    value(
        "disposed?",
        "w = Window.new; a = w.disposed?; w.dispose; [a, w.disposed?]",
        "[false, true]",
    ),
];

// --- Tilemap ----------------------------------------------------------------------------------

/// A tileset whose second tile (identifier 385) is green, and a tilemap `t` with it and an
/// empty map of 20 by 15 cells in three layers.
macro_rules! map {
    () => {
        "set = Bitmap.new(256, 32); set.fill_rect(32, 0, 32, 32, Color.new(0, 255, 0)); \
         data = Table.new(20, 15, 3); t = Tilemap.new; t.tileset = set; t.map_data = data; "
    };
}

pub(crate) const TILEMAP: &[Case] = &[
    value(
        "construction and defaults",
        "v = Viewport.new(0, 0, 1, 1); t = Tilemap.new(v); u = Tilemap.new; \
         r = [t.viewport.equal?(v), u.viewport, u.tileset, u.map_data, u.flash_data, \
         u.priorities, u.visible, u.ox, u.oy, u.disposed?, u.autotiles.class, u.update]; \
         t.dispose; u.dispose; v.dispose; r",
        "[true, nil, nil, nil, nil, nil, true, 0, 0, false, TilemapAutotiles, nil]",
    ),
    raises("wrong argument", "Tilemap.new(0)", "TypeError"),
    value(
        "plain properties",
        "t = Tilemap.new; t.visible = false; t.ox = 33; t.oy = -2; \
         r = [t.visible, t.ox, t.oy]; t.dispose; r",
        "[false, 33, -2]",
    ),
    value(
        "a tileset tile appears at its cell",
        concat!(
            map!(),
            "data[2, 1, 0] = 385; \
             r = [pixel(64, 32), pixel(95, 63), pixel(63, 32), pixel(96, 32), pixel(64, 64)]; \
             t.ox = 32; r << pixel(32, 32) << pixel(64, 32); t.visible = false; \
             r << pixel(32, 32); t.dispose; set.dispose; r"
        ),
        "[[0, 255, 0, 255], [0, 255, 0, 255], [0, 0, 0, 255], [0, 0, 0, 255], \
         [0, 0, 0, 255], [0, 255, 0, 255], [0, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    value(
        "table changes reach the tilemap",
        concat!(
            map!(),
            "r = [t.map_data.equal?(data), pixel(96, 32)]; data[3, 1, 0] = 385; \
             r << pixel(96, 32); t.map_data[3, 1, 0] = 0; r << pixel(96, 32); \
             t.dispose; set.dispose; r"
        ),
        "[true, [0, 0, 0, 255], [0, 255, 0, 255], [0, 0, 0, 255]]",
    ),
    value(
        "a copy of a table is not connected",
        concat!(
            map!(),
            "copy = data.dup; copy[3, 1, 0] = 385; r = [pixel(96, 32), data[3, 1, 0]]; \
             t.map_data = nil; data[3, 1, 0] = 385; r << t.map_data << pixel(96, 32); \
             t.dispose; set.dispose; r"
        ),
        "[[0, 0, 0, 255], 0, nil, [0, 0, 0, 255]]",
    ),
    value(
        "tables and tileset are kept by reference",
        "t = Tilemap.new; set = Bitmap.new(256, 32); flash = Table.new(2, 2); \
         priorities = Table.new(400); t.tileset = set; t.flash_data = flash; \
         t.priorities = priorities; \
         r = [t.tileset.equal?(set), t.flash_data.equal?(flash), \
         t.priorities.equal?(priorities)]; t.tileset = nil; t.flash_data = nil; \
         t.priorities = nil; r << t.tileset << t.flash_data << t.priorities; \
         t.tileset = set; t.dispose; r << set.disposed?",
        "[true, true, true, nil, nil, nil, false]",
    ),
    raises(
        "no table as the map data",
        "Tilemap.new.map_data = [1, 2]",
        "TypeError",
    ),
    raises(
        "no bitmap as the tileset",
        "Tilemap.new.tileset = Table.new(1)",
        "TypeError",
    ),
    value(
        "autotile list",
        "t = Tilemap.new; b = Bitmap.new(96, 128); t.autotiles[3] = b; \
         r = [t.autotiles[3].equal?(b), t.autotiles[0], t.autotiles.equal?(t.autotiles)]; \
         t.dispose; r",
        "[true, nil, true]",
    ),
    value(
        "an autotile index outside 0 to 6 reads nil and writes nothing",
        "t = Tilemap.new; b = Bitmap.new(96, 128); a = t.autotiles; a[7] = b; a[-1] = b; \
         r = [a[7], a[-1], (0..6).map { |i| a[i] }.compact.size]; a[6] = b; a[6] = nil; \
         r << a[6]; t.dispose; r",
        "[nil, nil, 0, nil]",
    ),
    raises(
        "no bitmap as an autotile",
        "Tilemap.new.autotiles[0] = 1",
        "TypeError",
    ),
    value(
        "an autotile appears at its cell",
        concat!(
            map!(),
            "auto = solid(96, 128, 0, 0, 255); t.autotiles[0] = auto; data[5, 5, 0] = 48; \
             r = [pixel(160, 160), pixel(191, 191), pixel(159, 160)]; \
             t.autotiles[0] = nil; r << pixel(160, 160); \
             t.dispose; set.dispose; auto.dispose; r"
        ),
        "[[0, 0, 255, 255], [0, 0, 255, 255], [0, 0, 0, 255], [0, 0, 0, 255]]",
    ),
    value(
        "the autotile list outlives its tilemap's disposal",
        "t = Tilemap.new; a = t.autotiles; b = Bitmap.new(96, 128); a[0] = b; t.dispose; \
         a[1] = b; [a[0].equal?(b), a[1].equal?(b), b.disposed?]",
        "[true, true, false]",
    ),
    raises_with(
        "use after dispose",
        "t = Tilemap.new; t.dispose; t.dispose; t.ox",
        "RGSSError",
        "disposed tilemap",
    ),
    value(
        "the autotiles of a disposed tilemap",
        "t = Tilemap.new; a = t.autotiles; t.dispose; [t.autotiles.equal?(a), a.class]",
        "[true, TilemapAutotiles]",
    ),
    raises_with(
        "update after dispose",
        "t = Tilemap.new; t.dispose; t.update",
        "RGSSError",
        "disposed tilemap",
    ),
    value(
        "disposed? and pixels gone",
        concat!(
            map!(),
            "data[2, 1, 0] = 385; r = [t.disposed?, pixel(64, 32)]; t.dispose; \
             r << t.disposed? << pixel(64, 32); set.dispose; r"
        ),
        "[false, [0, 255, 0, 255], true, [0, 0, 0, 255]]",
    ),
];

// --- Copies and collection --------------------------------------------------------------------

pub(crate) const NO_COPIES: &[Case] = &[
    raises_with(
        "clone",
        "Sprite.new.clone",
        "TypeError",
        "can't clone Sprite",
    ),
    raises_with("dup", "Sprite.new.dup", "TypeError", "can't dup Sprite"),
    value(
        "every element class refuses both",
        "v = Viewport.new(0, 0, 1, 1); \
         r = [Sprite.new, v, Window.new, Plane.new, Tilemap.new].map { |e| \
         [:clone, :dup].map { |copy| \
         begin; e.send(copy); :copied; rescue TypeError; $!.message; end } }; v.dispose; r",
        r#"[["can't clone Sprite", "can't dup Sprite"], ["can't clone Viewport", "can't dup Viewport"], ["can't clone Window", "can't dup Window"], ["can't clone Plane", "can't dup Plane"], ["can't clone Tilemap", "can't dup Tilemap"]]"#,
    ),
    raises_with(
        "a subclass is named in the message",
        "class CaseCopySprite < Sprite; end; CaseCopySprite.new.clone",
        "TypeError",
        "can't clone CaseCopySprite",
    ),
    raises(
        "a copy that does not come through clone or dup",
        "Sprite.allocate.send(:initialize_copy, Sprite.new)",
        "TypeError",
    ),
    value(
        "the refused copy leaves the element as it was",
        "s = Sprite.new; s.x = 4; begin; s.clone; rescue TypeError; end; [s.x, s.disposed?]",
        "[4, false]",
    ),
];

pub(crate) const LIFETIME: &[Case] = &[
    value(
        "forgotten sprites",
        "1000.times { |i| Sprite.new; GC.start if i % 100 == 99 }; GC.start; \
         scene_count(:sprite) < 500",
        "true",
    ),
    value(
        "forgotten elements of the other classes",
        "300.times { |i| Viewport.new(0, 0, 1, 1); Window.new; Plane.new; Tilemap.new; \
         GC.start if i % 100 == 99 }; GC.start; \
         [:viewport, :window, :plane, :tilemap].map { |kind| scene_count(kind) < 150 }",
        "[true, true, true, true]",
    ),
    value(
        "kept sprite",
        "b = solid(1, 1, 255, 0, 0); $case_kept = Sprite.new; $case_kept.bitmap = b; \
         $case_kept.x = 130; 3.times { GC.start; 100.times { Sprite.new } }; \
         r = [pixel(130, 0), $case_kept.disposed?]; $case_kept.dispose; $case_kept = nil; \
         b.dispose; r",
        "[[255, 0, 0, 255], false]",
    ),
    value(
        "a bitmap kept alive only by a sprite",
        "s = Sprite.new; s.x = 131; s.bitmap = solid(1, 1, 255, 0, 0); \
         3.times { GC.start; 100.times { Bitmap.new(1, 1) } }; \
         r = [pixel(131, 0), s.bitmap.disposed?]; s.bitmap.dispose; s.dispose; r",
        "[[255, 0, 0, 255], false]",
    ),
    value(
        "a viewport kept alive only by its sprite",
        "s = Sprite.new(Viewport.new(132, 0, 1, 1)); s.bitmap = solid(1, 1, 255, 0, 0); \
         3.times { GC.start; 100.times { Viewport.new(0, 0, 1, 1) } }; \
         r = [pixel(132, 0), s.viewport.rect.x]; s.bitmap.dispose; s.viewport.dispose; \
         s.dispose; r",
        "[[255, 0, 0, 255], 132]",
    ),
    value(
        "the objects of a window and a tilemap kept alive only by them",
        "w = Window.new; w.windowskin = Bitmap.new(192, 128); w.contents = Bitmap.new(8, 8); \
         t = Tilemap.new; t.tileset = Bitmap.new(256, 32); t.map_data = Table.new(2, 2, 3); \
         t.autotiles[2] = Bitmap.new(96, 128); \
         3.times { GC.start; 100.times { Bitmap.new(1, 1); Table.new(1) } }; \
         t.map_data[1, 1, 2] = 7; \
         r = [w.windowskin.disposed?, w.contents.width, t.tileset.disposed?, \
         t.map_data[1, 1, 2], t.autotiles[2].height]; w.dispose; t.dispose; r",
        "[false, 8, false, 7, 128]",
    ),
    value(
        "an autotile list keeps its tilemap",
        "a = Tilemap.new.autotiles; b = Bitmap.new(96, 128); \
         3.times { GC.start; 100.times { Tilemap.new } }; a[0] = b; a[0].equal?(b)",
        "true",
    ),
    value(
        "an object property kept after its sprite was collected",
        "c = Sprite.new.color; 3.times { GC.start; 100.times { Sprite.new } }; \
         c.set(1, 2, 3, 4); [c.red, c.alpha]",
        "[1.0, 4.0]",
    ),
];
