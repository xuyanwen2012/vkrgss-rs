## ADDED Requirements

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
- **WHEN** the tests of `vkxp-core` that existed before this change run
- **THEN** they pass with no expected value and no file under `crates/vkxp-core/tests/data/` edited

### Requirement: Turned sprites in viewports of RGSS3
In a scene of RGSS3 a sprite with an angle inside a viewport SHALL be drawn as a sprite
without a viewport that stands at its own position plus the viewport's rectangle position
less the viewport's origin, clipped to the viewport's rectangle: the bounding box and the
source positions SHALL both be computed from that position.

#### Scenario: Half turn, measured rule
- **WHEN** in a scene of RGSS3 a viewport has the rectangle (100, 50, 60, 40) and the origin (10, 4), and holds a sprite at (30, 20) with angle 180 of an opaque bitmap 16 by 8 whose texel column c is (c, 255 - c, 128)
- **THEN** exactly 120 pixels are drawn, the columns 105 to 119 of the rows 58 to 65, and in each row column 105 shows red 15 and column 119 red 1

#### Scenario: Quarter turn, measured rule
- **WHEN** the same sprite has angle 90
- **THEN** exactly 105 pixels are drawn, the columns 121 to 127 of the rows 51 to 65

#### Scenario: As on the screen
- **WHEN** the frames of those two scenes are compared with the frames of the same sprite without a viewport at (120, 66)
- **THEN** they are equal byte for byte

#### Scenario: Clipped by the viewport
- **WHEN** the viewport of the half turn has the rectangle (100, 50, 15, 10) instead
- **THEN** exactly 20 pixels are drawn, the columns 105 to 114 of the rows 58 and 59

#### Scenario: A large origin only moves the sprite
- **WHEN** in a scene of RGSS3 a viewport has the rectangle (100, 50, 200, 150) and the origin (-50, -30) and holds that sprite at (30, 20) with angle 180
- **THEN** exactly 120 pixels are drawn, the columns 165 to 179 of the rows 92 to 99

#### Scenario: Source rectangle leaving the bitmap
- **WHEN** in a scene of RGSS3 a sprite with angle 30 in a viewport has the source rectangle (-1, 0, 24, 16) on a bitmap of 24 by 16
- **THEN** nothing of it is drawn

### Requirement: Viewport of the screen and a change of its size
The scene SHALL be able to create a viewport without a rectangle, whose rectangle is
(0, 0, width, height) of the logical size at that moment. A change of the logical size SHALL
change no rectangle of a viewport, no coordinate of an element, and neither the brightness,
the frame count nor the frame rate; each viewport SHALL clip to the part of its rectangle
inside the new size, and elements without a viewport SHALL be drawn over the whole new size.

#### Scenario: Size of the moment, measured on the original of RGSS3
- **WHEN** a scene of 544 by 416 creates a viewport without a rectangle, and again after its size was set to 640 by 480, to 320 by 240, to 200 by 300 and to 1 by 1
- **THEN** the rectangles read (0, 0, 544, 416), (0, 0, 640, 480), (0, 0, 320, 240), (0, 0, 200, 300) and (0, 0, 1, 1), and the first still reads (0, 0, 544, 416) at the end

#### Scenario: Larger screen, measured on the original of RGSS3
- **WHEN** a scene of 544 by 416 holds a white plane in a viewport made without a rectangle, or one in a viewport of (100, 100, 600, 500), or one without a viewport, and its size is set to 640 by 480
- **THEN** the first plane covers exactly the columns 0 to 543 of the rows 0 to 415, the second the columns 100 to 639 of the rows 100 to 479, and the third all 640 by 480 pixels

#### Scenario: Smaller screen, measured on the original of RGSS3
- **WHEN** the size of that scene is set to 320 by 240
- **THEN** the first plane covers all 320 by 240 pixels and the second the columns 100 to 319 of the rows 100 to 239

#### Scenario: Bookkeeping is kept
- **WHEN** a scene with brightness 100, frame count 7 and frame rate 30 has its size set to 640 by 480
- **THEN** it reads brightness 100, frame count 7 and frame rate 30, and no frame was extracted

### Requirement: Elements moved between viewports
An element moved to another viewport, or to none, SHALL be drawn by the clip and origin of
its new place and SHALL keep the rank its creation gave it among the elements of equal z
there, whatever the rules of the scene. Moving an element into a disposed viewport SHALL
fail with a disposed error and move nothing.

#### Scenario: A plane follows its viewport, measured on the original of RGSS3
- **WHEN** a plane with `ox = 5` of an opaque bitmap 24 by 16 whose texel column c is (c, 255 - c, 128) is created in a viewport of (4, 4, 60, 50) and then moved to a viewport of (70, 20, 80, 90) with the origin (7, 3)
- **THEN** before the move pixel (4, 4) shows red 5 and pixel (64, 54) nothing; after it pixel (4, 4) shows nothing, (70, 20) red 12, (149, 109) red 19 and (150, 110) nothing

#### Scenario: To the screen
- **WHEN** that plane is then moved out of any viewport on a screen of 544 by 416
- **THEN** pixel (0, 0) shows red 5 and pixel (543, 415) red 20

#### Scenario: The rank stays, measured on the original of RGSS3
- **WHEN** a red plane is created in one viewport and then a green sprite of equal z in another viewport with the same rectangle, and the plane is moved into the sprite's viewport; and again with the sprite created first
- **THEN** where both lie the first frame shows green and the second red

#### Scenario: Disposed viewport
- **WHEN** a sprite and a plane are moved into a viewport that was disposed
- **THEN** each move fails with the disposed error of a viewport, the elements stay where they were and the next frame is extracted without an error

### Requirement: Sprites at which the original of RGSS3 faults
The scene SHALL draw by its rules, without an error, the sprites at which the original
runtime of RGSS3 ends with a page fault or reads outside its sine table: such a sprite SHALL
be extracted into a packet both compositors accept, and a negative wave length SHALL take
the angle of the wave modulo 360.

#### Scenario: The sprite that ends the original
- **WHEN** in a scene of RGSS3 a sprite of an opaque white bitmap 20 by 90 with `src_rect` (8, 38, 9, 26), at (434, 160), `zoom_x` 2.5, `zoom_y` 0.7, angle 180, mirrored, with `bush_depth` 20, is extracted and composed
- **THEN** no error occurs and exactly 360 pixels are drawn, the columns 414 to 433 of the rows 142 to 159

#### Scenario: Negative wave length
- **WHEN** the wave offset is computed for amplitude 10, length -50 and phase 0 at row 8 of a sprite whose top is screen row 0
- **THEN** it is -8, the rounded product of 10 and the sine of 302 degrees

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
- **WHEN** the files this change adds under `crates/vkxp-core/tests/data/` are read
- **THEN** they hold scenes, numbers and digests only, and the head of each names the topic, the model and the DLL versions of the measurement

## MODIFIED Requirements

### Requirement: Turned sprites in viewports and with cut source rectangles
In a scene of RGSS1 or RGSS2 the bounding box of a sprite with an angle inside a viewport
SHALL be computed from the position less the viewport's origin and placed at the viewport's
corner, while the source positions are computed from the position itself; a scene of RGSS3
SHALL follow "Turned sprites in viewports of RGSS3" instead. In every scene a sprite with an
angle whose source rectangle does not lie entirely inside its bitmap SHALL not be drawn.

#### Scenario: Viewport with an origin
- **WHEN** each committed case of a turned sprite in a viewport measured on the original runtime (origins (0,0), (13,7), (-9,-5) and (100,-50), viewports partly off screen) is extracted and rendered
- **THEN** every pixel of the listed rows equals the measured one, and the cases with the origin (100,-50) draw nothing

#### Scenario: Source rectangle leaving the bitmap
- **WHEN** a sprite with angle 30 has the source rectangle (-1, 0, 24, 16) on a bitmap of 24 by 16
- **THEN** nothing of it is drawn

#### Scenario: The shift stays in RGSS1 and RGSS2
- **WHEN** in a scene of RGSS1, and in one of RGSS2, a viewport has the rectangle (100, 50, 60, 40) and the origin (10, 4), and holds a sprite at (30, 20) of an opaque bitmap 16 by 8 whose texel column c is (c, 255 - c, 128), with angle 180 and then with angle 90
- **THEN** with angle 180 exactly 20 pixels are drawn, the columns 115 to 119 of the rows 62 to 65, showing red 15 down to 11 in each row, and with angle 90 nothing is drawn
