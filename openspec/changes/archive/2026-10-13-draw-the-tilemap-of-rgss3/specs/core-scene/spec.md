## ADDED Requirements

### Requirement: Tilemap of RGSS3 in the scene
The scene SHALL be able to create a tilemap of RGSS3. While it is visible it SHALL be
extracted as the tilemap of RGSS2 is: two plane ops at z 0 and z 200 with the same origin,
wrapping, counter, animation step and flash level, but naming the rule set of RGSS3, with
its flags table in the place of the passages table and its map table sent with every layer
it has.

#### Scenario: Two planes of the rule set of RGSS3
- **WHEN** a tilemap of RGSS3 with a map of 4 by 3 by 4 elements and a flags table is extracted
- **THEN** the packet has a lower and an upper plane op that both name the rule set of RGSS3, the map table with its 48 elements and the flags table

#### Scenario: Star tiles among sprites, measured on the original
- **WHEN** a tilemap of RGSS3 has the tile 300 with the flag 0x10 in layer 2 of one cell and in layer 0 of another, and shares a viewport with sprites at z 100 and z 300 that cover both cells
- **THEN** the sprite at z 100 is under the tile of layer 2 and over the tile of layer 0, and the sprite at z 300 is over both

#### Scenario: A shadow from layer 3, measured on the original
- **WHEN** a tilemap of RGSS3 over a sprite of level 200 at z -10 has 15 in layer 3 of cell (1, 1) of an otherwise empty map
- **THEN** the frame shows 99 in the 32 by 32 pixels of that cell and 200 around it

#### Scenario: Counter and scrolling as RGSS2
- **WHEN** a tilemap of RGSS3 on a map 5 cells wide has ox -40 and is updated 30 times
- **THEN** the first cell is 3, the plane's corner is 24 pixels left of the viewport's corner, and the animation step is 1

#### Scenario: Latch as RGSS2, measured on the original
- **WHEN** a tilemap of RGSS3 in a viewport with ox 0 is assigned a viewport with ox 16 and a frame is extracted, then the tilemap is refreshed and another is extracted
- **THEN** the first frame has the origin of the old viewport and the second is moved by 16

### Requirement: Visibility of a tilemap of RGSS3
While the `visible` of a tilemap of RGSS3 is false, the scene SHALL extract no plane op for
it, so that neither its tiles, its shadows nor its flash are drawn. Its update counter SHALL
go on. A tilemap of RGSS2 SHALL be extracted whatever its `visible` is, as before.

#### Scenario: Hidden, measured on the original
- **WHEN** a tilemap of RGSS3 with tiles, a star tile, a shadow and a flash table is extracted, then `visible` is set to false and it is extracted again
- **THEN** the first packet has two plane ops and the second has none

#### Scenario: Back, measured on the original
- **WHEN** such a tilemap is updated 30 times while `visible` is false and `visible` is then set to true
- **THEN** the next packet has two plane ops whose animation step is 1

#### Scenario: RGSS2 stays, measured on the original
- **WHEN** `visible = false` is set on a tilemap of RGSS2
- **THEN** the next packet has its two plane ops as before

### Requirement: Tilemaps of RGSS1 and RGSS2 stay as they are
Adding the tilemap of RGSS3 SHALL NOT change what a tilemap of RGSS1 or of RGSS2 extracts:
a tilemap of RGSS2 names the rule set of RGSS2 in its plane ops, and no expected value of an
existing test of tilemaps is edited.

#### Scenario: Existing tests
- **WHEN** the tests of `crates/vkxp-core/src/tilemap/tests.rs` and `crates/vkxp-core/src/tilemap/rgss2/tests.rs` that existed before this change run
- **THEN** they pass with their expected values as they were, and every line that `crates/vkxp-core/tests/data/tilemap-frames.txt` had is unchanged

#### Scenario: Rule set of RGSS2
- **WHEN** a tilemap created as one of RGSS2 is extracted
- **THEN** both its plane ops name the rule set of RGSS2
