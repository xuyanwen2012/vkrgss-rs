## ADDED Requirements

### Requirement: Rule set of a tilemap plane op
A tilemap plane op SHALL be evaluated by the rule set it names. With that of RGSS2 nothing
changes. With that of RGSS3 the numbering, sheets, autotile shapes, animation, plane size,
wrapping, blend, flash and the drawing of a table tile SHALL be those of RGSS2, and tables,
the edge under a table, shadows and star tiles SHALL follow the requirements of RGSS3 below.

#### Scenario: RGSS2 unchanged
- **WHEN** the tests of `crates/vkxp-compose-ref/tests/tilemap_rgss2.rs` run with their ops naming the rule set of RGSS2
- **THEN** they pass, and `tests/data/tilemap-rgss2-digests.txt` of that crate is byte for byte what it was before this change

#### Scenario: Common numbering, measured on the original
- **WHEN** in the small map of this change's design layer 0 of cell (1, 1) is 1545 and layer 2 of cell (2, 1) is 37
- **THEN** pixel (35, 40) shows A5(35, 40) and pixel (67, 40) shows B(163, 136)

#### Scenario: Common animation, measured on the original
- **WHEN** layer 0 of cell (1, 1) of the small map is 2048 and the op is composed at the animation steps 0, 1, 2 and 3
- **THEN** pixel (35, 40) shows A1(35, 72), A1(99, 72), A1(163, 72) and A1(99, 72)

#### Scenario: Common blend, measured on the original
- **WHEN** layer 0 of cell (1, 1) of the small map is 1545, the sheets are those of the generated set whose every alpha is 128, and the ground has level 200 in every channel
- **THEN** pixel (35, 40) shows (140, 112, 182), each channel being `(s * 129 + 200 * 128) >> 8` of the texel (81, 24, 163)

#### Scenario: Common flash, measured on the original
- **WHEN** the small map is empty over a ground of level 100, the flash table of 4 by 3 has the entry 0x111 at (0, 0), the flash level is 4, and the lower and the upper plane are both composed
- **THEN** pixel (4, 4) shows 108 in every channel and pixel (40, 4) shows 100

### Requirement: Tables of RGSS3
In the rule set of RGSS3 a tile SHALL be a table when its number is from 2816 to 4351 and
its entry of the flags table has the bit 0x80, in any layer and on either plane; the column
of its autotile SHALL NOT matter and the bit SHALL do nothing on other numbers. The entry of
a number SHALL be the element at that number, and 0 for a negative number, for a number not
below the x size of the flags table, and without a flags table.

#### Scenario: A table by its flag, measured on the original
- **WHEN** layer 0 of cell (1, 0) of the small map is 2844 and its flag is 0x80
- **THEN** the pixels (35, 4), (35, 16), (52, 23), (35, 24), (52, 31), (35, 32) and (52, 39) show A2(35, 68), A2(35, 48), A2(20, 55), A2(35, 80), A2(20, 87), A2(35, 88) and A2(20, 95), and pixel (35, 40) is the ground

#### Scenario: Without the flag, measured on the original
- **WHEN** the same map is composed with the flag 0, and again without a flags table, and again with a flags table of 100 elements that are all 0x80
- **THEN** each time the pixels (35, 16) and (35, 24) show A2(35, 80) and A2(35, 88) and pixel (35, 32) is the ground

#### Scenario: Column 7 is no table, measured on the original
- **WHEN** layer 0 of cell (1, 0) of the small map is 3180 and its flag is 0
- **THEN** the pixels (35, 16) and (35, 24) show A2(483, 80) and A2(483, 88) and pixel (35, 32) is the ground

#### Scenario: The flag on another sheet, measured on the original
- **WHEN** layer 0 of cell (1, 0) of the small map is 4380 and its flag is 0x80
- **THEN** the pixels (35, 16) and (35, 24) show A3(3, 0) and A3(3, 8) and pixel (35, 32) is the ground

#### Scenario: A table in layer 2, measured on the original
- **WHEN** layer 2 of cell (1, 0) of the small map is 2844 and its flag is 0x80
- **THEN** the pixels (35, 16), (35, 24) and (35, 32) show A2(35, 48), A2(35, 80) and A2(35, 88)

#### Scenario: Band of a table, measured on the original
- **WHEN** layer 0 of cell (1, 0) of the small map is 2844 with the flag 0x80 and the corner of the op is (0, -3)
- **THEN** the pixels (35, 28), (35, 29) and (35, 31) show A2(35, 87), A2(35, 88) and A2(35, 90), and pixel (35, 32) is the ground

### Requirement: Edge under a table of RGSS3
In the rule set of RGSS3 the lower plane SHALL draw into rows 0 to 7 of a cell the rows 88
to 95 of the two lower pieces of the table in layer 1 of the map cell above, when that
table's shape has the lower edge, the cell's layer-1 tile is no table and its layer-0 tile
is not from 4352 to 8191. It SHALL be drawn after the cell's layer 1 and shadow and before
its layer 2. The cell above SHALL NOT be looked up across the edge of the map.

#### Scenario: The edge, measured on the original
- **WHEN** in the small map layer 1 of cell (1, 0) is 2844 with the flag 0x80 and layer 0 of cell (1, 1) is 1545
- **THEN** the pixels (35, 32) and (52, 39) show A2(35, 88) and A2(20, 95), and pixel (35, 40) shows A5(35, 40)

#### Scenario: Tables of layers 0 and 2 give none, measured on the original
- **WHEN** the table 2844 lies in layer 0 of cell (1, 0) instead, and again in layer 2
- **THEN** each time the pixels (35, 32) and (52, 39) show A5(35, 32) and A5(52, 39)

#### Scenario: A shape without the lower edge, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2816 with the flag 0x80 and layer 0 of cell (1, 1) is 1545
- **THEN** pixel (35, 32) shows A5(35, 32)

#### Scenario: The tile of layer 1 of the cell, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80, layer 0 of cell (1, 1) is 1545, and layer 1 of cell (1, 1) is first 2844 and then 2864 with the flag 0
- **THEN** pixel (35, 32) shows A2(35, 64) the first time, and A2(35, 88) the second time with A2(99, 72) at pixel (35, 40)

#### Scenario: The tile of layer 0 of the cell, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80 and layer 0 of cell (1, 1) is first 4352 and then 2892 with the flag 0x80
- **THEN** pixel (35, 32) shows A3(35, 32) the first time and A2(35, 88) the second time

#### Scenario: Layer 2 covers the edge, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80, layer 0 of cell (1, 1) is 1545 and layer 2 of cell (1, 1) is 37
- **THEN** pixel (35, 32) shows B(163, 128)

#### Scenario: An empty cell takes the edge, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80 and cell (1, 1) is empty
- **THEN** the pixels (35, 32) and (52, 39) show A2(35, 88) and A2(20, 95) and pixel (35, 40) is the ground

#### Scenario: No look-up across the edge of the map, measured on the original
- **WHEN** layer 1 of cell (1, 2) is 2844 with the flag 0x80 and layer 0 of cell (1, 0) is 1545, so that the plane shows cell (1, 0) again under cell (1, 2)
- **THEN** the pixels (35, 0) and (35, 96) show A5(35, 32) and pixel (35, 95) shows A2(35, 87)

#### Scenario: A star table in layer 1, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x90 and layer 0 of cell (1, 1) is 1545
- **THEN** the lower plane shows A2(35, 68) at pixel (35, 4) and A2(35, 88) at pixel (35, 32)

### Requirement: Shadow mask of RGSS3
In the rule set of RGSS3 no shadow SHALL follow from walls. For each of the bits 0 to 3 set
in the cell's element of layer 3 the lower plane SHALL darken one quarter of the cell, in
this order upper left, upper right, lower left, lower right: each channel `v` becomes
`(v * 127) >> 8`. It SHALL do so after the cell's layer 1 and before the edge under a table
and layer 2. Higher bits SHALL have no effect, and the upper plane SHALL draw no shadow.

#### Scenario: Quarters, measured on the original
- **WHEN** the small map is empty over a ground of level 200 and layer 3 is -1 in cell (1, 1), 1 in cell (2, 1) and 8 in cell (3, 1)
- **THEN** every pixel of cell (1, 1) shows 99; of cell (2, 1) pixel (64, 32) shows 99 and the pixels (80, 32) and (64, 48) show 200; of cell (3, 1) pixel (127, 63) shows 99 and the pixels (112, 47) and (111, 48) show 200; pixel (35, 31) shows 200

#### Scenario: Low bits only, measured on the original
- **WHEN** layer 0 of cell (1, 1) is 1545 and layer 3 of the cell is 5, then 0x3f0a, then 16
- **THEN** with 5 the pixels (35, 40) and (35, 56) show (40, 11, 80) and (40, 11, 112) and the pixels (52, 40) and (52, 56) show A5(52, 40) and A5(52, 56); with 0x3f0a the pixels (52, 40) and (52, 56) show (40, 79, 80) and (40, 79, 112) and the pixels (35, 40) and (35, 56) show A5(35, 40) and A5(35, 56); with 16 nothing is darkened

#### Scenario: Layer 1 is darkened and layer 2 is not, measured on the original
- **WHEN** layer 3 of cell (1, 1) is 15 and the tile 37 lies first in layer 1 and then in layer 2 of the cell
- **THEN** pixel (35, 40) shows (50, 12, 17), the texel B(163, 136) = (101, 26, 35) darkened, the first time and B(163, 136) the second time

#### Scenario: The edge is not darkened, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80, layer 0 of cell (1, 1) is 1545 and layer 3 of cell (1, 1) is 15
- **THEN** the pixels (35, 32) and (35, 39) show A2(35, 88) and A2(35, 95), and pixel (35, 40) shows (40, 11, 80)

#### Scenario: No shadow from walls, measured on the original
- **WHEN** layer 0 of the cells (0, 0) and (0, 1) is 4367 and layer 0 of cell (1, 1) is 2912, with no element of layer 3 set
- **THEN** the pixels (35, 40) and (52, 40) show A2(163, 72) and A2(148, 72)

### Requirement: Star tiles of RGSS3
In the rule set of RGSS3 the upper plane SHALL draw the tiles of layer 2 whose flags entry
has the bit 0x10, and the lower plane every other tile, among them the tiles of layers 0 and
1 that have the bit. No bit of a flags entry other than 0x10 and 0x80 SHALL change what is
drawn.

#### Scenario: A star tile in a lower layer, measured on the original
- **WHEN** the tile 300 with the flag 0x10 lies in layer 0 of cell (1, 1) of the small map, and again in layer 1
- **THEN** each time the lower plane alone shows C(131, 168) at pixel (35, 40), and the upper plane alone leaves the frame as the ground

#### Scenario: A star tile in layer 2, measured on the original
- **WHEN** the tile 300 with the flag 0x10 lies in layer 2 of cell (1, 1)
- **THEN** the upper plane alone shows C(131, 168) at pixel (35, 40), and the lower plane alone leaves the frame as the ground

#### Scenario: A star table on the upper plane, measured on the original
- **WHEN** layer 2 of cell (1, 0) is 2844 with the flag 0x90 and the corner of the op is (0, -3)
- **THEN** the upper plane alone shows A2(35, 71), A2(35, 88) at the pixels (35, 4), (35, 29), and the ground at pixel (35, 32); the lower plane alone leaves the frame as the ground

#### Scenario: The flag of number 0, measured on the original
- **WHEN** the small map is empty and element 0 of the flags table is 0x10
- **THEN** neither plane changes the frame

#### Scenario: Other bits, measured on the original
- **WHEN** the scenarios "A table by its flag" and "A star tile in layer 2" are composed with the flags 0x6f, 0x100 and 0x7f6f in place of 0x80 and 0x10
- **THEN** each frame equals the one composed with the flag 0

### Requirement: Undefined maps and numbers under the rule set of RGSS3
Where the original runtime of RGSS3 reads outside its arrays, a plane op of the rule set of
RGSS3 SHALL read nothing outside its tables: a number from 1024 to 1535 and a negative
number SHALL draw nothing, a layer the map does not have SHALL be empty, the shadow layer
among them, and a layer beyond the fourth SHALL NOT be read.

#### Scenario: Undefined numbers
- **WHEN** layer 0 of the cells (0, 0), (1, 0), (2, 0), (3, 0) and (0, 1) of the small map holds 1024, 1535, -256, -1 and 1300
- **THEN** the frame is the ground everywhere

#### Scenario: Fewer layers
- **WHEN** a map of 4 by 3 cells has three layers with 1545 in layer 0 of cell (1, 1), then one layer with the same, then two dimensions with the same
- **THEN** each time pixel (35, 40) shows A5(35, 40), nothing is darkened and no error is reported

#### Scenario: A fifth layer
- **WHEN** a map of 4 by 3 cells and five layers has 1545 in layer 0 of cell (1, 1), 37 in layer 4 of cell (1, 1) and 15 in layer 4 of cell (2, 1)
- **THEN** pixel (35, 40) shows A5(35, 40) and cell (2, 1) is the ground

### Requirement: Whole tilemaps of RGSS3 as measured
The CPU reference SHALL draw the topics `tables2`, `shadow3` and `edge1` and the first forty
frames of the topic `rand4` of the measurement of RGSS3, rebuilt by formula with generated
sheets, as the model of that measurement renders them. Their expected frames SHALL be
committed as digests that the model produced, not a compositor of this repository.

#### Scenario: Topics
- **WHEN** the 6 frames of `tables2`, the 22 of `shadow3`, the 48 of `edge1` and the first 40 of `rand4` are composed by the CPU reference
- **THEN** every frame has its committed digest

#### Scenario: Nothing of the original
- **WHEN** the files this change adds are listed
- **THEN** none is a picture, a DLL, a script of the publisher or map data of a game, and the file of digests holds only comments, topic names, frame numbers and digests
