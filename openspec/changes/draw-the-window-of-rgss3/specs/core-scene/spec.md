## ADDED Requirements

### Requirement: Window of RGSS3
The scene SHALL be able to create a window of RGSS3, drawn by the requirements named "... of
a window of RGSS3" and, where they say nothing, as a window of RGSS2. It SHALL start with z
100, back opacity 192, padding 8, bottom padding 8, arrows allowed, a neutral tone and
openness 255, and SHALL be one element at its z. Assigning `padding` SHALL set both paddings,
`padding_bottom` the lower one alone, neither clamped.

#### Scenario: Defaults, measured on the original
- **WHEN** a window of RGSS3 is created and its values are read
- **THEN** z is 100, the back opacity 192, the padding 8, the bottom padding 8, arrows are allowed, the tone is (0, 0, 0, 0) and the openness is 255

#### Scenario: Among sprites at its default z, measured on the original
- **WHEN** a window of 100 by 100 with the piece skin and back opacity 255 shares the scene with a flat green sprite of 100 by 100 at z 99 and a flat red sprite of 32 by 32 at z 101, all at (0, 0)
- **THEN** the frame has no green pixel and 1024 red pixels

#### Scenario: Paddings, measured on the original
- **WHEN** `padding = 12` is assigned, then `padding_bottom = 20`, then `padding = -5`, then `padding_bottom = 100000`, the two values being read after each
- **THEN** they are (12, 12), (12, 20), (-5, -5) and (-5, 100000)

#### Scenario: A window of another rule set ignores the new calls
- **WHEN** a window of RGSS1 and one of RGSS2 take the calls of `padding`, `padding_bottom` and `arrows_visible`
- **THEN** both are drawn as before the calls

#### Scenario: XP and VX unchanged
- **WHEN** the tests of the windows of RGSS1 and RGSS2 that existed before this change run
- **THEN** they pass with no expected value edited, except the two pictures that "Skins smaller than the layout" names

### Requirement: Background of a window of RGSS3
The background SHALL be built as that of RGSS2 and drawn in the normal blend at every
openness, after the window's tone has changed its colours: components truncated towards
zero, grey by `l = (77 r + 150 g + 29 b) >> 8` and the normal formula with the grey as
alpha, then the offsets clamped to 0 to 255, the alpha kept. Frame, cursor, contents, arrows
and pause sign SHALL NOT be toned, and the frame SHALL be blended as in RGSS2.

#### Scenario: Normal blend at openness 255, measured on the original
- **WHEN** a window of 68 by 68 with the readable skin and the default back opacity 192 lies over a flat sprite of (200, 200, 200)
- **THEN** the pixel (30, 40) is (70, 77, 124)

#### Scenario: Tone, measured on the original
- **WHEN** the tone (40, -30, 0, 128) is assigned to that window, and then the tone (40.9, -30.9, 0.9, 128.9)
- **THEN** the pixel (30, 40) is (105, 56, 101) both times

#### Scenario: Grey and clamping
- **WHEN** that window has back opacity 255 and the tone (0, 0, 0, 255), and then the tone (-255, 255, 20, 0)
- **THEN** the pixel (30, 40) is (42, 42, 42) and then (0, 255, 120)

#### Scenario: Only the background is toned
- **WHEN** a window of 100 by 100 with the piece skin, back opacity 255, the tone (-255, -255, -255, 0) and `cursor_rect = (20, 20, 16, 16)` is drawn
- **THEN** the pixel (0, 0) has the colour of the frame, the pixel (60, 60) is black, and 256 pixels have the colour of the cursor

#### Scenario: Opening
- **WHEN** a window of 68 by 100 with the piece skin, whose background block is (0, 0, 200), and the tone (40, -30, 0, 128) is drawn at openness 255 and at openness 128
- **THEN** the pixel (30, 50) is (38, 0, 82) both times, and at openness 128 the pixels (30, 23) and (30, 74) are black while (0, 24) and (0, 73) have the colour of the frame

### Requirement: Cursor of a window of RGSS3
The cursor SHALL be composed as in RGSS2 with a border of 4 and the centre (68, 68, 24, 24),
stretched bilinearly with numerator 23 when both inner sides exceed 12 and are not 24 by 24,
else by the nearest rule. Its corner SHALL lie at `pad + |rect.x - ox|`,
`pad + |rect.y - oy|` of the window, and it SHALL be drawn whole from there. Blink and
opacity SHALL be those of RGSS2.

#### Scenario: Pieces, measured on the original
- **WHEN** a window of 100 by 100 with the readable skin has `cursor_rect = (10, 6, 16, 16)`
- **THEN** the pixels (18, 14), (21, 14), (22, 14), (22, 18), (23, 18) and (33, 29) show the skin pixels (64, 64), (67, 64), (68, 64), (68, 68), (71, 68) and (95, 95)

#### Scenario: Bilinear centre
- **WHEN** that window has `cursor_rect = (10, 6, 40, 40)`
- **THEN** the pixel (40, 30) is (80, 76, 100)

#### Scenario: Position by the absolute difference, measured on the original
- **WHEN** a window of 300 by 100 with the piece skin has in turn `cursor_rect = (10, 0, 16, 16)` with `ox = 30`; `cursor_rect = (-250, 0, 16, 16)` with `ox = 0`; `cursor_rect = (10, 20, 16, 16)` with `oy = 50`
- **THEN** the 256 pixels of the cursor's colour fill (28 .. 44, 8 .. 24), then (258 .. 274, 8 .. 24), then (18 .. 34, 38 .. 54)

### Requirement: Cursor placement of a window of RGSS3
The corner of the cursor and its visible part, the cursor's rectangle cut by
(pad, pad, W - pad, H - bottom pad) and by the window, SHALL be computed only at an
assignment of `cursor_rect`, `ox`, `oy`, `padding` or `padding_bottom`, also of an equal
value, and at the assignment of another skin, and SHALL be kept until the next of them.
When drawn, the kept part SHALL be cut by the window as it is then, and SHALL move with it.

#### Scenario: Kept while the window grows, measured on the original
- **WHEN** a window of 40 by 40 with the piece skin gets `cursor_rect = (0, 0, 100, 100)`, and then `width = 120`, `height = 120`, `x = 0`, `y = 3`, `y = 0`, `z = 5`, `opacity = 255`, `contents_opacity = 255`, `openness = 255`, `arrows_visible = true`, a tone and the skin it has
- **THEN** before and after those calls 576 pixels have the cursor's colour, in (8 .. 32, 8 .. 32)

#### Scenario: Move does not place
- **WHEN** the same window gets `move(0, 0, 120, 120)` in place of those calls
- **THEN** 576 pixels have the cursor's colour, in (8 .. 32, 8 .. 32)

#### Scenario: Calls that place, measured on the original
- **WHEN** the window grown to 120 by 120 then gets one of `ox = 0`, `oy = 0`, `padding = 8`, `padding_bottom = 8`, `cursor_rect = (0, 0, 100, 100)`, or `windowskin = nil` followed by the skin again
- **THEN** after each of them 10000 pixels have the cursor's colour, in (8 .. 108, 8 .. 108)

#### Scenario: Cut by the window that shrinks, measured on the original
- **WHEN** a window of 120 by 120 with the piece skin gets `cursor_rect = (0, 0, 100, 100)`, then `width = 40` and `height = 40`, then the same `cursor_rect` again
- **THEN** the cursor's colour fills 10000 pixels, then the 1024 of (8 .. 40, 8 .. 40), then the 576 of (8 .. 32, 8 .. 32)

#### Scenario: Moves with the window
- **WHEN** the window of 120 by 120 with its placed cursor gets `x = 50` and `y = 20`
- **THEN** the cursor's colour fills (58 .. 158, 28 .. 128)

### Requirement: Contents of a window of RGSS3
Contents pixel (u, v) SHALL be drawn at (pad + u - ox, pad + v - oy) inside
(pad, pad, W - pad, H - bottom pad) cut by the window's rectangle, with the alpha and blend
of RGSS2, with or without a skin. Whether contents are shown SHALL be decided at the calls
of RGSS2 and also at every assignment of `padding` and `padding_bottom`.

#### Scenario: Paddings clip, measured on the original
- **WHEN** a window of 60 by 50 with the piece skin and flat opaque contents of 100 by 100 has in turn the default paddings; `padding_bottom = 20`; `padding = 12`; `padding = 30`
- **THEN** the contents fill (8 .. 52, 8 .. 42), 1496 pixels; then (8 .. 52, 8 .. 30), 968; then (12 .. 48, 12 .. 38), 936; then nothing

#### Scenario: Negative padding stays inside the window, measured on the original
- **WHEN** that window gets `padding = -3`
- **THEN** the contents fill the 3000 pixels of the window and no pixel outside it

#### Scenario: Without a skin
- **WHEN** a window of 60 by 50 without a skin has those contents, `ox = 5` and `oy = 7`
- **THEN** the contents fill (8 .. 52, 8 .. 42)

#### Scenario: Paddings show the contents again, measured on the original
- **WHEN** a window of 100 by 100 with opaque contents of 100 by 100 is hidden, given the contents and shown again, and then gets one of `tone`, `arrows_visible = true`, `padding = 8`, `padding_bottom = 8`, `update` or `move(0, 0, 100, 100)`
- **THEN** no pixel of the contents shows before the call nor after the first two, and 7056 show after each of the other four

### Requirement: Scroll arrows of a window of RGSS3
Arrows SHALL stand where RGSS2 has them, halves rounded towards zero: down when
`ch - oy > H - pad - bottom pad`, left when `ox > 0`, right when `cw - ox > W - 2 * pad`, up
when `oy > 0`, none while arrows are not allowed. They SHALL be decided, and only with live
contents, at every assignment of `width`, `height`, `ox`, `oy` and `arrows_visible`, at
`move`, and when another skin or contents object is assigned; at no other call.

#### Scenario: Places, measured on the original
- **WHEN** a window of 100 by 100 with the piece skin and contents of 200 by 200 at contents opacity 0 gets `ox = 10`, and then `oy = 5`
- **THEN** first the down arrow fills (42 .. 58, 88 .. 96), the left (4 .. 12, 42 .. 58), the right (88 .. 96, 42 .. 58) and no up arrow shows; then the up arrow fills (42 .. 58, 4 .. 12) as well

#### Scenario: The paddings count but do not decide, measured on the original
- **WHEN** such a window with contents of 80 by 80 gets `padding = 16`, then `update`, `x = 5`, `active = false`, `pause = true`, a `cursor_rect`, `contents_opacity = 0` and `openness = 255`, and then `ox = 0`
- **THEN** no arrow shows until `ox = 0`, after which the down and the right arrow show and no other

#### Scenario: Other calls that decide
- **WHEN** in place of `ox = 0` the window gets `width = 100`, or `move(0, 0, 100, 100)`
- **THEN** the down and the right arrow show after either

#### Scenario: Bottom padding
- **WHEN** such a window with contents of 80 by 80 and the default padding gets `padding_bottom = 40`, and then `height = 100`
- **THEN** no arrow shows after the first call and the down arrow alone after the second

#### Scenario: Arrows allowed, measured on the original
- **WHEN** the window that shows the down and the right arrow gets `arrows_visible = false`, and then `arrows_visible = true`
- **THEN** no arrow shows after the first call and both show after the second

#### Scenario: Without live contents the list stays
- **WHEN** a window of 100 by 100 with contents of 200 by 200 and `ox = 10` has its contents bitmap disposed, or set to nil, and then gets `ox = 0`
- **THEN** the down, left and right arrows are drawn whole, 128 pixels each, the left one although ox is 0

### Requirement: Order of the parts of a window of RGSS3
An open window SHALL draw, in this order: background, frame, cursor, arrows (down, left,
right, up), pause sign, contents. The pause sign SHALL be that of RGSS2 at
(W / 2 - 8, H - 16) with the division rounding towards zero, cut by the window. Cursor,
arrows, pause sign and contents SHALL be drawn only at openness 255.

#### Scenario: Contents over arrows, measured on the original
- **WHEN** a window of 100 by 100 with the piece skin and flat opaque contents of 200 by 200 gets `ox = 10`, and then `padding = 0`
- **THEN** first the contents fill 7056 pixels and the down, left and right arrows show 64 pixels each, in (42 .. 58, 92 .. 96), (4 .. 8, 42 .. 58) and (92 .. 96, 42 .. 58); then the contents fill 10000 pixels and no pixel of an arrow shows

#### Scenario: Contents over the pause sign, measured on the original
- **WHEN** a window of 100 by 100 with the piece skin gets `pause = true` and 8 updates, once with the transparent contents and once with flat opaque contents of 200 by 200 and `padding = 0`
- **THEN** the first image of the pause sign fills (42 .. 58, 84 .. 100) the first time and shows no pixel the second

#### Scenario: Contents over the cursor
- **WHEN** a window of 100 by 100 with the piece skin, flat opaque contents of 100 by 100 and `cursor_rect = (0, 0, 16, 16)` is drawn
- **THEN** no pixel has the cursor's colour

### Requirement: Window of RGSS3 without area
A window of RGSS3 whose width or height is 0 or less SHALL draw no background and no frame.
Its cursor, arrows, pause sign and contents SHALL be cut to the one pixel at the window's
origin in place of the window's rectangle, when they are drawn and when the cursor is
placed, and SHALL be drawn there by their usual place, alpha and order.

#### Scenario: One pixel of the contents, measured on the original
- **WHEN** a window of 0 by 50 at (100, 100) without a skin has flat opaque contents of 100 by 100 and `padding = -3`
- **THEN** the pixel (100, 100) has the colour of the contents and no other pixel of the frame is drawn

#### Scenario: Nothing with a padding that leaves the pixel out
- **WHEN** that window keeps the default padding
- **THEN** nothing is drawn

#### Scenario: No background and no frame
- **WHEN** a window of 50 by 0 at (100, 100) with the piece skin has those contents and `padding = -3`
- **THEN** the pixel (100, 100) has the colour of the contents and no other pixel is drawn

#### Scenario: An arrow over the pixel, measured on the original
- **WHEN** a window of 5 by -3 at (100, 100) with the readable skin and contents of 200 by 200 at contents opacity 0 gets `width = 5`
- **THEN** the pixel (100, 100) is (111, 33, 100), which is the right arrow at (-7, -9), and no other pixel is drawn

#### Scenario: The pause sign over the pixel
- **WHEN** a window of -3 by 10 at (100, 100) with the readable skin gets `pause = true` and 8 updates
- **THEN** the pixel (100, 100) is (105, 70, 100), which is the pause sign at (-9, -6), and no other pixel is drawn

### Requirement: Cached parts and move of a window of RGSS3
Background, frame and cursor of a window of RGSS3 SHALL be built at the calls at which RGSS2
builds them. `move(x, y, width, height)` SHALL be the assignments of `x`, `y`, `width` and
`height` in this order. Assigning `tone`, `padding`, `padding_bottom` or `arrows_visible`
SHALL build nothing.

#### Scenario: Move builds, measured on the original
- **WHEN** the pixels of the skin are changed after `windowskin=` and the window gets `move` with the position and size it has
- **THEN** background and frame are drawn from the new pixels

#### Scenario: The new properties build nothing, measured on the original
- **WHEN** the pixels of the skin are changed after `windowskin=` and the window gets `tone`, `padding`, `padding_bottom` and `arrows_visible`
- **THEN** background and frame are drawn from the old pixels

#### Scenario: Move reads back
- **WHEN** a window gets `move(5, 6, 100, 80)`
- **THEN** its x, y, width and height are 5, 6, 100 and 80

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
- **WHEN** the scenes `g6-skin_sizes_t` and `g8-smallskin_t_0` of `crates/vkxp-core/tests/data/window-rgss2-cases.txt` are composed
- **THEN** the digests of their frames are `edbddf94f4992ce6` and `0263b5710a3a9da6`, which are those of the pictures of the original runtime of VX, and the file no longer names them as not measured

#### Scenario: Nothing else of RGSS2 changes
- **WHEN** `window-rgss2-cases.txt` and `window-rgss2-random.txt` are compared with what they were before this change
- **THEN** every scene and every other digest is as it was: 145 of the 147 frames keep their digest

#### Scenario: Cursor of RGSS3, measured on the original
- **WHEN** a window of RGSS3 of 100 by 100 has the left 80 columns of the piece skin as its skin and `cursor_rect` (0, 0, 32, 32), then (0, 0, 16, 16), then (0, 0, 40, 40)
- **THEN** the cursor's colour fills (8 .. 24, 8 .. 40), 512 pixels; then (8 .. 15, 8 .. 24), 112; then (8 .. 27, 8 .. 48), 760

### Requirement: Disposed skin and contents of a window of RGSS3
A window of RGSS3 whose skin bitmap is disposed SHALL draw nothing that comes from the skin,
the background, frame and cursor it built included, whatever is assigned later short of
another skin. A disposed contents bitmap SHALL NOT be drawn and SHALL count as no contents
when the arrows are decided. No call on the window SHALL fail for either.

#### Scenario: Skin disposed, measured on the original
- **WHEN** a window of 100 by 100 with the piece skin, flat opaque contents of 100 by 100 and `cursor_rect = (0, 0, 16, 16)` has its skin disposed, and is drawn at once and again after `width = 100` and `update`
- **THEN** both frames have exactly 7056 pixels that are not black, all of the colour of the contents

#### Scenario: Contents disposed, measured on the original
- **WHEN** a window of 100 by 100 with the piece skin, flat opaque contents of 200 by 200 and `ox = 10` has its contents disposed
- **THEN** no pixel of the contents shows, and the down, left and right arrows show whole, 128 pixels each

### Requirement: Calls at which the original of RGSS3 faults
Where the original runtime of RGSS3 ends with a page fault, the scene SHALL go on: a call
that places the cursor in a window on which no skin was ever assigned SHALL take effect by
the rules above, and a window or a cursor too large for a bitmap SHALL be drawn without the
parts that would have to be built.

#### Scenario: Cursor placed without a skin ever assigned
- **WHEN** a new window of RGSS3 of 100 by 100, on which no skin was assigned, gets `ox = 0`, `oy = 0`, `padding = 8`, `padding_bottom = 8` and `cursor_rect = (0, 0, 0, 0)`, and a frame is extracted
- **THEN** every call returns and the frame is black

#### Scenario: Sizes at which the original faults
- **WHEN** a window of RGSS3 with a skin gets the size 20000 by 20000, then 100000 by 100000, then a `cursor_rect` of 100000 by 100000, a frame being extracted after each
- **THEN** every call returns and every frame is extracted

### Requirement: Whole windows of RGSS3 as measured
The CPU reference and the Vulkan backend SHALL draw scenes of windows of RGSS3 as the model
of the measurement of 2026-10-13 does, and SHALL agree with each other byte for byte. The
scenes SHALL be committed as text with every bitmap made by a formula, and their pictures as
digests.

#### Scenario: Cases of the measurement
- **WHEN** the committed scenes of the case files of the measurement, at least one of each of a1 to a9 and a11 to a16 that has a scene the fixture can hold, are extracted and composed by the CPU reference
- **THEN** every frame has the digest of the picture the model draws of its scene

#### Scenario: Random scenes held out
- **WHEN** thirty committed scenes of the measurement's random generator with seed 3 are extracted and composed by the CPU reference
- **THEN** every frame has the digest of the picture the model draws of its scene

#### Scenario: Both backends
- **WHEN** the packets of every frame of both files are composed by the CPU reference and by the Vulkan backend
- **THEN** the two frames are equal byte for byte and have the committed digest

#### Scenario: Nothing of the original
- **WHEN** the two files are read
- **THEN** every bitmap in them is made by a formula, and their head names the model and the date of the measurement and every case file that has no scene in them, with the reason
