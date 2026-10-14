## MODIFIED Requirements

### Requirement: Rule set of a tilemap plane op
A tilemap plane op SHALL be evaluated by the rule set it names. With that of RGSS2 nothing
changes. With that of RGSS3 the numbering, sheets, autotile shapes, animation, plane size,
wrapping, blend, flash and the drawing of a table tile SHALL be those of RGSS2, and tables,
the edge under a table, shadows and star tiles SHALL follow the requirements of RGSS3 below.

#### Scenario: RGSS2 unchanged
- **WHEN** the tests of `crates/vkrgss-compose-ref/tests/tilemap_rgss2.rs` run with their ops naming the rule set of RGSS2
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
