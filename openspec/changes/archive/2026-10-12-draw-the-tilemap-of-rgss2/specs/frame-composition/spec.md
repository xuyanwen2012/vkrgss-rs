## ADDED Requirements

### Requirement: Tiles of a tilemap plane of RGSS2
A tilemap plane op of RGSS2 SHALL draw, for each cell of its 21 by 16 cells and each of the
three layers in order, the tile the map names by the numbering, sheets, autotile shapes and
animation of rules 1 to 3 of this change's design. A number from 1024 to 1535, a negative
number, an absent bitmap and a source pixel outside its bitmap SHALL draw nothing. CPU
reference and GPU SHALL agree bit for bit.

#### Scenario: Every number and shape
- **WHEN** each committed case of the measurement that shows single tile numbers of every range and every shape of every autotile kind is composed from sheets with distinct pixels
- **THEN** every pixel equals the expected picture of the case

#### Scenario: Animation, measured on the original
- **WHEN** a map of A1 tiles is composed at the animation steps 0 to 5
- **THEN** floors of the animated kinds come from the block moved by 0, 64, 128, 64, 0 and 64 pixels, kinds 2 and 3 do not move, and waterfalls come from the block moved down by 0, 32, 64, 0, 32 and 64

#### Scenario: Undefined numbers
- **WHEN** a map holds the numbers 1024, 1535 and -256
- **THEN** their cells are unchanged

### Requirement: Planes, tables and shadows of RGSS2
The lower plane SHALL draw the tiles whose entry of the passages table lacks the bit 0x10, the
upper plane the tiles of layer 2 that have it, and no plane a tile of layer 0 or 1 that has
it. The lower plane SHALL draw the table tiles, the edge under a table and the shadows by
rule 4 of this change's design. Nothing SHALL be drawn outside the 672 by 512 pixels of a
plane.

#### Scenario: Cases of the measurement
- **WHEN** each committed case of planes, tables and shadows is composed
- **THEN** every pixel equals the expected picture of the case

#### Scenario: Shadow, measured on the original
- **WHEN** a cell of an A2 floor that is not a table has A4 tiles in layer 0 at its left and upper left neighbours, over a frame level of 200 in that cell
- **THEN** the left 16 by 32 pixels of the cell show the tile darkened by `(v * 127) >> 8`

#### Scenario: Star tile in a lower layer, measured on the original
- **WHEN** a tile with the bit 0x10 lies in layer 0
- **THEN** neither plane draws it

### Requirement: Blend and flash of a tilemap plane of RGSS2
A tile pixel SHALL be blended as `(s * (a + 1) + d * (256 - a)) >> 8`, the last pixel of a
clipped row of odd length by the formula of a sprite. Each plane SHALL then add to every
channel of a cell whose flash entry is above 0 the entry's nibble for that channel times the
op's flash level, limited to 255.

#### Scenario: Translucent tiles, measured on the original
- **WHEN** the committed case of translucent tiles over a coloured frame is composed
- **THEN** every pixel equals the expected picture

#### Scenario: Flash twice
- **WHEN** a cell of level 100 has the flash entry 0x111 and the level is 4, with both planes drawn
- **THEN** the cell shows 108

### Requirement: Whole tilemaps of RGSS2 as measured
The CPU reference and the Vulkan backend SHALL draw scenes with tilemaps of RGSS2 as the model
of the measurement does.

#### Scenario: Random scenes
- **WHEN** forty committed frames of the measurement's random generator are composed by both backends
- **THEN** every frame equals its expected picture, byte for byte
