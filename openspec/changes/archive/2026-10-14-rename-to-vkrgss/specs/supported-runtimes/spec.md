## MODIFIED Requirements

### Requirement: Recorded decisions on text and bitmaps of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", the decisions 2, 7 and 9 of this change's
`design.md`: which DLL version vkrgss follows and how the glyph rule of the other is selected,
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
`docs/decisions.md` SHALL record, under "VX Ace", what vkrgss draws where the tilemap of the
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
`docs/decisions.md` SHALL record, under "VX Ace", that vkrgss goes on where the window of the
original runtime of RGSS3 ends with a page fault, as a known difference, that the rule for
a window without area rests on few measured pixels, and what of the window of RGSS2 stays
unmeasured. `docs/compatibility.md` SHALL say, for the window of VX, what is now measured of
skins smaller than the layout and what is not.

#### Scenario: The decisions are there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 4 to 7 of this change's `design.md`
- **THEN** it names the calls and sizes at which the original faults and what vkrgss does there, the 14 pixels behind the window without area, and, for RGSS2, the cursor from a skin that cuts its block, the list of calls that decide the arrows and disposed bitmaps as not measured, with the date of the measurement

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
the frozen picture and what of it is decided without a measurement; and what vkrgss does where
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
that those of VX do not, and names as known differences every place where vkrgss does not do
what the original runtime of RGSS3 was measured to do. It SHALL NOT change what the
documents say about the state of VX Ace.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with the decisions 4, 6, 7 and 8 and the Follow-ups of this change's `design.md`
- **THEN** it names: `==` of version 3.0.0.1, which raises where vkrgss returns false; `set` with an object of another native class; `Color._load` of 32 bytes, which the original does not limit; `Rect._load` and `Table._load` of a short String; the integers and the message of a `RangeError` that depend on the word size; and `Table#[]=` with one index too few

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

#### Scenario: The section is there already
- **WHEN** the change starts from a tree whose `docs/compatibility.md` has a section "RPG Maker VX Ace"
- **THEN** the subsection is added to it, no second section of that name exists, and a sentence of that section that calls the value classes those of RGSS2 is brought up to date

### Requirement: The window class of RGSS3 in the documents
`docs/compatibility.md` SHALL have, under the section "RPG Maker VX Ace", a subsection on
the window that says what the class `Window` of a game of VX Ace has, that it is drawn by
the rules measured on the original, where vkrgss goes on while the original ends with a page
fault, and what was not measured. It SHALL NOT change what the documents say about the
state of VX Ace.

#### Scenario: What the subsection says
- **WHEN** the subsection is compared with the decisions 2, 4, 6 and 7 and the Follow-ups of this change's `design.md`
- **THEN** it names the constructor without a viewport, the methods RGSS3 adds, the tone object, the calls and sizes at which the original faults with what vkrgss does there, a disposed skin or contents bitmap drawn as none, and as not measured or not followed: a window whose viewport is disposed, `viewport=` with a disposed viewport, and the wording of the errors it lists under Follow-ups

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

#### Scenario: The section of VX is left alone
- **WHEN** the difference of `docs` against the start of the change is read
- **THEN** it names `compatibility.md` only, and in it nothing outside the section "RPG Maker VX Ace" changed

### Requirement: Known differences of the scene classes and Graphics of RGSS3
`docs/compatibility.md` SHALL have, under "RPG Maker VX Ace", a subsection on `Sprite`,
`Plane`, `Viewport` and `Graphics` that names as known differences every place where vkrgss
does not do what the original of RGSS3 was measured to do, and `docs/decisions.md` SHALL
record what was decided without a measurement. The state of VX Ace in the documents SHALL
NOT change.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with the decisions 4, 6, 8 and 9 and the Follow-ups of this change's `design.md`
- **THEN** it names: a disposed viewport given to `viewport=`, which ends the original and raises `RGSSError` in vkrgss; the elements of a disposed viewport, drawn nowhere; `Graphics.play_movie`, which plays nothing; and `Graphics.frame_count` beyond 2^31 - 1, which does not wrap

#### Scenario: The decisions are there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is read
- **THEN** it says that disposing a viewport of RGSS3 leaves its elements alive and that vkrgss draws them nowhere, not measured, and how `play_movie` looks for a file, not measured

#### Scenario: VX and the tilemap of VX Ace
- **WHEN** the section "RPG Maker VX" and the subsection on the tilemap of VX Ace of `docs/compatibility.md` are read
- **THEN** the first says that vkrgss disposes the elements of a disposed viewport where the original of VX keeps them, and the second no longer says that `Viewport#dispose` disposes a tilemap of VX Ace

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

### Requirement: Known differences of font and text of RGSS3
`docs/compatibility.md` SHALL have, in its section "RPG Maker VX Ace", a subsection on font
and text that says what `Font` and the text and creation of `Bitmap` of a game of VX Ace do
that those of VX do not, and names as known differences every place where vkrgss does not do
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

### Requirement: Known differences of Input of RGSS3
`docs/compatibility.md` SHALL have, under "RPG Maker VX Ace", a subsection on `Input` that
says what a game of VX Ace gives `Input` and when it sees a key, that its keys are those of
VX, and names as known differences every place where vkrgss does not do what the original of
RGSS3 was measured to do. The state of VX Ace in the documents SHALL NOT change.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with decision 5 of the archived `draw-sprites-and-snapshots-as-rgss3` and the Follow-ups of this change's `design.md`
- **THEN** it names: the count of key messages a frame of the original takes, a key pressed and released between two frames, the key the original takes during `Kernel#sleep`, and the key functions of `Win32API`, which follow the rules of VX

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

### Requirement: Win32API and DL of VX Ace in the documents
The section "RPG Maker VX Ace" of `docs/compatibility.md` SHALL say what `Win32API` and `DL`
of a game of VX Ace do as measured on the original, which libraries vkrgss finds, and SHALL
list as known differences, each with its reason, the rules vkrgss does not follow. It SHALL
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

### Requirement: Endings and loading of VX Ace in the documents
The section "RPG Maker VX Ace" of `docs/compatibility.md` SHALL say which rules of the
endings, of `load_data`, of the program name, of the title, of magic comments and of
loading code are those measured on the original, and SHALL list as known differences, each
with its reason, the rules vkrgss does not follow. It SHALL NOT change what the documents
say about the state of VX Ace.

#### Scenario: What the section says
- **WHEN** the section is compared with the decisions 1 to 12 and the Follow-ups of this change's `design.md`
- **THEN** it names the long report shown whole and status 1 after an error with their reasons, `Failed to load script.` with status 1, `at_exit`, `exit!` and the status it gives, `$0`, `Untitled`, the magic comments left to the encodings made at first use, the source, names, lookup and errors of `load_data`, a path outside the game directory, `require_relative`, `autoload` raising at the call, and a required file compiled as UTF-8

#### Scenario: Old statements are gone
- **WHEN** the list of known differences of the start of VX Ace is read
- **THEN** it no longer says that `at_exit` handlers are not run as on the original while `exit!` is not followed, that a failed archive shows no `Failed to load script.`, or that `load_data` freezes no strings and has the errors and lookup of RGSS2

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged
