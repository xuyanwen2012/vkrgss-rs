## MODIFIED Requirements

### Requirement: Viewport disposal
In a game of RGSS1 or RGSS2 disposing a viewport SHALL dispose every sprite, plane, window
and tilemap in it, and SHALL leave elements outside it untouched. In a game of RGSS3 it
SHALL dispose none of them: each SHALL stay alive and usable, its `viewport` SHALL return
the disposed `Viewport` object, it SHALL be drawn nowhere while it is in that viewport, and
`Graphics.update` SHALL go on.

#### Scenario: Viewport takes its elements along, measured on the original
- **WHEN** in a game of RGSS1 a sprite, a plane, a window and a tilemap are created in a viewport, a second sprite is created without one, and the viewport is disposed
- **THEN** `disposed?` is `true` for the four, `s.x` raises `RGSSError` with `disposed sprite`, disposing them again raises nothing, and the second sprite is alive

#### Scenario: RGSS3 keeps the elements, measured on the original
- **WHEN** a game of RGSS3 evaluates `v = Viewport.new(0, 0, 50, 50); s = Sprite.new(v); p = Plane.new(v); w = Window.new; w.viewport = v; t = Tilemap.new(v); v.dispose; Graphics.update; s.x = 3` and then `[s.disposed?, p.disposed?, w.disposed?, t.disposed?, s.x, s.viewport.equal?(v), p.viewport.equal?(v), w.viewport.equal?(v), t.viewport.equal?(v)]`
- **THEN** no error is raised and the result is `[false, false, false, false, 3, true, true, true, true]`

#### Scenario: Usable after the viewport, measured on the original
- **WHEN** in a game of RGSS3 the four elements of that viewport get `ox = 4` (the window and the tilemap), `x = 3` (the sprite) and `z = 2` (the plane), the sprite, the window and the tilemap are updated, and the four are then disposed one by one
- **THEN** each assignment and `update` succeed, each `dispose` returns `nil`, and `disposed?` is `true` afterwards for each

#### Scenario: Drawn nowhere
- **WHEN** a game of RGSS3 shows an opaque red sprite at (10, 10) in a viewport (0, 0, 50, 50), disposes the viewport and renders the scene
- **THEN** the pixel (10, 10) is the background and the sprite is not disposed

#### Scenario: Moved out of the disposed viewport
- **WHEN** that sprite then gets `viewport = nil` and the scene is rendered
- **THEN** the pixel (10, 10) is red and `s.viewport` is `nil`

## ADDED Requirements

### Requirement: The scene of a game of RGSS3
The scene of a game of RGSS3 SHALL be made with the rules of RGSS3 of `core-scene`, so that
a turned sprite in a viewport is drawn by "Turned sprites in viewports of RGSS3" and
everything else as in RGSS2.

#### Scenario: A turned sprite in a viewport, measured rule
- **WHEN** a game of RGSS3 makes a viewport with the rectangle (100, 50, 60, 40), `ox = 10` and `oy = 4`, and in it a sprite at (30, 20) with `angle = 180` of an opaque bitmap 16 by 8 whose texel column c is (c, 255 - c, 128), and renders the scene
- **THEN** the pixel (105, 58) has red 15, the pixel (119, 65) red 1, and the pixels (104, 58), (120, 58), (105, 57) and (105, 66) are the background

#### Scenario: The same sprite in RGSS2
- **WHEN** a game of RGSS2 makes the same scene and renders it
- **THEN** the pixel (105, 58) is the background and the pixel (115, 62) has red 15

#### Scenario: The order of RGSS2
- **WHEN** a game of RGSS3 makes an opaque red sprite of 20 by 20 at (0, 9) and then an opaque green sprite of 20 by 20 at (0, 0), both at z 0, and renders the scene
- **THEN** the pixel (5, 12) is red

### Requirement: Viewport of RGSS3
In a game of RGSS3 `Viewport.new` SHALL also take no argument and then have the rectangle
(0, 0, `Graphics.width`, `Graphics.height`) of that moment, which no later resize changes.
One `Rect` SHALL be copied and four values converted to Integers. Any other count, or one
argument that is no `Rect`, SHALL raise `ArgumentError` with the measured message.

#### Scenario: Without arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `r = Viewport.new.rect; [r.x, r.y, r.width, r.height]` at the start, after `Graphics.resize_screen(640, 480)`, after `Graphics.resize_screen(320, 240)`, after `Graphics.resize_screen(200, 300)` and after `Graphics.resize_screen(1, 1)`
- **THEN** the results are `[0, 0, 544, 416]`, `[0, 0, 640, 480]`, `[0, 0, 320, 240]`, `[0, 0, 200, 300]` and `[0, 0, 1, 1]`, and a viewport has z 0, ox and oy 0 and is visible

#### Scenario: Forms, measured on the original
- **WHEN** a game of RGSS3 reads the rectangles of `Viewport.new(Rect.new(3, 4, 5, 6))`, `Viewport.new(1.9, 2.9, 3.9, 4.9)` and `Viewport.new(-5, -6, -7, -8)`, and of the first after its argument was changed to (9, 9, 9, 9)
- **THEN** they are (3, 4, 5, 6), (1, 2, 3, 4), (-5, -6, -7, -8) and (3, 4, 5, 6), and the rectangle read is not the object given

#### Scenario: Wrong arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `Viewport.new(1)`, `Viewport.new(nil)`, `Viewport.new("a")`, `Viewport.new(Viewport.new)`, `Viewport.new(1, 2)`, `Viewport.new(1, 2, 3)`, `Viewport.new(1, 2, 3, 4, 5)`, `Viewport.new(Rect.new(0, 0, 1, 1), 1)` and `Viewport.new(nil, 2, 3, 4)`
- **THEN** the first eight raise `ArgumentError` with `wrong number of arguments (4 for 1)` four times, then `(4 for 2)`, `(4 for 3)`, `(4 for 5)` and `(1 for 2)`, and the last raises `TypeError` with `no implicit conversion from nil to integer`

#### Scenario: Kept across a resize, measured on the original
- **WHEN** a game of RGSS3 makes `v = Viewport.new` at 544 by 416 with an opaque white plane in it and a second white plane without a viewport hidden, calls `Graphics.resize_screen(640, 480)` and takes a snapshot, then shows the second plane and takes another
- **THEN** `v.rect` is still (0, 0, 544, 416); in the first snapshot (543, 415) is white and (544, 0) and (0, 416) are (0, 0, 0, 255); in the second (639, 479) is white

#### Scenario: Disposed viewport, measured on the original
- **WHEN** a game of RGSS3 disposes a viewport and evaluates `rect`, `z`, `z = 1`, `update`, `flash(Color.new(1, 2, 3), 2)`, `dispose` and `disposed?` on it
- **THEN** the first five raise `RGSSError` with `disposed viewport`, `dispose` returns `nil` and `disposed?` raises `NoMethodError`

#### Scenario: Properties, measured on the original
- **WHEN** a game of RGSS3 evaluates, each on a new viewport, `v.z = 5.9`, `v.z = -5.9`, `v.ox = 5.9`, `v.oy = 5.9`, `v.visible = nil`, `v.visible = 5` and reads each back
- **THEN** the values read are 5, -5, 5, 5, `false` and `true`

### Requirement: Plane viewport of RGSS3
In a game of RGSS3 `Plane#viewport` SHALL be a reader without an argument and
`Plane#viewport=` a writer that takes a `Viewport` or `nil`, moves the plane there and
keeps its creation rank, as `Sprite#viewport=` does. Another object SHALL raise `TypeError`
and a disposed viewport `RGSSError`.

#### Scenario: Reader and writer, measured on the original
- **WHEN** a game of RGSS3 evaluates `p = Plane.new; v = Viewport.new(0, 0, 9, 9); [Plane.instance_method(:viewport).arity, Plane.instance_method(:viewport=).arity, p.viewport, (p.viewport = v; p.viewport.equal?(v)), (p.viewport = nil; p.viewport)]`
- **THEN** the result is `[0, 1, nil, true, nil]`

#### Scenario: Wrong arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `p.viewport = 5` and `p.viewport(nil)`
- **THEN** the first raises `TypeError` with `can't convert Fixnum into Viewport` and the second `ArgumentError`

#### Scenario: Seen by the scene, measured on the original
- **WHEN** a game of RGSS3 makes a plane with `ox = 5` of an opaque bitmap 24 by 16 whose texel column c is (c, 255 - c, 128) in a viewport (4, 4, 60, 50), renders, then sets `viewport =` a viewport (70, 20, 80, 90) with `ox = 7` and `oy = 3` and renders again
- **THEN** in the first frame (4, 4) has red 5 and (64, 54) is the background; in the second (4, 4) is the background, (70, 20) has red 12, (149, 109) red 19 and (150, 110) is the background

#### Scenario: Disposed viewport
- **WHEN** a game of RGSS3 evaluates `p.viewport = v` for a disposed viewport `v`, and likewise `s.viewport = v` for a sprite
- **THEN** each raises `RGSSError` with `disposed viewport` and the element keeps the viewport it had

### Requirement: Constructors and object arguments of RGSS3
In a game of RGSS3 `Sprite.new`, `Plane.new` and `Tilemap.new` SHALL take at most one
argument, `nil` or a `Viewport`, with the measured count message, and a value of another
class SHALL raise `TypeError` `can't convert <class> into Viewport`. An argument of
`Sprite`, `Plane` or `Viewport` that must be an object of an RGSS class SHALL raise
`TypeError` worded `can't convert <class> into <class expected>` when it is not.

#### Scenario: Constructors, measured on the original
- **WHEN** a game of RGSS3 evaluates `Sprite.new.viewport`, `Sprite.new(nil).viewport`, `Sprite.new(5)`, `Sprite.new(Viewport.new, 1)`, `Plane.new(5)`, `Plane.new(Viewport.new, 1)` and `Tilemap.new(Viewport.new, 1)`
- **THEN** the first two are `nil`, `Sprite.new(5)` and `Plane.new(5)` raise `TypeError` with `can't convert Fixnum into Viewport`, and the other three raise `ArgumentError` with `wrong number of arguments (1 for 2)`

#### Scenario: Object writers, measured on the original
- **WHEN** a game of RGSS3 evaluates `s.color = nil`, `s.color = 5`, `s.tone = nil`, `s.src_rect = nil`, `s.src_rect = 5`, `s.bitmap = 5` for a sprite `s`, `v.rect = nil`, `v.rect = 5`, `v.color = nil`, `v.tone = nil` for a viewport `v`, and `s.flash(5, 3)` and `v.flash(5, 3)`
- **THEN** each raises `TypeError` with, in order, `can't convert NilClass into Color`, `can't convert Fixnum into Color`, `can't convert NilClass into Tone`, `can't convert NilClass into Rect`, `can't convert Fixnum into Rect`, `can't convert Fixnum into Bitmap`, `can't convert NilClass into Rect`, `can't convert Fixnum into Rect`, `can't convert NilClass into Color`, `can't convert NilClass into Tone`, and twice `can't convert Fixnum into Color`

#### Scenario: Plane writers
- **WHEN** a game of RGSS3 evaluates `p.color = nil`, `p.tone = 5` and `p.bitmap = 5` for a plane `p`
- **THEN** each raises `TypeError` with `can't convert NilClass into Color`, `can't convert Fixnum into Tone` and `can't convert Fixnum into Bitmap`

#### Scenario: Plain values of a sprite, measured on the original
- **WHEN** a game of RGSS3 sets on a sprite and reads back `x = 5.9`, `x = -5.9`, `opacity = 256`, `opacity = -0.5`, `bush_opacity = 300`, `bush_depth = -3`, `wave_amp = 5.7`, `wave_phase = 725.5`, `wave_phase = -30`, `zoom_x = -1.5`, `angle = 725.5`, `mirror = 0`, `visible = nil` and `blend_type = 1.9`
- **THEN** the values read are 5, -5, 255, 0, 300, -3, 5, 5.5, 330.0, -1.5, 725.5, `true`, `false` and 1

#### Scenario: Errors of plain values, measured on the original
- **WHEN** a game of RGSS3 evaluates on a sprite `x = nil`, `x = "3"`, `zoom_x = nil`, `blend_type = 3`, `blend_type = -1` and `flash(Color.new(1, 2, 3), nil)`
- **THEN** they raise `TypeError` with `no implicit conversion from nil to integer`, `TypeError` with `can't convert String into Integer`, `TypeError` with `no implicit conversion to float from nil`, `ArgumentError` with `bad value for blend_type` twice, and `TypeError` with `no implicit conversion from nil to integer`

#### Scenario: Plane values, measured on the original
- **WHEN** a game of RGSS3 reads a new plane's `bitmap`, `viewport`, `z`, `ox`, `oy`, `zoom_x`, `opacity`, `blend_type` and `visible`, then sets and reads `opacity = 300`, `opacity = 77.9`, `ox = -5.9`, `zoom_x = 0` and `blend_type = 1.9`, and evaluates `blend_type = 3`
- **THEN** the defaults are `nil`, `nil`, 0, 0, 0, 1.0, 255, 0 and `true`, the values read are 255, 77, -5, 0.0 and 1, and the last raises `ArgumentError` with `bad value for blend_type`

#### Scenario: Disposed sprite and plane, measured on the original
- **WHEN** a game of RGSS3 disposes a sprite without a bitmap and a plane and evaluates on the sprite `x`, `x = 1`, `update`, `width`, `viewport`, `bitmap`, `dispose` and `disposed?`, and on the plane `ox` and `viewport`
- **THEN** the sprite's first five raise `RGSSError` with `disposed sprite`, `bitmap` and `dispose` return `nil` and `disposed?` `true`, and the plane's two raise `RGSSError` with `disposed plane`

### Requirement: Method surface of the scene classes in RGSS3
In a game of RGSS3 `Sprite`, `Plane`, `Viewport`, `Tilemap` and `Tilemap::BitmapArray` SHALL
define every method of the committed method list of RGSS3 with its arity and no public
instance method beside them, and the test of the list SHALL name them as bound.

#### Scenario: Every listed method exists
- **WHEN** the test of the list of RGSS3 runs after this change
- **THEN** it checks `Sprite`, `Plane`, `Viewport`, `Tilemap`, `Tilemap::BitmapArray` and `Graphics` beside the classes bound before, and passes

#### Scenario: Nothing beside the list
- **WHEN** `public_instance_methods(false)` of `Sprite`, `Plane` and `Viewport` of a game of RGSS3 is compared with the list
- **THEN** no class has a method the list lacks, and `Viewport` has no `disposed?`

### Requirement: The scene classes of RGSS1 and RGSS2 are unchanged by RGSS3
The scene classes of a game of RGSS1 or RGSS2 SHALL NOT change with those of RGSS3: every
test of the two binding packages that passed before SHALL pass under its name with its
expected values unedited, over both interpreters.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkxp-binding`, `vkxp-binding-rgss3` and `vkxp-core` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Plane.new.respond_to?(:viewport=)`, `Viewport.new` without arguments, and `v = Viewport.new(0, 0, 8, 8); s = Sprite.new(v); v.dispose; s.disposed?`
- **THEN** the results are `false`, an `ArgumentError`, and `true`, over both interpreters
