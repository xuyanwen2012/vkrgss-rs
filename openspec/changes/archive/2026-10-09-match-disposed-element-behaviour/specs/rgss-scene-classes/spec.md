# Spec Delta

## MODIFIED Requirements

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

### Requirement: Disposal
`dispose` SHALL remove the element from the scene and `disposed?`, where the class has it,
SHALL report it. Any other method of a disposed element SHALL raise `RGSSError` with the
message `disposed sprite`, `disposed viewport`, `disposed window`, `disposed plane` or
`disposed tilemap`, except `Sprite#bitmap` and `Tilemap#autotiles`, which SHALL keep returning
their object. Disposing a sprite, plane or window SHALL NOT dispose its bitmaps. Disposing a
viewport SHALL dispose every sprite, plane, window and tilemap created in it.

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

#### Scenario: Viewport takes its elements along, measured on the original
- **WHEN** a sprite, a plane, a window and a tilemap are created in a viewport, a second sprite is created without one, and the viewport is disposed
- **THEN** `disposed?` is `true` for the four, `s.x` raises `RGSSError` with `disposed sprite`, disposing them again raises nothing, and the second sprite is alive
