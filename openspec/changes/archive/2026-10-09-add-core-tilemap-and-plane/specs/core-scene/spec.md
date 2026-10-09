## REMOVED Requirements

### Requirement: Planes and tilemaps draw nothing yet
**Reason**: Planes and tilemaps are now drawn; the requirements added below define how.
**Migration**: None for callers. The tilemap's flash data still draws nothing, which the
requirement "Tilemap flash data is stored only" now states.

## ADDED Requirements

### Requirement: Plane
A visible plane with a bitmap SHALL repeat that bitmap without end in both directions over its
viewport's rectangle, or the whole screen without a viewport, so that bitmap pixel (ox, oy) is
at the top-left corner of that area. Its zoom SHALL scale the repeated image about that corner.
Its opacity, blend type, colour and tone SHALL act as a sprite's do. Repeats SHALL meet without
gaps or overlaps.

#### Scenario: Tiling
- **WHEN** a plane with a 64 by 64 bitmap covers a 640 by 480 screen
- **THEN** screen pixel (x, y) shows bitmap pixel (x mod 64, y mod 64) for every x and y

#### Scenario: Scrolling
- **WHEN** the plane's origin is set to 10, 20
- **THEN** screen pixel (0, 0) shows bitmap pixel (10, 20)

#### Scenario: Negative origin
- **WHEN** the plane's origin is -1, -1
- **THEN** screen pixel (0, 0) shows bitmap pixel (63, 63)

#### Scenario: Zoom
- **WHEN** the plane's zoom is 2 in both directions and its origin is 0, 0
- **THEN** screen pixels (0, 0) to (1, 1) show bitmap pixel (0, 0) and the image repeats every 128 pixels

#### Scenario: Seams with half-transparent pixels
- **WHEN** a plane with a uniformly half-transparent bitmap is drawn over a flat background at zoom 1.5
- **THEN** every covered screen pixel has the same colour

#### Scenario: Inside a viewport
- **WHEN** the plane belongs to a viewport with the rectangle 100, 50, 200, 100
- **THEN** it is drawn only inside that rectangle, with bitmap pixel (ox, oy) at screen pixel (100, 50)

### Requirement: Tile identifiers
A tile identifier of 384 or more SHALL name the 32 by 32 tile of the tileset bitmap at column
`(id - 384) mod 8` and row `(id - 384) div 8`. An identifier from 48 to 383 SHALL name pattern
`id mod 48` of autotile `id div 48 - 1`. An identifier below 48, a tile outside its bitmap, and
a tile whose tileset or autotile bitmap is missing or disposed SHALL draw nothing.

#### Scenario: Tileset tile
- **WHEN** a map cell holds 384 + 8 + 3
- **THEN** it shows the tileset pixels from (96, 32) to (127, 63)

#### Scenario: Empty cell
- **WHEN** a map cell holds 0
- **THEN** nothing is drawn for that cell in that layer

#### Scenario: Missing tileset
- **WHEN** a tilemap has map data and no tileset bitmap
- **THEN** its tileset tiles draw nothing and extraction succeeds

### Requirement: Autotile patterns
An autotile tile SHALL be assembled from four 16 by 16 quarters of its 96-pixel-wide,
128-pixel-high sheet, chosen by the pattern number through the 48-entry table of the original
runtime, in the order top-left, top-right, bottom-left, bottom-right. A sheet that is 32 pixels
high SHALL instead supply the whole 32 by 32 tile for every pattern.

#### Scenario: Enclosed pattern
- **WHEN** a cell holds pattern 0 of an autotile
- **THEN** its four quarters come from sheet positions (32, 64), (48, 64), (32, 80) and (48, 80)

#### Scenario: Isolated pattern
- **WHEN** a cell holds pattern 47 of an autotile
- **THEN** its four quarters come from sheet positions (0, 0), (16, 0), (0, 16) and (16, 16)

#### Scenario: Every pattern
- **WHEN** each of the 48 patterns is drawn from a sheet whose 48 quarters are all different
- **THEN** each quarter of each drawn tile equals the quarter the table names

#### Scenario: Single-tile sheet
- **WHEN** an autotile sheet is 32 by 32 pixels
- **THEN** every pattern of that autotile shows the whole sheet

### Requirement: Autotile animation
An autotile sheet wider than one frame SHALL be animated: with frames 96 pixels wide (32 for a
32-pixel-high sheet), the frame shown SHALL advance by one, cyclically, with every 16 updates of
the tilemap. Without updates the frame SHALL NOT change.

#### Scenario: Four frames
- **WHEN** a tilemap with a 384-pixel-wide autotile sheet is updated 16, 32, 48 and 64 times
- **THEN** it shows frames 1, 2, 3 and then 0 again

#### Scenario: No update
- **WHEN** frames are extracted repeatedly without updating the tilemap
- **THEN** the same animation frame is shown each time

### Requirement: Tilemap layout
A visible tilemap with map data SHALL draw each cell (x, y) of each of its three layers as a
32 by 32 tile at `x * 32 - ox`, `y * 32 - oy` within its viewport's rectangle, or the screen
without a viewport, the layers in ascending order where they share a draw position in the
order. The map SHALL repeat in both directions beyond its edges.

#### Scenario: Position
- **WHEN** a 20 by 15 map is drawn with origin 0, 0
- **THEN** the tile of cell (3, 2) covers screen pixels (96, 64) to (127, 95)

#### Scenario: Scrolling by pixels
- **WHEN** the origin is 40, 0
- **THEN** the tile of cell (1, 0) starts 8 pixels left of the screen and cell (2, 0) starts at x 24

#### Scenario: Upper layer over lower
- **WHEN** layers 0 and 2 both have an opaque tile in the same cell with the same priority
- **THEN** the cell shows the tile of layer 2

#### Scenario: Wrap-around
- **WHEN** a 20 by 15 map is drawn with origin 320, 0
- **THEN** the right half of the screen shows the map's columns 0 to 9

### Requirement: Tile priorities
A tile's priority SHALL be the priorities table's value for its identifier, 0 when the table
has no such entry. Tiles of priority 0 SHALL be drawn at z 0 among the elements of the
tilemap's viewport. A tile of priority p from 1 to 5 SHALL be drawn at the z given by its top
edge's y position in the viewport plus `(p + 1) * 32`, so that it ranks among sprites by row.

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

### Requirement: Shared tables
The tilemap SHALL show the contents its map data and priorities tables have at each extraction,
including changes made to a table after it was assigned to the tilemap.

#### Scenario: Cell changed after assignment
- **WHEN** a map cell is changed through the table that was earlier assigned as map data
- **THEN** the next extracted frame shows the new tile, without the table being assigned again

### Requirement: Tilemap flash data is stored only
A tilemap SHALL store and return its flash data table and SHALL draw nothing for it.

#### Scenario: Flash data present
- **WHEN** a tilemap is given a flash data table with non-zero entries
- **THEN** its frames equal those without the table
