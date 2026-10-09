# Spec Delta

## Purpose
Defines the Ruby classes `Sprite`, `Viewport`, `Window`, `Plane` and `Tilemap` as scripts see
them: construction, properties, the objects those properties hand out, disposal and garbage
collection, on top of the scene that `core-scene` defines.

## ADDED Requirements

### Requirement: Method surface of the scene classes
`Sprite`, `Viewport`, `Window`, `Plane`, `Tilemap` and `TilemapAutotiles` SHALL define every
method the original runtime defines for them with the same arity, as listed in the committed
method list, and no `disposed?` method on `Viewport`, which the original does not have.

#### Scenario: Every listed method exists
- **WHEN** each entry of the method list for these classes is checked with `instance_method`
- **THEN** the method exists and its arity equals the listed arity

#### Scenario: Whole list covered
- **WHEN** the arity test runs after this change
- **THEN** it checks every entry of the method list, for all fifteen classes and modules

### Requirement: Construction
`Sprite.new`, `Plane.new`, `Window.new` and `Tilemap.new` SHALL take an optional viewport, and
`Viewport.new` SHALL take either x, y, width and height or a `Rect`. Each SHALL create the
element in the scene with the defaults of `core-scene`. An argument of the wrong class SHALL
raise `TypeError`, and a disposed viewport given to a constructor SHALL raise `RGSSError`.

#### Scenario: Sprite in a viewport
- **WHEN** `v = Viewport.new(0, 0, 100, 100); s = Sprite.new(v)` is evaluated
- **THEN** `s.viewport.equal?(v)` is `true`

#### Scenario: Viewport from a rectangle
- **WHEN** `Viewport.new(Rect.new(1, 2, 3, 4)).rect` is evaluated
- **THEN** the result equals `Rect.new(1, 2, 3, 4)`

#### Scenario: Wrong argument
- **WHEN** `Sprite.new("viewport")` is evaluated
- **THEN** a `TypeError` is raised

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
`disposed tilemap`. Disposing a sprite, plane or window SHALL NOT dispose its bitmaps.

#### Scenario: Use after dispose
- **WHEN** `s = Sprite.new; s.dispose; s.x` is evaluated
- **THEN** an `RGSSError` with the message `disposed sprite` is raised

#### Scenario: Bitmap survives
- **WHEN** a sprite with a bitmap is disposed
- **THEN** the bitmap's `disposed?` is `false`

#### Scenario: Not drawn any more
- **WHEN** a visible sprite is disposed and the scene is rendered
- **THEN** its pixels are gone

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
