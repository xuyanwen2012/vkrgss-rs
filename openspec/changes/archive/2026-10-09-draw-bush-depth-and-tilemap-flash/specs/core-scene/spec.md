# Spec Delta

## ADDED Requirements

### Requirement: Sprite bush depth
A sprite with a bush depth greater than zero SHALL draw the lowest rows of its source
rectangle, as many as the bush depth, with half of the opacity of the rows above, and the rest
unchanged. A bush depth of zero or less SHALL change nothing, and one that exceeds the source
height SHALL apply to the whole sprite.

#### Scenario: Lower part is fainter
- **WHEN** an opaque white 32 by 32 sprite with bush depth 8 is drawn over black
- **THEN** its upper 24 rows are white and its lower 8 rows are mid grey

#### Scenario: With zoom
- **WHEN** that sprite is drawn with a vertical zoom of 2
- **THEN** the lower 16 screen rows are the fainter ones

#### Scenario: Zero
- **WHEN** the bush depth is 0
- **THEN** the frame equals that of a sprite without the property

### Requirement: Tilemap flash data
For every map cell whose entry in the flash data table is non-zero, a tilemap SHALL draw a
32 by 32 overlay above the cell's ground tiles in the colour the entry encodes, four bits each
for red, green and blue from the high to the low end of its lowest twelve bits, with a strength
that rises and falls as the tilemap is updated. Cells with a zero entry and a tilemap without
flash data SHALL be drawn as before.

#### Scenario: Flashing cell
- **WHEN** the flash data holds `0xF00` for one cell and the tilemap is rendered at the strongest point of the pulse
- **THEN** that cell is redder than without the entry and its neighbours are unchanged

#### Scenario: Pulse
- **WHEN** the tilemap is updated repeatedly
- **THEN** the strength of the overlay differs between at least two of the frames and repeats

#### Scenario: Scrolled map
- **WHEN** the tilemap's origin is moved by 16 pixels
- **THEN** the overlay moves with its cell

## REMOVED Requirements

### Requirement: Tilemap flash data is stored only
**Reason**: Flash data is now drawn, as the new requirement "Tilemap flash data" states.
**Migration**: None; a tilemap without flash data draws as before.
