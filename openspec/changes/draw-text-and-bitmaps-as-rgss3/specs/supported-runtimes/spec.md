## ADDED Requirements

### Requirement: Recorded decisions on text and bitmaps of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", the decisions 2, 7 and 9 of this change's
`design.md`: which DLL version vkxp follows and how the glyph rule of the other is selected,
with the table from version to rule and the known difference of `Color#==`; the largest
bitmap of RGSS3 and its reason; and what stands for each answer of GDI, with what is an
accepted limit.

#### Scenario: Each decision is there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 2, 7 and 9 of this change's `design.md`
- **THEN** every one of the three is stated, with the version table of decision 2 and the list of decision 9

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they still say that VX Ace is planned, and `docs/compatibility.md` is unchanged
