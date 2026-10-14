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
