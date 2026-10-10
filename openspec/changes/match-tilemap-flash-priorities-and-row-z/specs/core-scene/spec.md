# Spec Delta

## MODIFIED Requirements

### Requirement: Tile priorities
A tile's priority SHALL be the priorities table's value for its identifier, 0 when the table
has no such entry. Tiles of priority 0 SHALL be drawn at z 0 among the elements of the
tilemap's viewport. A tile of priority p from 1 to 5 in map row R SHALL be drawn at
`z = 32 * R - oy + 32 + 32 * p` with `oy` the tilemap's origin; the viewport's origin and
rectangle do not enter. A tile whose priority is not 0 to 5 SHALL not be drawn.

#### Scenario: Ground under sprites
- **WHEN** a sprite with z 1 overlaps tiles of priority 0
- **THEN** the sprite is in front

#### Scenario: Same row
- **WHEN** a tile of priority 1 has its top edge at y 96 and a sprite with z 159 overlaps it
- **THEN** the tile is in front

#### Scenario: Row below
- **WHEN** that sprite's z is 192
- **THEN** the sprite is in front of the tile

#### Scenario: Higher priority
- **WHEN** a tile of priority 2 with its top edge at y 96 overlaps a sprite with z 191
- **THEN** the tile is in front

#### Scenario: Viewport origin
- **WHEN** a tile of priority 1 in map row 1 is drawn with the tilemap's oy 0 in a viewport whose oy is 10
- **THEN** its z is 96

#### Scenario: Measured rows
- **WHEN** the rows (R, p, oy) = (1, 1, 7), (2, 2, 32), (3, 3, 109), (-1, 5, -20) and (11, 5, 300) are drawn
- **THEN** their z is 89, 128, 115, 180 and 244

#### Scenario: Priority out of range
- **WHEN** the priorities table holds 6, 9, -1 or 100 for a tile on the map
- **THEN** the tile is not drawn, neither as ground nor in a row

### Requirement: Tilemap flash data
For every visible cell whose entry in the flash data table is 0 or more and has a non-zero
nibble among its lowest twelve bits, a tilemap SHALL add to what lies below the ground layer's
top, per channel and saturating at 255, the nibble (red, green, blue from the high to the low
end of those bits) times `m = 4 + [0, 1, 2, 3, 4, 3, 2, 1][(u / 4) % 8]`, with `u` the number
of updates of that tilemap. Bits 12 to 14 SHALL be ignored and a negative entry SHALL draw
nothing. A flash table of another size than the map SHALL be indexed from the first visible
column and row, wrapped by the map's size, counted on modulo the flash table's size.

#### Scenario: Flashing cell
- **WHEN** the flash data holds `0xF00` for one cell and the tilemap is rendered
- **THEN** that cell is redder than without the entry and its neighbours are unchanged

#### Scenario: Pulse
- **WHEN** a cell with `0xFFF` over black is drawn after 0, 4, 16, 28 and 32 updates
- **THEN** each channel is 60, 75, 120, 75 and 60

#### Scenario: Measured samples
- **WHEN** the entries `0x7ABC` and `0xFFF` over (50, 100, 150) are drawn after 4 updates, `0xF00` over (204, 51, 110) after 44, and `-1` and `0x1000` over any colour
- **THEN** the results are (100, 155, 210), (125, 175, 225), (255, 51, 110), and the colour unchanged for the last two

#### Scenario: Scrolled map
- **WHEN** the tilemap's origin is moved by 16 pixels
- **THEN** the overlay moves with its cell

#### Scenario: Flash table of another size
- **WHEN** a map of 7 by 5 cells has a flash table of 5 by 4 and ox is 0, 40, 264, -50 and -300
- **THEN** the screen columns from the one that holds pixel ox use the flash columns starting 0, 1, 1, 0 and 4, each counted on modulo 5
