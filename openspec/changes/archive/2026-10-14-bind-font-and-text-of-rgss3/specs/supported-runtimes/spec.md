## ADDED Requirements

### Requirement: Known differences of font and text of RGSS3
`docs/compatibility.md` SHALL have, in its section "RPG Maker VX Ace", a subsection on font
and text that says what `Font` and the text and creation of `Bitmap` of a game of VX Ace do
that those of VX do not, and names as known differences every place where vkxp does not do
what the original runtime of RGSS3 was measured to do. It SHALL NOT change what the
documents say about the state of VX Ace.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with the decisions 3, 8 and 11 and the first item of the Follow-ups of this change's `design.md`
- **THEN** it names: the largest bitmap; text wider than 640 pixels; `text_size` of a font without a name; the vertical names beginning with `@`; substituted names that count as existing; the message when "VL Gothic" is missing; the other language's name of the two fonts of the run-time package; and that what GDI answers is Wine's

#### Scenario: The version is no longer a difference
- **WHEN** the section "RPG Maker VX Ace" is read after this change
- **THEN** it no longer says that `RGSS_VERSION` is `"3.0.1"` whatever DLL the game ships, it says that the version and the glyph rule follow the DLL, and no sentence of it calls `Font` or text those of VX

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

### Requirement: Recorded application of the DLL version of RGSS3
`docs/decisions.md` SHALL say, under "VX Ace", how the table from DLL version to glyph rule
is applied: the guest reads the version from the file `Library` names before the first
script, with no field of the guest record; `RGSS_VERSION` follows the same table; and a game
of RGSS3 takes fonts from the `Fonts` directories of its run-time packages, with no
substitute for "VL Gothic".

#### Scenario: Each point is there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 10 and 11 of this change's `design.md`
- **THEN** both are stated, and no sentence of the section says any longer that applying the table waits for the binding or that `RGSS_VERSION` does not follow the DLL

#### Scenario: Where to run
- **WHEN** `docs/running.md` is read after this change
- **THEN** the row of `--font-dir` and the text on VX Ace name the `Fonts` directories of the run-time packages for a game of VX Ace
