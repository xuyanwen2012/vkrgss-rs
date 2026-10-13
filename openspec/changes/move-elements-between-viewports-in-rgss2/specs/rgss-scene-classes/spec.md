## MODIFIED Requirements

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

## ADDED Requirements

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
