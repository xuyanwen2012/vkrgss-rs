# Spec Delta

## ADDED Requirements

### Requirement: Tilemap layer evaluation
For each pixel `(X, Y)` in its clip a tilemap layer op SHALL read the tile number of its map
layer at cell `((X + ox) div 32, (Y + oy) div 32)`, wrapped into the table's size. When the
priorities table's entry for that number (0 when it has none) equals the op's priority, the
op SHALL blend the tile's texel for that pixel onto the frame with the normal blend formula
at opacity 255; otherwise it SHALL draw nothing. CPU reference and GPU SHALL agree bit for
bit.

#### Scenario: Tileset tile
- **WHEN** a cell holds tile number 384 + 9 and the op's priority matches
- **THEN** the cell shows the 32 by 32 block at column 1, row 1 of the tileset

#### Scenario: Priority filter
- **WHEN** a cell's tile has priority 2 and the op draws priority 0
- **THEN** the cell is unchanged by this op

#### Scenario: Wrap
- **WHEN** the origin is beyond the map's size or negative
- **THEN** the map repeats in both directions

#### Scenario: Translucent texel
- **WHEN** a tile texel has alpha 142 over a frame pixel
- **THEN** the result is that of a sprite pixel with the same values

### Requirement: Tileset tiles in the tilemap op
A tile number from 384 SHALL select the tileset block at column `(number - 384) mod 8`, row
`(number - 384) div 8`, and a number below 48 SHALL draw nothing. An absent bitmap, or a texel
outside its bitmap, SHALL draw nothing.

#### Scenario: Block position
- **WHEN** tile number 384 + 17 is drawn
- **THEN** it shows the block at column 1, row 2 of the tileset

#### Scenario: Short tileset
- **WHEN** a tile number names a row beyond the tileset bitmap's height
- **THEN** the cell is unchanged

### Requirement: Autotiles in the tilemap op
A tile number from 48 to 383 SHALL select the autotile bitmap `number div 48 - 1` and the
pattern `number mod 48`, whose four 16 by 16 quarters come from the autotile sheet as the
pattern table of `core-scene` says, offset by 96 pixels times the animation step modulo the
sheet's number of frames.

#### Scenario: Every autotile pattern
- **WHEN** each of the 48 patterns is drawn from a sheet whose 16 by 16 quarters have distinct colours
- **THEN** every quarter equals the one the pattern table names

#### Scenario: Animation step
- **WHEN** a four-frame autotile sheet is drawn at animation steps 0, 1 and 5
- **THEN** the picture comes from frame 0, frame 1 and frame 1

#### Scenario: Missing autotile
- **WHEN** a cell names an autotile whose bitmap is absent
- **THEN** the cell is unchanged
