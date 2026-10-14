## ADDED Requirements

### Requirement: Recorded text metrics of RGSS3
`docs/decisions.md` SHALL say, in its list of what stands for the answers of GDI in RGSS3,
that advances, overhangs and the average width come from outlines hinted as Wine's FreeType
hints them, with the rules of the decisions 2 to 7 of this change's `design.md`, and why
RGSS1 and RGSS2 keep their advances. The other items of that list SHALL be unchanged.

#### Scenario: Each rule is there
- **WHEN** the part "Text and bitmaps of RGSS3" of `docs/decisions.md` is compared with the decisions 2 to 7 of this change's `design.md`
- **THEN** the choice of the hinting engine, the advance with its fixed-pitch rule, the box and overhangs, the average width, the glyph images and the reason RGSS1 and RGSS2 are unchanged are stated, and the item that called the advance "rounded as FreeType rounds it" is gone

#### Scenario: The rest of the list
- **WHEN** the items on `tmHeight`, the coverage levels, the width for an `lfWidth`, the System font and the font for a missing name are compared with the start of the change
- **THEN** they are unchanged

### Requirement: Text metrics of VX Ace in the documents
`docs/compatibility.md` SHALL say in "Font and text of VX Ace" that text of a game of VX Ace
is measured and placed from glyphs hinted as Wine's FreeType hints them, and SHALL keep its
statements on XP and VX unchanged. What it says about the state of VX Ace SHALL NOT change.

#### Scenario: The sentence is there
- **WHEN** the subsection "Font and text of VX Ace" is read after this change
- **THEN** it says that advances, overhangs and placement follow the hinted glyphs of Wine's FreeType and that glyph shapes still differ

#### Scenario: Nothing else changes
- **WHEN** the README, the agent guide, the opening of `docs/compatibility.md` and its sections on XP and VX are compared with the start of the change
- **THEN** they are unchanged
