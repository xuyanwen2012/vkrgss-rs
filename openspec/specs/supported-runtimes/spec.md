# supported-runtimes Specification

## Purpose
Defines which RPG Maker runtimes the project covers, to what standard and in what order, and
how the documents state what works at a given time.

## Requirements

### Requirement: Runtimes in scope
`docs/decisions.md` SHALL name RGSS1, RGSS2 and RGSS3 as the runtimes in scope and RPG Maker
MV, MZ, 2000 and 2003 as out of scope, and SHALL state that VX and VX Ace are held to the
fidelity standard of XP.

#### Scenario: Scope
- **WHEN** the section "Goal and scope" of `docs/decisions.md` is read
- **THEN** it names the three runtimes as in scope and no longer says that RGSS2 and RGSS3 are out of scope

#### Scenario: Standard
- **WHEN** the same section is read
- **THEN** it says that frames, saves and determinism of VX and VX Ace are compared with the original as those of XP are

### Requirement: Order and milestones of VX
`docs/decisions.md` SHALL state that VX comes before VX Ace, SHALL list the milestones V0 to
V3 of VX with a status that matches the evidence recorded for each, and SHALL state that no
change implementing VX in the guest is drafted before V0 has passed.

#### Scenario: Milestone table
- **WHEN** the section "Milestones" is read
- **THEN** it has the rows V0 to V3 with outcome, status, evidence and what is not covered, below the rows of XP, which are unchanged

#### Scenario: Status follows evidence
- **WHEN** a row says a milestone is done
- **THEN** its evidence names the games, their DLLs, the input schedules and the compared frames; a milestone without such a record says "Not started" or "In progress"

### Requirement: Recorded decisions on several runtimes
`docs/decisions.md` SHALL record the decisions of this change on the structure of the
program, the corpus, the experimental state before V3, the direction for VX Ace and the
process.

#### Scenario: Each decision is there
- **WHEN** `docs/decisions.md` is compared with the section "Decisions" of this change's `design.md`
- **THEN** every decision of that section is stated in the document

### Requirement: Documents say what is supported
The README, the agent guide and `docs/compatibility.md` SHALL state for each runtime in
scope whether it is supported, experimental or planned, and that statement SHALL match the
milestones recorded in `docs/decisions.md`: a runtime is supported when its milestones are
done, experimental while its games start but its milestones are open, planned before.

#### Scenario: README
- **WHEN** the first paragraph of `README.md` is read
- **THEN** it gives the state of XP, VX and VX Ace, each as the milestone table has it

#### Scenario: Agent guide and compatibility
- **WHEN** the first sentence of `AGENTS.md` and the opening of `docs/compatibility.md` are read
- **THEN** neither says more about VX or VX Ace than the milestone table supports

### Requirement: Recorded frame times of VX
`docs/decisions.md` SHALL record, beside the risk that asks for them, the frame times of
representative VX scenes measured before version 0.2.0: the machine and the build, the runs
and their scenes, the mean and the largest value of the stages, and what was not measured.

#### Scenario: The record is there
- **WHEN** the risks of the extension to VX in `docs/decisions.md` are read
- **THEN** the risk on frame times is followed by the measurement of 2026-10-13 with the numbers of this change's `design.md`

### Requirement: Version 0.2.0
The version of the workspace SHALL be 0.2.0, and `CHANGELOG.md` SHALL have a section
`[0.2.0] - 2026-10-13` above that of 0.1.0 which names support for RPG Maker VX.

#### Scenario: Version and lock file
- **WHEN** `cargo metadata --locked --no-deps` is run
- **THEN** it succeeds and gives the package `vkxp` the version 0.2.0

#### Scenario: Documents after V3
- **WHEN** the first paragraph of `README.md`, the first sentence of `AGENTS.md` and the openings of `docs/compatibility.md` and `docs/running.md` are read
- **THEN** each says that XP and VX are supported and that VX Ace is planned, and none calls VX experimental

### Requirement: Order and milestones of VX Ace
`docs/decisions.md` SHALL give VX Ace the milestones A0 (the original runtime under Wine),
A1 (title screen and menu), A2 (walking on a map) and A3 (battle, saving and loading), held
to the standard of V1 to V3, and SHALL say that version 0.3.0 is released when A3 is reached.

#### Scenario: Rows
- **WHEN** the milestone table is read
- **THEN** it has the rows A0 to A3, A0 "Done" with the games, their DLLs, the seeds, the schedule and the repeated run, and A1 to A3 "Not started"

#### Scenario: Limits of A0
- **WHEN** the row A0 is read
- **THEN** its "Not covered" names frames inside a transition, saving, loading and reset, and the single runs of the encrypted games

### Requirement: Recorded decisions on VX Ace
`docs/decisions.md` SHALL record the decisions of this change on VX Ace: the guest
executable and how it is chosen and checked, `--guest thread`, the vendored interpreter and
its sys crate, the one source of the binding, encodings, the hash seed, script threads, the
word size, the order of work, the original side without device nodes, and the process.

#### Scenario: Each decision is there
- **WHEN** the section "VX Ace" and the section "Process" of `docs/decisions.md` are compared with the section "Decisions" of this change's `design.md`
- **THEN** every one of the twelve decisions is stated

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they still say that VX Ace is planned

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

### Requirement: Recorded decisions on the tilemap of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", what vkxp draws where the tilemap of the
original runtime of RGSS3 reads outside its arrays: nothing for the tile numbers 1024 to
1535 and for negative numbers, and an empty layer for each layer a map lacks; and SHALL name
as known differences the negative numbers that draw a block on the original and the maps of
fewer than three layers.

#### Scenario: The decision is there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with decision 4 of this change's `design.md`
- **THEN** the three choices and the two known differences are stated, with the date of the measurement

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they do not say that a game of VX Ace runs, and `docs/compatibility.md` is unchanged

### Requirement: Recorded decisions on the window of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", that vkxp goes on where the window of the
original runtime of RGSS3 ends with a page fault, as a known difference, that the rule for
a window without area rests on few measured pixels, and what of the window of RGSS2 stays
unmeasured. `docs/compatibility.md` SHALL say, for the window of VX, what is now measured of
skins smaller than the layout and what is not.

#### Scenario: The decisions are there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 4 to 7 of this change's `design.md`
- **THEN** it names the calls and sizes at which the original faults and what vkxp does there, the 14 pixels behind the window without area, and, for RGSS2, the cursor from a skin that cuts its block, the list of calls that decide the arrows and disposed bitmaps as not measured, with the date of the measurement

#### Scenario: The window of VX
- **WHEN** the section "The window of VX" of `docs/compatibility.md` is read
- **THEN** it no longer lists the background from a skin narrower or lower than 64 pixels as a difference, and it says that the cursor from a skin that cuts the cursor block was not measured on VX

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they do not say that a game of VX Ace runs, and `docs/compatibility.md` differs from before only in the section "The window of VX"

### Requirement: Recorded decisions on sprites, snapshots and input of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", the decisions 1, 5, 8 and 10 of this
change's `design.md`: that the rules of a scene have a value for RGSS3 and what it changes;
when an input snapshot is seen and what of the original's key messages is not reproduced;
the frozen picture and what of it is decided without a measurement; and what vkxp does where
the original faults or reads outside its sine table.

#### Scenario: The decisions are there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 1, 5, 8 and 10 of this change's `design.md`
- **THEN** each is stated with the date of the measurement, and the known differences name: the count of key messages a frame takes, a key pressed and released between two frames, a disposed viewport given to a move, the sprite that ends the original, and a negative wave length

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they do not say that a game of VX Ace runs, and `docs/compatibility.md` is unchanged

### Requirement: Known differences of the value classes of RGSS3
`docs/compatibility.md` SHALL have, under a section "RPG Maker VX Ace", a subsection on the
value classes that says what `Color`, `Tone`, `Rect` and `Table` of a game of VX Ace take
that those of VX do not, and names as known differences every place where vkxp does not do
what the original runtime of RGSS3 was measured to do. It SHALL NOT change what the
documents say about the state of VX Ace.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with the decisions 4, 6, 7 and 8 and the Follow-ups of this change's `design.md`
- **THEN** it names: `==` of version 3.0.0.1, which raises where vkxp returns false; `set` with an object of another native class; `Color._load` of 32 bytes, which the original does not limit; `Rect._load` and `Table._load` of a short String; the integers and the message of a `RangeError` that depend on the word size; and `Table#[]=` with one index too few

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

#### Scenario: The section is there already
- **WHEN** the change starts from a tree whose `docs/compatibility.md` has a section "RPG Maker VX Ace"
- **THEN** the subsection is added to it, no second section of that name exists, and a sentence of that section that calls the value classes those of RGSS2 is brought up to date

### Requirement: State of VX Ace once its games start
The README, the agent guide, `docs/compatibility.md`, `docs/running.md` and the usage
message SHALL say that RPG Maker VX Ace is experimental, and SHALL NOT say that it is
supported or that a milestone of it is reached. The rows A1 to A3 of the milestone table
SHALL stay "Not started". This statement SHALL replace what earlier scenarios of this
capability say of VX Ace as planned.

#### Scenario: Documents
- **WHEN** the first paragraph and the table of runtimes of `README.md`, the first sentence of `AGENTS.md` and the openings of `docs/compatibility.md` and `docs/running.md` are read
- **THEN** each says that XP and VX are supported and that VX Ace is experimental, and none says that VX Ace is planned or does not run

#### Scenario: What experimental means here
- **WHEN** the section on VX Ace of `docs/compatibility.md` is read
- **THEN** it says that the scripts of a game of VX Ace run in a guest executable of their own with the classes of RGSS3, and that no frame was compared with the original, and it does not say that a game made with the standard scripts ends before its title screen

#### Scenario: Milestones
- **WHEN** the milestone table of `docs/decisions.md` is read
- **THEN** the rows A1, A2 and A3 say "Not started" and the row A0 is unchanged

#### Scenario: README stays within its length
- **WHEN** the lines of `README.md` are counted
- **THEN** there are at most 200

### Requirement: Recorded decisions on the guest executable and the start of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", the decisions 5 to 16 of this change's
`design.md`: where the guest executable is looked for and that nothing overrides it, how
tests reach it, the form of the guest and the experimental state, the preparation, script
loading, data files, reset, endings and the report with what of the original is not
followed, the message boxes and console output, states with script threads, and the
release archive.

#### Scenario: Each decision is there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 5 to 16 of this change's `design.md`
- **THEN** every one of them is stated, and each difference from the original that the design names as not followed is named as a known difference

#### Scenario: The changes that follow
- **WHEN** the section "VX Ace" is read
- **THEN** it says that the class surface of RGSS3 is bound by the changes that follow and names what a title screen of the standard scripts calls of it

### Requirement: The window class of RGSS3 in the documents
`docs/compatibility.md` SHALL have, under the section "RPG Maker VX Ace", a subsection on
the window that says what the class `Window` of a game of VX Ace has, that it is drawn by
the rules measured on the original, where vkxp goes on while the original ends with a page
fault, and what was not measured. It SHALL NOT change what the documents say about the
state of VX Ace.

#### Scenario: What the subsection says
- **WHEN** the subsection is compared with the decisions 2, 4, 6 and 7 and the Follow-ups of this change's `design.md`
- **THEN** it names the constructor without a viewport, the methods RGSS3 adds, the tone object, the calls and sizes at which the original faults with what vkxp does there, a disposed skin or contents bitmap drawn as none, and as not measured or not followed: a window whose viewport is disposed, `viewport=` with a disposed viewport, and the wording of the errors it lists under Follow-ups

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

#### Scenario: The section of VX is left alone
- **WHEN** the difference of `docs` against the start of the change is read
- **THEN** it names `compatibility.md` only, and in it nothing outside the section "RPG Maker VX Ace" changed

### Requirement: Known differences of the scene classes and Graphics of RGSS3
`docs/compatibility.md` SHALL have, under "RPG Maker VX Ace", a subsection on `Sprite`,
`Plane`, `Viewport` and `Graphics` that names as known differences every place where vkxp
does not do what the original of RGSS3 was measured to do, and `docs/decisions.md` SHALL
record what was decided without a measurement. The state of VX Ace in the documents SHALL
NOT change.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with the decisions 4, 6, 8 and 9 and the Follow-ups of this change's `design.md`
- **THEN** it names: a disposed viewport given to `viewport=`, which ends the original and raises `RGSSError` in vkxp; the elements of a disposed viewport, drawn nowhere; `Graphics.play_movie`, which plays nothing; and `Graphics.frame_count` beyond 2^31 - 1, which does not wrap

#### Scenario: The decisions are there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is read
- **THEN** it says that disposing a viewport of RGSS3 leaves its elements alive and that vkxp draws them nowhere, not measured, and how `play_movie` looks for a file, not measured

#### Scenario: VX and the tilemap of VX Ace
- **WHEN** the section "RPG Maker VX" and the subsection on the tilemap of VX Ace of `docs/compatibility.md` are read
- **THEN** the first says that vkxp disposes the elements of a disposed viewport where the original of VX keeps them, and the second no longer says that `Viewport#dispose` disposes a tilemap of VX Ace

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

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

### Requirement: Known differences of Input of RGSS3
`docs/compatibility.md` SHALL have, under "RPG Maker VX Ace", a subsection on `Input` that
says what a game of VX Ace gives `Input` and when it sees a key, that its keys are those of
VX, and names as known differences every place where vkxp does not do what the original of
RGSS3 was measured to do. The state of VX Ace in the documents SHALL NOT change.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with decision 5 of the archived `draw-sprites-and-snapshots-as-rgss3` and the Follow-ups of this change's `design.md`
- **THEN** it names: the count of key messages a frame of the original takes, a key pressed and released between two frames, the key the original takes during `Kernel#sleep`, and the key functions of `Win32API`, which follow the rules of VX

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

### Requirement: Win32API and DL of VX Ace in the documents
The section "RPG Maker VX Ace" of `docs/compatibility.md` SHALL say what `Win32API` and `DL`
of a game of VX Ace do as measured on the original, which libraries vkxp finds, and SHALL
list as known differences, each with its reason, the rules vkxp does not follow. It SHALL
NOT change what the documents say about the state of VX Ace.

#### Scenario: What the section says
- **WHEN** the section is compared with the decisions 1 to 10 and the Follow-ups of this change's `design.md`
- **THEN** it names the class and its declarations, the libraries found with the list of system libraries, the conversions and the low 32 bits with the reason of the object ids, the values of the exports, the bare profile name, the refused address trick, `DL` refused, and that functions of a library a game ships are not run

#### Scenario: The old statement is gone
- **WHEN** the list of known differences of the start of VX Ace is read
- **THEN** it no longer says that `Win32API` and `DL` are not those of RGSS3

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged

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
