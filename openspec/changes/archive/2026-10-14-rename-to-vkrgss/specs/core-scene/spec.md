## MODIFIED Requirements

### Requirement: Tilemaps of RGSS1 and RGSS2 stay as they are
Adding the tilemap of RGSS3 SHALL NOT change what a tilemap of RGSS1 or of RGSS2 extracts:
a tilemap of RGSS2 names the rule set of RGSS2 in its plane ops, and no expected value of an
existing test of tilemaps is edited.

#### Scenario: Existing tests
- **WHEN** the tests of `crates/vkrgss-core/src/tilemap/tests.rs` and `crates/vkrgss-core/src/tilemap/rgss2/tests.rs` that existed before this change run
- **THEN** they pass with their expected values as they were, and every line that `crates/vkrgss-core/tests/data/tilemap-frames.txt` had is unchanged

#### Scenario: Rule set of RGSS2
- **WHEN** a tilemap created as one of RGSS2 is extracted
- **THEN** both its plane ops name the rule set of RGSS2

### Requirement: Skins smaller than the layout
In windows of RGSS2 and RGSS3 a stretched background block that leaves the skin SHALL be cut
and its target shortened: per axis, with S = 64, A what the skin has and D the target, a
copy draws A columns, the nearest stretch `ceil(A * D / S) - 1`, the bilinear
`ceil((A - 1) * D / (S - 1)) - 1`. In RGSS3 this SHALL also hold for centre and edges of the
cursor, and a part cut by the skin's edge SHALL count its cut width in the odd-column rule.

#### Scenario: Background, measured on the originals of VX and VX Ace
- **WHEN** a window of RGSS2 and one of RGSS3, both with back opacity 255 and a flat opaque red skin, are drawn with a skin of 32 by 32 in a window of 76 by 132, of 63 by 128 in 76 by 68, of 40 by 128 in 24 by 24 and in 68 by 68, of 32 by 32 in 24 by 24, and of 1 by 1 in 76 by 76
- **THEN** in both the red pixels fill (2 .. 37, 2 .. 64), 2170 pixels; (2 .. 72, 2 .. 66), 4480; (2 .. 14, 2 .. 22), 240; (2 .. 42, 2 .. 66), 2560; (2 .. 11, 2 .. 11), 81; and none, and no other pixel is drawn

#### Scenario: Two pictures of RGSS2 corrected, measured on the original of VX
- **WHEN** the scenes `g6-skin_sizes_t` and `g8-smallskin_t_0` of `crates/vkrgss-core/tests/data/window-rgss2-cases.txt` are composed
- **THEN** the digests of their frames are `edbddf94f4992ce6` and `0263b5710a3a9da6`, which are those of the pictures of the original runtime of VX, and the file no longer names them as not measured

#### Scenario: Nothing else of RGSS2 changes
- **WHEN** `window-rgss2-cases.txt` and `window-rgss2-random.txt` are compared with what they were before this change
- **THEN** every scene and every other digest is as it was: 145 of the 147 frames keep their digest

#### Scenario: Cursor of RGSS3, measured on the original
- **WHEN** a window of RGSS3 of 100 by 100 has the left 80 columns of the piece skin as its skin and `cursor_rect` (0, 0, 32, 32), then (0, 0, 16, 16), then (0, 0, 40, 40)
- **THEN** the cursor's colour fills (8 .. 24, 8 .. 40), 512 pixels; then (8 .. 15, 8 .. 24), 112; then (8 .. 27, 8 .. 48), 760

### Requirement: Scene of RGSS3
A scene SHALL be creatable with the rules of RGSS3, a third value beside those of RGSS1 and
RGSS2, which it keeps when cleared and gives to its sprites. Such a scene SHALL follow every
requirement that names a scene of RGSS2, except where a requirement names RGSS3.

#### Scenario: Scenes that RGSS3 shares with RGSS2, measured on the original
- **WHEN** every scene of `sprite-rgss2-place.txt`, `sprite-rgss2-bush.txt` and `sprite-rgss2-wave.txt` and every frame of the topics `order`, `viewport`, `flash`, `plane` and `size2` of `rgss2-scene-cases.txt` is extracted from a scene of RGSS3 and composed
- **THEN** every frame has the digest committed for RGSS2 in those files

#### Scenario: y orders sprites of equal z
- **WHEN** in a scene of RGSS3 two overlapping sprites of equal z are made, the first with y 9 and the second with y 0
- **THEN** the first is drawn over the second

#### Scenario: Flash without a colour
- **WHEN** in a scene of RGSS3 a viewport with a tone and a sprite flashes with no colour for 5 updates
- **THEN** the frames of those updates show what is behind the viewport unchanged

#### Scenario: Plane tile and sprite width
- **WHEN** in a scene of RGSS3 a plane of a bitmap 90 high has `zoom_y = 0.7`, and a sprite 90 wide with `zoom_x = 0.7` stands at x 272
- **THEN** the plane repeats every 62 rows and the sprite is 63 pixels wide

#### Scenario: Kept by a clear
- **WHEN** a scene of RGSS3 is cleared and a sprite is created in it
- **THEN** the scene reports the rules of RGSS3 and the sprite is drawn by them

#### Scenario: RGSS1 and RGSS2 unchanged
- **WHEN** the tests of `vkrgss-core` that existed before this change run
- **THEN** they pass with no expected value and no file under `crates/vkrgss-core/tests/data/` edited

### Requirement: Whole scenes of sprites of RGSS3 as measured
The CPU reference and the Vulkan backend SHALL draw scenes of sprites, planes and viewports
of RGSS3 as the model of the measurement does, and SHALL agree with each other byte for
byte. The committed scenes SHALL be made by formula and their expected pictures SHALL be
digests of the model's frames.

#### Scenario: Turned sprites in viewports
- **WHEN** the 8 frames of the topic `atvp` and the 4 frames of the topic `turnedvp` are extracted from a scene of RGSS3 and composed by the reference compositor
- **THEN** every frame has the digest of the frame the model of the measurement draws with its rule of RGSS3

#### Scenario: Moved planes and sprites
- **WHEN** the 6 frames of the topic `aplanevp` are extracted from a scene of RGSS3 and composed by the reference compositor
- **THEN** every frame has the digest of the model's frame

#### Scenario: Random scenes held out
- **WHEN** the committed scenes of the held-out set `aheldx`, twenty less any that the head of their file names as left out with its reason, are extracted from a scene of RGSS3 and composed by the reference compositor
- **THEN** every frame has the digest of the model's frame

#### Scenario: Both backends
- **WHEN** every frame of the three files of scenes of RGSS3 is composed by the reference compositor and by the Vulkan backend
- **THEN** the two frames are equal byte for byte and have the committed digest

#### Scenario: Not the rule of RGSS2
- **WHEN** the frames of `atvp` are extracted from a scene of RGSS2
- **THEN** at least one frame does not have the digest committed for RGSS3

#### Scenario: Nothing of the original
- **WHEN** the files this change adds under `crates/vkrgss-core/tests/data/` are read
- **THEN** they hold scenes, numbers and digests only, and the head of each names the topic, the model and the DLL versions of the measurement

### Requirement: Disposing a viewport without its elements
The scene SHALL offer, beside disposing a viewport with its elements, disposing a viewport
alone: the viewport SHALL be gone, and every sprite, plane, window and tilemap in it SHALL
stay alive, keep its viewport handle and be drawn nowhere until it is moved out. A disposed
or unknown handle SHALL change nothing. Disposing a viewport with its elements SHALL stay as
it is.

#### Scenario: The elements stay
- **WHEN** a sprite, a plane, a window of RGSS3 and a tilemap of RGSS3 are created in a viewport, a sprite is created without one, and the viewport is disposed alone
- **THEN** the viewport is disposed, none of the five elements is, the four report the viewport handle, and their properties can be read and set

#### Scenario: Drawn nowhere
- **WHEN** each of the four had drawn opaque pixels inside the viewport's rectangle and a frame is extracted and composed after the viewport is disposed alone
- **THEN** the frame is extracted without an error and shows only the sprite without a viewport

#### Scenario: Moved out
- **WHEN** the sprite of that viewport is then moved out of any viewport
- **THEN** it is drawn on the screen at its position, and moving it into the disposed viewport fails with the disposed error of a viewport

#### Scenario: Twice and a reused slot
- **WHEN** the same handle is disposed alone again, and a new viewport that takes its slot is created
- **THEN** nothing changes, and the four elements are not in the new viewport

#### Scenario: With the elements, as before
- **WHEN** the tests of `vkrgss-core` that dispose a viewport with its elements run
- **THEN** they pass with their expected values unedited
