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

### Requirement: Tilemap class of RGSS2
In RGSS2 `Tilemap` SHALL define `bitmaps`, which returns one `Tilemap::BitmapArray` of nine
slots that keep `nil` or a `Bitmap` by reference, and `passages` and `passages=` for a
`Table` kept by reference, and SHALL NOT define `tileset`, `autotiles` or `priorities`;
`TilemapAutotiles` SHALL NOT be defined. In RGSS1 the class SHALL stay as it is.

#### Scenario: Slots, measured on the original
- **WHEN** a game of RGSS2 evaluates `t.bitmaps[9] = b; t.bitmaps[9]`, `t.bitmaps[-1]`, `t.bitmaps[1] = b; t.bitmaps[1].equal?(b)` and `t.bitmaps.equal?(t.bitmaps)`
- **THEN** the results are `nil`, `nil`, `true` and `true`

#### Scenario: Tables, measured on the original
- **WHEN** a game of RGSS2 sets `t.map_data = table` and then `t.map_data = nil`, and likewise `passages`, and `t.flash_data = table; t.flash_data = nil`
- **THEN** the first two raise `TypeError` and keep the table, and the third succeeds

#### Scenario: Disposed, measured on the original
- **WHEN** a game of RGSS2 disposes a tilemap and evaluates `t.bitmaps[1] = b; t.bitmaps[1].equal?(b)` and `t.passages`
- **THEN** the first is `true` and the second raises `RGSSError`

#### Scenario: Names of RGSS1, measured on the original
- **WHEN** a game of RGSS2 evaluates `Tilemap.new.respond_to?(:tileset)` and `Object.const_defined?(:TilemapAutotiles)`
- **THEN** both are `false`, and the method test of RGSS2 names none of the tilemap's methods as pending

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Tilemap.new.respond_to?(:bitmaps)`
- **THEN** the result is `false`

### Requirement: Sprite properties of RGSS2
In RGSS2 `Sprite` SHALL define `bush_opacity` (default 128), `wave_amp` (0), `wave_length`
(180) and `wave_speed` (360) as integers kept as written without a limit, and `wave_phase`
(0.0) as a Float kept modulo 360 in [0, 360); `bitmap=` SHALL reset `src_rect` only for
another bitmap object. In RGSS1 none of the five SHALL be defined.

#### Scenario: Kept as written, measured on the original
- **WHEN** a game of RGSS2 sets `bush_opacity = 300`, `wave_amp = -1000` and `wave_length = 0`, reading each
- **THEN** the values are 300, -1000 and 0

#### Scenario: Phase, measured on the original
- **WHEN** a game of RGSS2 sets `wave_phase` to 370.5, -1 and 1000, reading it after each
- **THEN** the values are 10.5, 359.0 and 280.0

#### Scenario: Same bitmap, measured on the original
- **WHEN** a game of RGSS2 evaluates `s.bitmap = b; s.src_rect.set(1, 2, 3, 4); s.bitmap = b; s.src_rect`
- **THEN** the rectangle is still (1, 2, 3, 4)

#### Scenario: Not pending any more
- **WHEN** the method test of RGSS2 runs
- **THEN** the five properties are checked and are not in its pending list

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Sprite.new.respond_to?(:wave_amp)`
- **THEN** the result is `false`

### Requirement: Tilemap class of RGSS3
In RGSS3 `Tilemap` SHALL define the methods the original runtime of VX Ace defines, with the
same arities: those of the class of RGSS2 with `flags` and `flags=` in the place of
`passages` and `passages=`. `bitmaps` SHALL return one `Tilemap::BitmapArray` of nine slots
that keep `nil` or a `Bitmap` by reference. In RGSS1 and RGSS2 `flags` SHALL NOT be defined
and the class SHALL stay as it is.

#### Scenario: Methods, measured on the original
- **WHEN** a game of RGSS3 evaluates `Tilemap.instance_methods(false).sort.map { |m| [m, Tilemap.instance_method(m).arity] }`
- **THEN** the result is `[[:bitmaps, 0], [:clone, 0], [:dispose, 0], [:disposed?, 0], [:dup, 0], [:flags, 0], [:flags=, 1], [:flash_data, 0], [:flash_data=, 1], [:map_data, 0], [:map_data=, 1], [:ox, 0], [:ox=, 1], [:oy, 0], [:oy=, 1], [:update, 0], [:viewport, 0], [:viewport=, 1], [:visible, 0], [:visible=, 1]]`

#### Scenario: Constructor and array, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Tilemap.instance_method(:initialize).arity, Tilemap::BitmapArray.instance_methods(false).sort.map { |m| [m, Tilemap::BitmapArray.instance_method(m).arity] }]`
- **THEN** the result is `[-1, [[:[], 1], [:[]=, 2]]]`

#### Scenario: A new tilemap, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = Tilemap.new; [t.map_data, t.flags, t.flash_data, t.viewport, t.ox, t.oy, t.visible, (0..9).map { |i| t.bitmaps[i] }.compact, t.bitmaps.class, t.bitmaps.equal?(t.bitmaps)]`
- **THEN** the result is `[nil, nil, nil, nil, 0, 0, true, [], Tilemap::BitmapArray, true]`

#### Scenario: Names of the other runtimes, measured on the original
- **WHEN** a game of RGSS3 evaluates `[:passages, :passages=, :tileset, :autotiles, :priorities].map { |m| Tilemap.new.respond_to?(m) }` and `Object.const_defined?(:TilemapAutotiles)`
- **THEN** the results are five times `false`, and `false`

#### Scenario: Viewport, measured on the original
- **WHEN** a game of RGSS3 evaluates `v = Viewport.new(1, 2, 3, 4); t = Tilemap.new(v); [t.viewport.equal?(v), Tilemap.new(nil).viewport, (t.viewport = nil), t.viewport]`
- **THEN** the result is `[true, nil, nil, nil]`

#### Scenario: Slots, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.bitmaps[9] = b; t.bitmaps[-1] = b; [t.bitmaps[9], t.bitmaps[-1], (t.bitmaps[3] = b; t.bitmaps[3].equal?(b)), (t.bitmaps[3] = nil; t.bitmaps[3])]` for a tilemap `t` and a bitmap `b`
- **THEN** the result is `[nil, nil, true, nil]`

#### Scenario: Wrong arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.bitmaps[3] = 5`, `t.bitmaps["a"]`, `t.viewport = 5`, `Tilemap.new(5)`, `Tilemap.new(Viewport.new(0, 0, 8, 8), 1)`, `t.ox = "3"`, `t.ox = nil` and `t.ox = 2**31`
- **THEN** they raise `TypeError`, `TypeError`, `TypeError`, `TypeError`, `ArgumentError`, `TypeError`, `TypeError` and `RangeError`

#### Scenario: Plain properties, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.ox = 1.9; t.oy = -1.9; a = [t.ox, t.oy]; t.ox = 2**30 - 1; t.oy = 1e3; t.visible = nil; b = t.visible; t.visible = 5; a + [t.ox, t.oy, b, t.visible, t.update]`
- **THEN** the result is `[1, -1, 1073741823, 1000, false, true, nil]`

#### Scenario: No copies and no dump, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.dup`, `t.clone` and `Marshal.dump(t)`
- **THEN** each raises `TypeError`, the first two with the messages `can't dup Tilemap` and `can't clone Tilemap`

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Tilemap.new.respond_to?(:flags)`, and the game of RGSS2 `Tilemap.new.respond_to?(:passages)`
- **THEN** the results are `false`, `false` and `true`

### Requirement: Tables of a tilemap of RGSS3
In RGSS3 `map_data=` and `flags=` SHALL take a `Table` of any dimensions and keep it by
reference, SHALL accept `nil` while the property never held a table, and once it holds one
SHALL raise `TypeError` with the message `can't convert NilClass into Table` for `nil` and
keep the table. `flash_data=` SHALL always accept `nil`. A value that is neither a `Table`
nor `nil` SHALL raise `TypeError`.

#### Scenario: Nil on a fresh tilemap, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = Tilemap.new; t.flags = nil; t.map_data = nil; t.flash_data = nil; [t.flags, t.map_data, t.flash_data]`
- **THEN** no error is raised and the result is `[nil, nil, nil]`

#### Scenario: Nil once a table is set, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.flags = f; t.map_data = m` for two tables and then `t.flags = nil` and `t.map_data = nil`
- **THEN** each of the two raises `TypeError` with the message `can't convert NilClass into Table`, and `[t.flags.equal?(f), t.map_data.equal?(m)]` is `[true, true]`

#### Scenario: Flash data, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.flash_data = Table.new(2, 2); t.flash_data = nil; t.flash_data`
- **THEN** no error is raised and the result is `nil`

#### Scenario: Any dimensions, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.flags = Table.new(4, 4); t.map_data = Table.new(3); [t.flags.ysize, t.map_data.xsize]`
- **THEN** no error is raised and the result is `[4, 3]`

#### Scenario: Other values, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.flags = 5`, `t.map_data = 5` and `t.flash_data = "a"`
- **THEN** each raises `TypeError`

#### Scenario: Kept alive
- **WHEN** a game of RGSS3 assigns a new table to `flags` of a tilemap it keeps, keeps no other reference to the table, forces garbage collections and evaluates `t.flags[1]`
- **THEN** the value is the one the table was given before the collections

### Requirement: A tilemap of RGSS3 is drawn by the rules of RGSS3
`Tilemap.new` of a game of RGSS3 SHALL make a tilemap of the scene that is drawn by the rule
set of RGSS3: from a map of four layers, of which those that the table lacks are empty, from
the table of `flags`, and not at all while `visible` is false. A change made through the
`Table` object of `flags` or of `map_data` SHALL show in the next frame without `update`.

#### Scenario: Hidden, measured on the original
- **WHEN** a game of RGSS3 gives a tilemap a red sheet of 64 by 32 in slot 5 and a map of 1 by 1 by 4 with tile 1 in layer 0, and reads the pixel at (0, 0) at once, after `visible = false` and after `visible = true`
- **THEN** the pixels are red, black and red

#### Scenario: Star tile of the lowest layer, measured on the original
- **WHEN** the same tilemap gets a flags table of 8192 entries with `flags[1] = 0x10`
- **THEN** the pixel at (0, 0) is red

#### Scenario: Star tile of the third layer, measured on the original
- **WHEN** a game of RGSS3 gives a tilemap the red sheet, a map of 1 by 1 by 4 with tile 1 in layer 2 and a flags table of zeros, shows a green sprite of 32 by 32 at z 100, and reads the pixel at (0, 0) at once and after `flags[1] = 0x10` through the table object, without `update`
- **THEN** the pixels are green and red

#### Scenario: Shadow of the fourth layer, measured on the original
- **WHEN** the tilemap of the first scenario gets `map_data[0, 0, 3] = 1` through the table object, without `update`
- **THEN** the pixel at (0, 0) is `[126, 0, 0, 255]` and the pixels at (16, 0) and (0, 16) are red

#### Scenario: A map of three layers, measured on the original
- **WHEN** a game of RGSS3 gives a tilemap the red sheet and a map of 1 by 1 by 3 with tile 1 in layer 0
- **THEN** no error is raised and the pixels at (0, 0) and (16, 16) are red

#### Scenario: A map of fewer layers
- **WHEN** a game of RGSS3 gives a tilemap the red sheet and a map of 1 by 1 with tile 1, and then a map `Table.new(1)` with tile 1
- **THEN** no error is raised and the pixel at (0, 0) is red with each

#### Scenario: No flags, measured on the original
- **WHEN** the tilemap of the first scenario, whose `flags` was never set, is drawn, and again after `flags = Table.new(1)`
- **THEN** the pixel at (0, 0) is red both times

#### Scenario: VX
- **WHEN** a game of RGSS2 gives a tilemap the red sheet, a map of 1 by 1 by 3 with tile 1 in layer 0 and `visible = false`
- **THEN** the pixel at (0, 0) is red

### Requirement: Disposed tilemap of RGSS3
In RGSS3 every property and `update` of a disposed tilemap SHALL raise `RGSSError` with the
message `disposed tilemap`, while `disposed?`, `dispose` and `bitmaps` work and a slot reads
what it held. A write to a slot of the array of a disposed tilemap SHALL raise nothing and
SHALL be kept, where the original runtime ends with a page fault.

#### Scenario: Disposed, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.bitmaps[1] = b; r = [t.disposed?, t.dispose, t.disposed?, t.dispose]` and then each of `t.ox`, `t.ox = 1`, `t.update`, `t.map_data`, `t.flags`, `t.flags = Table.new(1)`, `t.flash_data`, `t.visible`, `t.viewport` and `t.viewport = nil`
- **THEN** `r` is `[false, nil, true, nil]` and each of the ten raises `RGSSError` with the message `disposed tilemap`

#### Scenario: The array of a disposed tilemap, measured on the original
- **WHEN** the game then evaluates `[t.bitmaps.class, t.bitmaps[1].equal?(b), t.bitmaps[9]]`
- **THEN** the result is `[Tilemap::BitmapArray, true, nil]`

#### Scenario: A write after dispose
- **WHEN** the game then evaluates `t.bitmaps[2] = b; t.bitmaps[1] = nil; [t.bitmaps[2].equal?(b), t.bitmaps[1]]` and shows a frame
- **THEN** no error is raised, the result is `[true, nil]`, and the frame shows nothing of the tilemap

#### Scenario: Not drawn any more
- **WHEN** the tilemap of "Hidden" is disposed
- **THEN** the pixel at (0, 0) is black

### Requirement: Window construction in RGSS3
In RGSS3 `Window.new` SHALL take up to four Integers, x, y, width and height, each 0 when
not given, truncated, negative values kept, and no viewport: "Construction" holds for
`Window.new` in RGSS1 and RGSS2 only. A fifth argument SHALL raise `ArgumentError` and a
`Viewport` `TypeError`. The window SHALL be the window of RGSS3 of the scene with its
defaults, and SHALL have a contents bitmap of 1 by 1 of its own.

#### Scenario: Without arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `w = Window.new; [w.x, w.y, w.width, w.height, w.z, w.padding, w.padding_bottom, w.back_opacity, w.openness, w.viewport, [w.contents.width, w.contents.height]]`
- **THEN** the result is `[0, 0, 0, 0, 100, 8, 8, 192, 255, nil, [1, 1]]`

#### Scenario: The other defaults, measured on the original
- **WHEN** a game of RGSS3 evaluates `w = Window.new; [w.ox, w.oy, w.opacity, w.contents_opacity, w.visible, w.active, w.pause, w.windowskin, w.cursor_rect.to_s, w.arrows_visible, w.tone.to_s, w.open?, w.close?]`
- **THEN** the result is `[0, 0, 255, 255, true, true, false, nil, "(0, 0, 0, 0)", true, "(0.000000, 0.000000, 0.000000, 0.000000)", true, false]`

#### Scenario: One to four arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `[w.x, w.y, w.width, w.height]` for `Window.new(1)`, `Window.new(1, 2)`, `Window.new(1, 2, 3)`, `Window.new(10, 20, 30, 40)`, `Window.new(10.7, 20.2, 30.9, 40.5)` and `Window.new(-10, -20, -30, -40)`
- **THEN** the results are `[1, 0, 0, 0]`, `[1, 2, 0, 0]`, `[1, 2, 3, 0]`, `[10, 20, 30, 40]`, `[10, 20, 30, 40]` and `[-10, -20, -30, -40]`, and each of these windows has z 100 and contents of 1 by 1

#### Scenario: No viewport argument, measured on the original
- **WHEN** a game of RGSS3 evaluates, with `v = Viewport.new(0, 0, 10, 10)`, `Window.new(v)`, `Window.new(nil, 0, 0, 0)`, `Window.new(10, 20, 30, 40, 5)`, `Window.new(v, 1, 2, 3, 4)` and `Window.new(1, 2, 3, 4, v)`
- **THEN** the first raises `TypeError` with `can't convert Viewport into Integer`, the second `TypeError` with `no implicit conversion from nil to integer`, and the last three `ArgumentError` with `wrong number of arguments (4 for 5)`

#### Scenario: Contents, measured on the original
- **WHEN** a game of RGSS3 evaluates `a = Window.new; b = Window.new(5, 6, 100, 80); [a.contents.equal?(b.contents), a.contents.equal?(a.contents), b.contents.width, b.contents.height, a.contents.get_pixel(0, 0).to_s, a.contents.disposed?]`
- **THEN** the result is `[false, true, 1, 1, "(0.000000, 0.000000, 0.000000, 0.000000)", false]`

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Window.new(Viewport.new(0, 0, 10, 10)).viewport.nil?` and `Window.new(1, 2, 3, 4)`
- **THEN** the first is `false` and the second raises `ArgumentError`

### Requirement: Window methods added in RGSS3
In RGSS3 `Window` SHALL define `padding` and `padding_bottom`, Integers truncated and not
limited, `padding=` setting both; `arrows_visible`, read as true or false;
`move(x, y, width, height)`, which assigns the four and returns the window; and `open?` and
`close?`, true at openness 255 and at openness 0 alone. Each SHALL reach the window of the
scene. In RGSS1 and RGSS2 none of them SHALL be defined.

#### Scenario: Paddings, measured on the original
- **WHEN** a game of RGSS3 evaluates, on one window, `[w.padding, w.padding_bottom]` after each of `w.padding = 20`, `w.padding_bottom = 5`, `w.padding = 7`, and after `w.padding_bottom = 30; w.padding = 7`
- **THEN** the results are `[20, 20]`, `[20, 5]`, `[7, 7]` and `[7, 7]`

#### Scenario: Values of the paddings, measured on the original
- **WHEN** a game of RGSS3 assigns -5, 100000, 10.7 and -10.7 to `padding` and to `padding_bottom`, reading the property after each, and then `nil` to each
- **THEN** the values read are -5, 100000, 10 and -10 for both, and `nil` raises `TypeError` with `no implicit conversion from nil to integer`

#### Scenario: Arrows flag, measured on the original
- **WHEN** a game of RGSS3 assigns `nil`, `0`, `1` and `"x"` to `arrows_visible`, reading it after each
- **THEN** the values are `false`, `true`, `true` and `true`

#### Scenario: Move, measured on the original
- **WHEN** a game of RGSS3 evaluates `[w.move(1, 2, 3, 4).equal?(w), w.x, w.y, w.width, w.height]`, the same with `move(-1, -2, -3, -4)` and with `move(1.9, 2.9, 3.9, 4.9)`, then `w.move(1, 2, 3)` and `w.move(nil, 2, 3, 4)`
- **THEN** the results are `[true, 1, 2, 3, 4]`, `[true, -1, -2, -3, -4]` and `[true, 1, 2, 3, 4]`, then an `ArgumentError` with `wrong number of arguments(3 for 4)` and a `TypeError` with `no implicit conversion from nil to integer`

#### Scenario: Open and closed, measured on the original
- **WHEN** a game of RGSS3 sets `openness` to 255, 254, 128, 1 and 0, evaluating `[w.open?, w.close?]` after each
- **THEN** the results are `[true, false]`, `[false, false]`, `[false, false]`, `[false, false]` and `[false, true]`

#### Scenario: Paddings move the contents
- **WHEN** a game of RGSS3 gives a window of 100 by 100 at (0, 0) without a skin the green contents of 50 by 50 and reads the frame, then assigns `padding = 20` and reads it, then `padding_bottom = 50` and reads it
- **THEN** in the first frame (8, 8) is green and (7, 7) black; in the second (8, 8) is black and (20, 20) and (30, 60) are green; in the third (30, 49) is green and (30, 50) black, and `w.padding` is still 20

#### Scenario: Arrows hidden and shown
- **WHEN** a game of RGSS3 gives a window of 100 by 100 at (0, 0) the arrow skin and transparent contents of 200 by 200, reads the pixel (50, 92), assigns `arrows_visible = false` and reads it, assigns `arrows_visible = true` and reads it
- **THEN** the second pixel differs from the first and the third is the same as the first

#### Scenario: Move reaches the scene
- **WHEN** a game of RGSS3 gives a window made by `Window.new(0, 0, 20, 20)` the grey skin and `back_opacity = 255`, and then calls `move(40, 30, 68, 68)`
- **THEN** the pixel (70, 70) is black before the call and not black after it, and the pixel (10, 10) is not black before and black after

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `w = Window.new; [:padding, :padding_bottom, :arrows_visible, :move, :open?, :close?, :tone].map { |name| w.respond_to?(name) }`
- **THEN** every element is `false`

### Requirement: Window tone in RGSS3
In RGSS3 `Window#tone` SHALL return the window's own `Tone`, the same object at every call.
`tone=` SHALL take a `Tone` only, copy its values into that object and leave the given one
unconnected; anything else SHALL raise `TypeError`. A change of the object through its own
methods SHALL change the window as an assignment does, and the frame drawn next SHALL show
it on the background.

#### Scenario: The object, measured on the original
- **WHEN** a game of RGSS3 evaluates `w.tone.equal?(w.tone)`, then `t = Tone.new(1, 2, 3, 4); w.tone = t; r = w.tone; [r.red, r.green, r.blue, r.gray, r.equal?(t)]`, and the same with `Tone.new(300, -300, 255.5, 300)`
- **THEN** the results are `true`, `[1.0, 2.0, 3.0, 4.0, false]` and `[255.0, -255.0, 255.0, 255.0, false]`

#### Scenario: Not a tone, measured on the original
- **WHEN** a game of RGSS3 evaluates `w.tone = nil`, `w.tone = 5` and `w.tone = Color.new(1, 2, 3, 4)`
- **THEN** each raises `TypeError`, with the messages `can't convert NilClass into Tone`, `can't convert Fixnum into Tone` and `can't convert Color into Tone`, and `w.tone` is as it was

#### Scenario: Changed through its own writer, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = w.tone; t.red = 50; w.tone.red`
- **THEN** the result is `50.0`

#### Scenario: Seen in the frame
- **WHEN** a game of RGSS3 gives a window of 68 by 68 at (0, 0) the grey skin and `back_opacity = 255`, and reads the pixel (30, 40) at once, after `w.tone.set(Tone.new(50, -30, 0, 0))`, after `w.tone.red = -255`, and after `w.tone = Tone.new`
- **THEN** the first pixel is the grey of the skin, (100, 100, 100); the second has 50 more red and 30 less green than the first and its blue; the third has red 0 and the green and blue of the second; the fourth equals the first

#### Scenario: The given object stays unconnected
- **WHEN** a game of RGSS3 evaluates `t = Tone.new(1, 2, 3, 4); w.tone = t; t.red = 99; w.tone.red`
- **THEN** the result is `1.0`

### Requirement: Assignments of a window in RGSS3 are calls of the scene
In RGSS3 every assignment of a property of a window, `move` and `update` SHALL be handed to
the window of the scene as one call, also when the value is the one the window has, and so
SHALL every change of `cursor_rect` through the methods of its `Rect`, also to equal
values. What a call recomputes is the scene's. The properties RGSS2 has SHALL take and
return values as in RGSS2.

#### Scenario: The cursor is placed through the Rect, measured on the original
- **WHEN** a game of RGSS3 gives a window of 40 by 40 at (0, 0) the cursor skin and `cursor_rect = Rect.new(0, 0, 100, 100)`, reads the pixel (20, 20) as the colour of the cursor, assigns `width = 120` and `height = 120` and reads the pixel (50, 50), then evaluates `w.cursor_rect.x = 0` and reads it again
- **THEN** the pixel (50, 50) differs from the colour of the cursor after the two assignments and has it after the change through the `Rect`

#### Scenario: Every way of changing the Rect, measured on the original
- **WHEN** the window of that scenario, grown to 120 by 120 with its cursor still cut to the old size, gets in turn, each on a window prepared anew, `w.cursor_rect.set(0, 0, 100, 100)`, `w.cursor_rect.set(Rect.new(0, 0, 100, 100))`, `w.cursor_rect = Rect.new(0, 0, 100, 100)`, `w.ox = 0` and `w.padding = 8`
- **THEN** after each of them the pixel (50, 50) has the colour of the cursor

#### Scenario: Calls that do not place it, measured on the original
- **WHEN** that window gets `w.move(0, 0, 120, 120)`, `w.x = 0`, `w.update` and `w.z = 100` in place of them
- **THEN** the pixel (50, 50) still differs from the colour of the cursor

#### Scenario: The cursor rectangle is the window's own, measured on the original
- **WHEN** a game of RGSS3 evaluates `r = Rect.new(1, 2, 3, 4); w.cursor_rect = r; c = w.cursor_rect; [c.x, c.y, c.width, c.height, c.equal?(r), c.equal?(w.cursor_rect)]`, then `w.cursor_rect = nil` and `w.cursor_rect = 5`
- **THEN** the result is `[1, 2, 3, 4, false, true]`, and the two others raise `TypeError`

#### Scenario: Limits of the opacities, measured on the original
- **WHEN** a game of RGSS3 assigns 300, 256, -5, 100.7, 254.9 and -0.5 to each of `opacity`, `back_opacity`, `contents_opacity` and `openness`, reading it after each
- **THEN** the values are 255, 255, 0, 100, 254 and 0 for each of the four

#### Scenario: Positions and flags, measured on the original
- **WHEN** a game of RGSS3 assigns -5, 100000, 10.7 and -10.7 to each of `x`, `y`, `z`, `width`, `height`, `ox` and `oy`, reading it after each, then `nil` to each, and `nil`, `0` and `"x"` to each of `visible`, `active` and `pause`
- **THEN** the numbers read are -5, 100000, 10 and -10, `nil` raises `TypeError` with `no implicit conversion from nil to integer`, and the flags read `false`, `true` and `true`

#### Scenario: Bitmaps and update, measured on the original
- **WHEN** a game of RGSS3 evaluates `b = Bitmap.new(8, 8); w.windowskin = b; w.contents = b; [w.windowskin.equal?(b), w.contents.equal?(b)]`, `w.windowskin = nil; w.contents = nil; [w.windowskin, w.contents]`, `w.contents = 5`, `w.windowskin = 5` and `w.update`
- **THEN** the results are `[true, true]`, `[nil, nil]`, two `TypeError`s and `nil`

#### Scenario: Viewport, measured on the original
- **WHEN** a game of RGSS3 evaluates `v = Viewport.new(0, 0, 10, 10); w.viewport = v; a = w.viewport.equal?(v); w.viewport = nil; [a, w.viewport]` and `w.viewport = 5`
- **THEN** the result is `[true, nil]`, and the second raises `TypeError` with `can't convert Fixnum into Viewport`

### Requirement: Disposal and disposed bitmaps of a window in RGSS3
In RGSS3 every method of a disposed window but `dispose` and `disposed?` SHALL raise
`RGSSError` with the message `disposed window`, and `dispose` SHALL return `nil` each time
and SHALL NOT dispose the window's bitmaps. A window whose skin or contents bitmap is
disposed SHALL raise nothing, SHALL go on returning that object, and SHALL be drawn as the
scene draws a window of RGSS3 without it.

#### Scenario: A disposed window, measured on the original
- **WHEN** a game of RGSS3 evaluates `c = Bitmap.new(10, 10); w.contents = c; w.dispose`, and then `[c.disposed?, w.disposed?, w.dispose]`, each of `x`, `width`, `contents`, `windowskin`, `tone`, `cursor_rect`, `open?`, `close?`, `update`, `padding`, `viewport` and `visible`, `w.x = 1` and `w.move(1, 2, 3, 4)`
- **THEN** the result is `[false, true, nil]`, and every one of the other fourteen raises `RGSSError` with the message `disposed window`

#### Scenario: A disposed skin, measured on the original
- **WHEN** a game of RGSS3 gives a window of 100 by 100 at (0, 0) the grey skin and `back_opacity = 255`, reads the pixel (50, 50), disposes the skin, reads it, then evaluates `w.update; w.width = 100; w.ox = 0; w.cursor_rect = Rect.new(0, 0, 8, 8); w.contents_opacity = 100; w.windowskin.equal?(skin)` and reads it again
- **THEN** the first pixel is not black, the second and the third are black, nothing is raised and the value is `true`

#### Scenario: Disposed contents, measured on the original
- **WHEN** a game of RGSS3 gives a window of 100 by 100 at (0, 0) without a skin the green contents, reads the pixel (10, 10), disposes the contents bitmap, evaluates `w.update; w.contents_opacity = 100` and reads the pixel again
- **THEN** the first pixel is green, the second is black, and nothing is raised

### Requirement: Calls at which the original of RGSS3 faults, through the class
Where the original runtime of RGSS3 ends with a page fault, a script of a game of RGSS3
SHALL go on: the assignments that place the cursor in a window on which `windowskin=` was
never called SHALL return and take effect, and so SHALL the assignment of contents after a
skin of 8 by 8 and a window or cursor rectangle of 100000 by 100000.

#### Scenario: No skin was ever assigned
- **WHEN** a game of RGSS3 evaluates `w = Window.new(0, 0, 96, 96); w.padding = 8; w.padding_bottom = 4; w.ox = 0; w.oy = 0; w.cursor_rect = Rect.new(0, 0, 0, 0); w.cursor_rect.set(0, 0, 0, 0); w.cursor_rect.x = 0; w.cursor_rect.empty; [w.padding, w.padding_bottom, w.cursor_rect.to_s]` and reads the pixel (48, 48)
- **THEN** the result is `[8, 4, "(0, 0, 0, 0)"]`, nothing is raised and the pixel is black

#### Scenario: A small skin and contents
- **WHEN** a game of RGSS3 evaluates `w = Window.new; w.windowskin = Bitmap.new(8, 8); w.contents = Bitmap.new(8, 8); w.contents.width`
- **THEN** the result is 8 and nothing is raised

#### Scenario: Sizes
- **WHEN** a game of RGSS3 evaluates `w = Window.new(0, 0, 100000, 100000); w.cursor_rect = Rect.new(0, 0, 100000, 100000); [w.width, w.cursor_rect.width]` and reads the pixel (0, 0)
- **THEN** the result is `[100000, 100000]` and nothing is raised

### Requirement: Method surface of Window in RGSS3
In RGSS3 `Window` SHALL define every method of the committed method list of RGSS3 with its
arity and no public instance method beside them: `stretch` and `stretch=` of RGSS1 SHALL
NOT be defined. The test of that list SHALL check `Window` beside the classes it checked
before.

#### Scenario: Every listed method exists
- **WHEN** the test of the list of RGSS3 runs after this change
- **THEN** the 53 entries of `Window` exist with their arities, among them `initialize` with -1, `move` with 4 and `open?`, `close?`, `padding`, `padding_bottom`, `arrows_visible`, `tone` and `viewport` with 0, and `Window` is in the list of bound classes

#### Scenario: Nothing beside the list
- **WHEN** `Window.public_instance_methods(false)` is compared with the entries of the list in a game of RGSS3, and `[Window.method_defined?(:stretch), Window.method_defined?(:stretch=)]` is evaluated
- **THEN** the class has no method that the list lacks, and the value is `[false, false]`

#### Scenario: No copies
- **WHEN** a game of RGSS3 evaluates `Window.new.clone` and `Window.new.dup`
- **THEN** each raises `TypeError`

### Requirement: The window of RGSS1 and RGSS2 is unchanged by RGSS3
The class `Window` of a game of RGSS1 or RGSS2 SHALL NOT change with that of RGSS3: every
test of the two binding packages that passed before SHALL pass under its name with its
expected values unedited, over both interpreters. The tests of RGSS3 SHALL be a file of the
package over Ruby 1.9.2 alone, and the scenarios named "XP and VX" SHALL be held by tests of
the shared files.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkrgss-binding` and of `vkrgss-binding-rgss3` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: Existing tests
- **WHEN** the difference of `crates/vkrgss-binding/tests` and `crates/vkrgss-binding-rgss3/tests` against the start of the change is read
- **THEN** it holds the new file of RGSS3, one new test in each of `tests/scene.rs` and `tests/scene_rgss2.rs` of `vkrgss-binding`, and `Window` added to the bound classes of `arity_rgss3.rs`; no test that existed, no expected value and no line of the allow-list changed

#### Scenario: Other crates
- **WHEN** the difference of `crates` against the start of the change is taken
- **THEN** it names files of `crates/vkrgss-binding` and `crates/vkrgss-binding-rgss3` only

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
- **WHEN** `cargo test --locked -- --list` of `vkrgss-binding`, `vkrgss-binding-rgss3` and `vkrgss-core` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Plane.new.respond_to?(:viewport=)`, `Viewport.new` without arguments, and `v = Viewport.new(0, 0, 8, 8); s = Sprite.new(v); v.dispose; s.disposed?`
- **THEN** the results are `false`, an `ArgumentError`, and `true`, over both interpreters
