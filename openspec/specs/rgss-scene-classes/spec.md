# rgss-scene-classes Specification

## Purpose
Defines the Ruby classes `Sprite`, `Viewport`, `Window`, `Plane` and `Tilemap` as scripts see
them: construction, properties, the objects those properties hand out, disposal and garbage
collection, on top of the scene that `core-scene` defines.

## Requirements

### Requirement: Method surface of the scene classes
`Sprite`, `Viewport`, `Window`, `Plane`, `Tilemap` and `TilemapAutotiles` SHALL define every
method the original runtime defines for them with the same arity, as listed in the committed
method list of the run's runtime, and no `disposed?` method on `Viewport`, which the original
does not have. For RGSS2 the test MAY name methods that are not implemented yet, and SHALL
fail when a method it names as missing exists.

#### Scenario: Every listed method exists
- **WHEN** each entry of the method list for these classes is checked with `instance_method`
- **THEN** the method exists and its arity equals the listed arity

#### Scenario: Whole list covered
- **WHEN** the arity test runs after this change
- **THEN** it checks every entry of the method list, for all fifteen classes and modules

#### Scenario: List of RGSS2
- **WHEN** the arity test runs in a game of RGSS2
- **THEN** every entry of the list of RGSS2 for the scene classes exists with its arity, except the entries the test names as pending, none of which exists

### Requirement: Construction
`Sprite.new`, `Plane.new`, `Window.new` and `Tilemap.new` SHALL take an optional viewport, and
`Viewport.new` SHALL take either x, y, width and height or a `Rect`. Each SHALL create the
element in the scene with the defaults of `core-scene`. An argument of the wrong class SHALL
raise `TypeError`. A disposed viewport SHALL be accepted: the new element is alive, returns
that viewport, and draws nothing.

#### Scenario: Sprite in a viewport
- **WHEN** `v = Viewport.new(0, 0, 100, 100); s = Sprite.new(v)` is evaluated
- **THEN** `s.viewport.equal?(v)` is `true`

#### Scenario: Viewport from a rectangle
- **WHEN** `Viewport.new(Rect.new(1, 2, 3, 4)).rect` is evaluated
- **THEN** the result equals `Rect.new(1, 2, 3, 4)`

#### Scenario: Wrong argument
- **WHEN** `Sprite.new("viewport")` is evaluated
- **THEN** a `TypeError` is raised

#### Scenario: Disposed viewport, measured on the original
- **WHEN** `v.dispose; n = Sprite.new(v); n.x = 5` is evaluated, and likewise `Plane.new(v)`, `Window.new(v)` and `Tilemap.new(v)`
- **THEN** no error is raised, `n.disposed?` is `false`, `n.x` is 5, `n.viewport.equal?(v)` is `true`, and the other three are not disposed either

### Requirement: Plain properties
Each numeric and boolean property of the five classes SHALL have a reader and a writer that
get and set the property of the scene element, with the conversions of `rgss-values`
(Integers for coordinates and opacities, Floats for zoom and angle, truthiness for flags).

#### Scenario: Round trip
- **WHEN** `s = Sprite.new; s.x = 10; s.zoom_x = 1.5; s.mirror = true; s.opacity = 300` is evaluated
- **THEN** `[s.x, s.zoom_x, s.mirror, s.opacity]` is `[10, 1.5, true, 255]`

#### Scenario: Seen by the scene
- **WHEN** a sprite with an opaque 1 by 1 red bitmap is moved to x 5, y 6 and the scene is rendered
- **THEN** the pixel at (5, 6) is red

### Requirement: Object properties are the element's own
The readers `color`, `tone` and `src_rect` of a sprite, `color`, `tone` and `rect` of a
viewport, `color` and `tone` of a plane and `cursor_rect` of a window SHALL return the same
object on every call, and changing that object SHALL change the element. The writers SHALL
copy the values of the given object into it, leaving the given object unconnected.

#### Scenario: Changing in place
- **WHEN** `s.color.set(255, 0, 0, 255)` is evaluated for a sprite with an opaque bitmap and the scene is rendered
- **THEN** the sprite's pixels are red

#### Scenario: Identity
- **WHEN** `s.src_rect.equal?(s.src_rect)` is evaluated
- **THEN** the result is `true`

#### Scenario: Assignment copies
- **WHEN** `c = Color.new(0, 255, 0); s.color = c; c.green = 0` is evaluated
- **THEN** `s.color.green` is `255.0`

### Requirement: Bitmaps and tables are kept by reference
The properties `bitmap` of a sprite and a plane, `windowskin` and `contents` of a window,
`tileset` and the seven autotiles of a tilemap, and a tilemap's `map_data`, `flash_data` and
`priorities` SHALL return the very object that was assigned, or `nil`. An assigned object SHALL
stay alive as long as the element refers to it. Assigning a bitmap to a sprite SHALL set its
`src_rect` to the bitmap's rectangle.

#### Scenario: Same object back
- **WHEN** `b = Bitmap.new(8, 8); s.bitmap = b` is evaluated
- **THEN** `s.bitmap.equal?(b)` is `true` and `s.src_rect` equals `Rect.new(0, 0, 8, 8)`

#### Scenario: Kept alive
- **WHEN** a bitmap is assigned to a sprite, no other reference to the bitmap is kept, and garbage collections are forced
- **THEN** the sprite still draws the bitmap

#### Scenario: Autotile list
- **WHEN** `t = Tilemap.new; b = Bitmap.new(96, 128); t.autotiles[3] = b` is evaluated
- **THEN** `t.autotiles[3].equal?(b)` is `true` and `t.autotiles[0]` is `nil`

#### Scenario: Table changes reach the tilemap
- **WHEN** a table is assigned as `map_data`, an element is then changed through the table object, and the scene is rendered
- **THEN** the frame shows the changed tile

### Requirement: Disposal
`dispose` SHALL remove the element from the scene and `disposed?`, where the class has it,
SHALL report it. Any other method of a disposed element SHALL raise `RGSSError` with the
message `disposed sprite`, `disposed viewport`, `disposed window`, `disposed plane` or
`disposed tilemap`, except `Sprite#bitmap` and `Tilemap#autotiles`, which SHALL keep returning
their object. Disposing a sprite, plane or window SHALL NOT dispose its bitmaps.

#### Scenario: Use after dispose
- **WHEN** `s = Sprite.new; s.dispose; s.x` is evaluated
- **THEN** an `RGSSError` with the message `disposed sprite` is raised

#### Scenario: Bitmap survives
- **WHEN** a sprite with a bitmap is disposed
- **THEN** the bitmap's `disposed?` is `false`

#### Scenario: Not drawn any more
- **WHEN** a visible sprite is disposed and the scene is rendered
- **THEN** its pixels are gone

#### Scenario: Bitmap of a disposed sprite, measured on the original
- **WHEN** `s.bitmap = b; s.dispose; s.bitmap.dispose` is evaluated
- **THEN** no error is raised and `b.disposed?` is `true`

#### Scenario: Autotiles of a disposed tilemap
- **WHEN** `t = Tilemap.new; t.dispose; t.autotiles` is evaluated
- **THEN** the autotile list is returned

### Requirement: Viewport disposal
Disposing a viewport SHALL dispose every sprite, plane, window and tilemap created in it, and
SHALL leave elements outside it untouched.

#### Scenario: Viewport takes its elements along, measured on the original
- **WHEN** a sprite, a plane, a window and a tilemap are created in a viewport, a second sprite is created without one, and the viewport is disposed
- **THEN** `disposed?` is `true` for the four, `s.x` raises `RGSSError` with `disposed sprite`, disposing them again raises nothing, and the second sprite is alive

### Requirement: No copies
`clone` and `dup` of a sprite, viewport, window, plane or tilemap SHALL raise `TypeError`.

#### Scenario: Clone
- **WHEN** `Sprite.new.clone` is evaluated
- **THEN** a `TypeError` is raised

### Requirement: Unreachable elements are released
A scene element whose Ruby object has become unreachable SHALL be removed from the scene by the
garbage collector as if it had been disposed, and an element that scripts still reach SHALL
never be removed that way.

#### Scenario: Forgotten sprites
- **WHEN** a thousand sprites are created without being kept or disposed and garbage collections are forced
- **THEN** the scene ends with far fewer than a thousand sprites

#### Scenario: Kept sprite
- **WHEN** a sprite is kept in a global variable through forced garbage collections
- **THEN** it is still drawn

### Requirement: Updates and flashes
`update` of a sprite, viewport, window and tilemap SHALL advance what `core-scene` says an
update advances. `flash(color, duration)` of a sprite and a viewport SHALL start a flash, with
`nil` as the colour meaning the colourless flash.

#### Scenario: Window cursor blinks
- **WHEN** an active window with a cursor rectangle is updated 16 times and the scene is rendered before and after
- **THEN** the cursor's pixels differ between the two frames

#### Scenario: Flash with nil
- **WHEN** `s.flash(nil, 2)` is evaluated for a visible sprite and the scene is rendered
- **THEN** the sprite is not drawn

### Requirement: Moving an element to another viewport
In RGSS2 `Sprite#viewport=`, `Window#viewport=`, `Tilemap#viewport=` and `Plane#viewport(v)`
SHALL move the element to the given viewport, or out of any for `nil`, and the readers of the
first three SHALL return the object given. Another object SHALL raise `TypeError`, and a
disposed viewport `RGSSError`. `Plane` SHALL have no reader and no `viewport=`. In RGSS1 none
of the writers SHALL be defined and `Plane#viewport` SHALL stay the reader.

#### Scenario: Plane, measured on the original
- **WHEN** a game of RGSS2 shows a red plane, evaluates `plane.viewport(Viewport.new(0, 0, 50, 50))` and takes a frame, then `plane.viewport(nil)` and another
- **THEN** the first frame is red at (10, 10) and black at (100, 100), and the second is red at both

#### Scenario: Seen by the scene
- **WHEN** a sprite with an opaque red bitmap at x 0, y 0 is moved into a viewport whose rectangle starts at (20, 30)
- **THEN** the next frame is red at (20, 30) and not at (0, 0)

#### Scenario: Readers and errors, measured on the original
- **WHEN** a game of RGSS2 evaluates `s.viewport = v; s.viewport.equal?(v)`, `s.viewport = nil; s.viewport`, `s.viewport = 5`, and `plane.viewport`
- **THEN** the results are `true`, `nil`, a `TypeError` with the message `can't convert Fixnum into Viewport`, and an `ArgumentError`

#### Scenario: Disposed
- **WHEN** a game of RGSS2 evaluates `s.viewport = v` for a disposed `v`, and `d.viewport = nil` for a disposed sprite `d`
- **THEN** both raise `RGSSError`

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Sprite.new.respond_to?(:viewport=)` and `Plane.new.viewport`
- **THEN** the results are `false` and `nil`

### Requirement: Small differences of RGSS2
In RGSS2 `Sprite#width` and `Sprite#height` SHALL return the width and height of `src_rect`;
`Plane#visible` SHALL return `true` whatever was set, while the plane is hidden as set; a new
window SHALL have a contents bitmap of 1 by 1 that stays alive when the window is disposed;
and `Window#stretch` and the constant `Hangup` SHALL NOT be defined. In RGSS1 all of these
SHALL stay as they are.

#### Scenario: Size of a sprite, measured on the original
- **WHEN** a game of RGSS2 evaluates `s = Sprite.new; [s.width, s.height]`, and again after `s.src_rect = Rect.new(1, 2, 100, -12)`
- **THEN** the results are `[0, 0]` and `[100, -12]`

#### Scenario: Plane that is hidden, measured on the original
- **WHEN** a game of RGSS2 shows a red plane, evaluates `plane.visible = false; plane.visible` and takes a frame
- **THEN** the value is `true` and the frame is black

#### Scenario: Contents of a new window, measured on the original
- **WHEN** a game of RGSS2 evaluates `w = Window.new; c = w.contents; [c.width, c.height, c.equal?(w.contents)]`, then `w.dispose; c.disposed?`
- **THEN** the results are `[1, 1, true]` and `false`

#### Scenario: Names that RGSS2 lacks, measured on the original
- **WHEN** a game of RGSS2 evaluates `Window.new.respond_to?(:stretch)` and `Object.const_defined?(:Hangup)`
- **THEN** both are `false`

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Window.new.contents`, `Sprite.new.respond_to?(:width)` and `Object.const_defined?(:Hangup)`
- **THEN** the results are `nil`, `false` and `true`

### Requirement: Openness
In RGSS2 `Window#openness` SHALL default to 255 and `openness=` SHALL take an integer, a Float
being truncated, clamped to 0 to 255; on a disposed window both SHALL raise `RGSSError`. In
RGSS1 neither SHALL be defined.

#### Scenario: Limits, measured on the original
- **WHEN** a game of RGSS2 sets `openness` to -1, 256, 100000 and -1.7, reading it after each
- **THEN** the values are 0, 255, 255 and 0

#### Scenario: Errors, measured on the original
- **WHEN** a game of RGSS2 evaluates `w.openness = nil` and, on a disposed window, `w.openness`
- **THEN** a `TypeError` and an `RGSSError` are raised

#### Scenario: Not pending any more
- **WHEN** the method test of RGSS2 runs
- **THEN** `Window#openness` and `openness=` are checked and are not in its pending list

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Window.new.respond_to?(:openness)`
- **THEN** the result is `false`
