## ADDED Requirements

### Requirement: Draw order of RGSS2
In a scene of RGSS2 elements SHALL be drawn in ascending order of (z, y, creation), where y is
the integer y of a sprite and 0 for every other element, and creation is the order in which
the elements were made, which nothing renews. A viewport SHALL take its place among the
top-level elements by (its z, 0, its creation), and its own elements SHALL be ordered by the
same key among themselves. A scene of RGSS1 SHALL keep "Draw order".

#### Scenario: y breaks the tie, measured on the original
- **WHEN** two overlapping sprites of equal z are made, the first with y 9 and the second with y 0
- **THEN** the first is drawn over the second

#### Scenario: The property, not the place, measured on the original
- **WHEN** an older sprite has `y = 20, oy = 20` and a younger one of equal z has `y = 0`
- **THEN** the older is drawn over the younger

#### Scenario: Viewport, measured on the original
- **WHEN** a sprite in a viewport has y 50 and a younger top-level sprite of the viewport's z has y 0
- **THEN** the top-level sprite is drawn over the one in the viewport

#### Scenario: Window, measured on the original
- **WHEN** an older sprite with y 50 and a younger window have equal z, and likewise with y -50
- **THEN** the sprite is over the window in the first case and under it in the second

#### Scenario: Cases of the measurement
- **WHEN** each committed case of the measurement's order topic is extracted and composed
- **THEN** every pixel equals the expected picture, and the same scenes in RGSS1 keep the order of RGSS1

### Requirement: Viewport flash of RGSS2
In a scene of RGSS2 a flashing viewport SHALL apply its tone, then its colour, then the flash
colour at its current alpha, and a flash without a colour SHALL, while it lasts, draw none of
the viewport's elements and apply neither its tone nor its colour. A scene of RGSS1 SHALL keep
"Viewport effects".

#### Scenario: Colour and flash, measured on the original
- **WHEN** each committed case of a viewport with a colour and a flash is composed
- **THEN** every pixel equals the expected picture

#### Scenario: Flash without a colour, measured on the original
- **WHEN** a viewport with a tone and a sprite flashes with `nil` for 5 updates
- **THEN** the frames of those updates show what is behind the viewport unchanged

### Requirement: Plane tile size of RGSS2
In a scene of RGSS2 the tile of a plane SHALL be `floor(w * zoom_x)` by `floor(h * zoom_y)`
pixels computed in double precision. A scene of RGSS1 SHALL keep "Planes tile a stretched
tile".

#### Scenario: Measured on the original
- **WHEN** a plane of a bitmap 90 high has `zoom_y = 0.7`
- **THEN** it repeats every 62 rows in RGSS2 and every 63 rows in RGSS1
