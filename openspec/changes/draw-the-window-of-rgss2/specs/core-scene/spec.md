## ADDED Requirements

### Requirement: Window of RGSS2
A window of a game of RGSS2 SHALL be drawn by the requirements named "... of a window of
RGSS2" in place of the requirements "Window ... as in the original", which describe RGSS1. It
SHALL be one element at its z, with no part at another z, and SHALL draw nothing when its
width or height is 0 or less.

#### Scenario: One layer, measured on the original
- **WHEN** a window of RGSS2 at z 100 with opaque contents overlaps sprites at z 101 and 102
- **THEN** both sprites are drawn over every part of the window, the contents included

#### Scenario: XP unchanged
- **WHEN** the existing tests of the window of RGSS1 run
- **THEN** they pass with no change to an expected value

### Requirement: Background and frame of a window of RGSS2
The background SHALL be the skin's (0,0,64,64) stretched to (W-4) by (H-4) with the tiled
pattern (0,64,64,64) laid over it, drawn at (2,2) with the alpha
`(a * ((back_opacity * opacity) / 255) + 127) / 255`. The frame SHALL be built from the corner
and edge rectangles of the skin, edges tiled in pieces of 32, and drawn with the alpha
`(a * opacity + 127) / 255`. Stretch, pattern and blend SHALL follow rules 1 to 3 of this
change's design.

#### Scenario: Cases of the measurement
- **WHEN** each committed case of background and frame (sizes even and odd, below and above 32 and 64, opacities, skins with alpha) is extracted and composed
- **THEN** every pixel equals the expected picture of the case

#### Scenario: Pattern keeps the alpha
- **WHEN** the background of the skin has alpha 128 and the pattern alpha 255
- **THEN** the background bitmap has the colour of the pattern and the alpha 128

### Requirement: Openness of a window of RGSS2
With openness o below 255 the window SHALL show `n = trunc(H * s)` rows from row
`top = trunc(H * (1 - s) / 2)`, where `s = o / 255.0` in double precision, screen row
`top + i` showing window row `stretch_index(i, H, n)` of background and frame, every pixel in
the normal blend. Contents, cursor, arrows and pause sign SHALL be drawn only at 255.

#### Scenario: Table of the measurement
- **WHEN** n and top are computed for each committed pair of height and openness measured on the original
- **THEN** both equal the measured values

#### Scenario: Half open
- **WHEN** a window of height 100 with contents has openness 128
- **THEN** rows 24 to 73 of its place show the sampled background and frame, the rows outside show what is underneath, and no contents are drawn

### Requirement: Contents and cursor of a window of RGSS2
Contents SHALL be drawn at (16 - ox, 16 - oy), clipped to the inner rectangle, with the alpha
`(a * contents_opacity + 127) / 255`, over the cursor and also without a skin. The cursor
SHALL be drawn at (16 + x, 16 + y) of the window, not moved by ox and oy, clipped by the
window, with the opacity `max(0, contents_opacity - 8 * m)`, where `m = c` for c up to 20 and
`40 - c` above, c advancing by one modulo 40 per update of an active window and becoming 16
when the window is made inactive.

#### Scenario: Blink, measured on the original
- **WHEN** an active window is updated 80 times and the cursor opacity is read after each
- **THEN** the sequence equals the committed measured sequence, which has the period 40

#### Scenario: Inactive
- **WHEN** `active = false` is set on a window with contents_opacity 255
- **THEN** the cursor is drawn at opacity 127 at once and stays there over later updates

#### Scenario: Cases of the measurement
- **WHEN** each committed case of contents and cursor (ox, oy, contents larger and smaller than the window, cursor sizes, cursor partly outside) is extracted and composed
- **THEN** every pixel equals the expected picture of the case

### Requirement: Pause sign and arrows of a window of RGSS2
The pause sign SHALL be image `(p / 16) mod 4` of the skin at (W/2 - 8, H - 16) with alpha
`min(255, 32 * p)`, p being 0 when `pause` turns true and one more per update. The arrows
SHALL be drawn in the order down, left, right, up at the places and under the conditions of
rule 8 of this change's design. No opacity of the window SHALL apply to either.

#### Scenario: Pause sequence, measured on the original
- **WHEN** `pause = true` is set and the window is updated 70 times
- **THEN** the image index and alpha after each update equal the committed measured sequence

#### Scenario: Arrows
- **WHEN** a window of 100 by 100 has contents of 200 by 200 and `ox = 10`, `oy = 0`
- **THEN** the down, left and right arrows are drawn and the up arrow is not

### Requirement: Latched state of a window of RGSS2
Whether the contents are shown, which arrows are drawn and from which skin, and the
background, frame and cursor bitmaps SHALL change only at the calls rule 9 of this change's
design names, and SHALL otherwise stay as they were.

#### Scenario: Contents given while hidden, measured on the original
- **WHEN** a window is hidden, gets contents, and is shown again with `visible = true`
- **THEN** the next frame shows the window without its contents, and the frame after an `update` shows them

#### Scenario: Skin pixels changed later, measured on the original
- **WHEN** the pixels of the skin bitmap are changed after `windowskin=` and no other call follows
- **THEN** the background and the frame are drawn from the old pixels and the pause sign from the new

#### Scenario: Width set to the same value
- **WHEN** the pixels of the skin are changed and `width = width` is evaluated
- **THEN** the background and the frame are drawn from the new pixels

### Requirement: Whole windows of RGSS2 as measured
The CPU reference and the Vulkan backend SHALL draw scenes of windows of RGSS2 as the model of
the measurement does.

#### Scenario: Random scenes
- **WHEN** thirty committed scenes of the measurement's random generator are extracted and composed by both backends
- **THEN** every frame equals the expected picture of its scene, byte for byte
