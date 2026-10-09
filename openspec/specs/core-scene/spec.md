# core-scene Specification

## Purpose
Defines the scene that scripts build out of sprites, viewports and windows, the order in which
its elements are drawn, and the extraction of a protocol frame packet from it. Rendering such a
packet with a compositor is how every requirement here is observed.

## Requirements

### Requirement: Scene elements
The scene SHALL hold sprites, viewports, windows, planes and tilemaps, each created with or
without a viewport, with the properties of its RGSS1 class and their RGSS1 defaults. After an
element is disposed, every operation on it other than disposing it again and asking whether it
is disposed SHALL fail with a disposed error, and it SHALL no longer be drawn.

#### Scenario: Defaults of a sprite
- **WHEN** a sprite is created
- **THEN** it is visible, at position 0, 0 with z 0, origin 0, 0, zoom 1 by 1, angle 0, not mirrored, opacity 255, normal blending, a colour of alpha 0 and a neutral tone, and has no bitmap

#### Scenario: Use after disposal
- **WHEN** a property of a disposed sprite is set
- **THEN** the operation fails with a disposed error

#### Scenario: Disposed elements are not drawn
- **WHEN** a sprite that was visible in one frame is disposed before the next
- **THEN** the next frame does not show it

### Requirement: Draw order
Elements SHALL be drawn in ascending order of z, so that an element with a larger z appears in
front. Among elements with equal z, the one that was created, or last given a different z, more
recently SHALL appear in front.

#### Scenario: Larger z in front
- **WHEN** two overlapping opaque sprites have z 1 and z 2
- **THEN** the overlap shows the sprite with z 2, whichever was created first

#### Scenario: Equal z
- **WHEN** two overlapping opaque sprites have the same z
- **THEN** the overlap shows the one created later

#### Scenario: Moving to an occupied z
- **WHEN** the earlier-created of two sprites is moved to another z and then back to the shared z
- **THEN** the overlap shows that sprite

#### Scenario: Setting the same z again
- **WHEN** the earlier-created of two sprites with equal z has its z set to the value it already has
- **THEN** the overlap still shows the later-created sprite

### Requirement: Sprite placement
A visible sprite with a bitmap SHALL draw its source rectangle so that the point of that
rectangle given by the sprite's origin lands on the sprite's position, one bitmap pixel per
screen pixel at zoom 1 and angle 0. Assigning a bitmap SHALL set the source rectangle to the
whole bitmap. The part of the source rectangle outside the bitmap SHALL draw nothing and SHALL
NOT shift the rest. A sprite without a bitmap, or whose bitmap is disposed, SHALL draw nothing.

#### Scenario: Position and origin
- **WHEN** a sprite with a 4 by 4 bitmap is at position 10, 20 with origin 1, 1
- **THEN** bitmap pixel (1, 1) is at screen pixel (10, 20) and bitmap pixel (0, 0) at (9, 19)

#### Scenario: Source rectangle
- **WHEN** the source rectangle is set to x 2, y 0, width 2, height 4 and the origin is 0, 0
- **THEN** bitmap pixel (2, 0) is at the sprite's position and columns 0 and 1 of the bitmap are not drawn

#### Scenario: Source rectangle beyond the bitmap
- **WHEN** the source rectangle of a 4 by 4 bitmap is x -2, y 0, width 4, height 4
- **THEN** bitmap pixel (0, 0) is drawn 2 pixels right of the sprite's position and nothing is drawn left of it

#### Scenario: Disposed bitmap
- **WHEN** the bitmap of a visible sprite is disposed
- **THEN** the sprite draws nothing and extraction succeeds

### Requirement: Sprite zoom, rotation and mirror
Zoom SHALL scale the sprite about its origin, independently in x and y. The angle SHALL rotate
it about its origin, counter-clockwise on screen, in degrees. Mirroring SHALL flip the source
rectangle left to right in place. Angles that are multiples of 90 degrees and whole-number zooms
SHALL map bitmap pixels to whole screen pixels exactly, and a zoom of 0 SHALL draw nothing.

#### Scenario: Zoom
- **WHEN** a sprite with a 2 by 1 bitmap holding pixels A and B is at 10, 10 with zoom 2 in x
- **THEN** screen pixels (10, 10) and (11, 10) are A and (12, 10) and (13, 10) are B

#### Scenario: Quarter turn
- **WHEN** the same sprite has zoom 1 and angle 90
- **THEN** A is at screen pixel (10, 9) and B at (10, 8)

#### Scenario: Half turn
- **WHEN** the same sprite has angle 180
- **THEN** A is at screen pixel (9, 9) and B at (8, 9)

#### Scenario: Mirror
- **WHEN** the same sprite has angle 0 and is mirrored
- **THEN** screen pixel (10, 10) is B and (11, 10) is A

### Requirement: Sprite appearance
A sprite's opacity SHALL be clamped to 0 to 255 and scale its alpha. Blend type 1 SHALL add, 2
SHALL subtract and any other value SHALL blend normally. Its tone and then its colour SHALL
recolour its own pixels only, as the quad op defines. An invisible sprite SHALL draw nothing.

#### Scenario: Opacity is clamped
- **WHEN** a sprite's opacity is set to 300 and then to -5
- **THEN** it reads back 255 and then 0

#### Scenario: Blend types
- **WHEN** an opaque white sprite with blend type 2 is drawn over a mid-grey sprite
- **THEN** the overlap is black

#### Scenario: Colour does not leak
- **WHEN** a sprite with a full-alpha red colour is drawn over a blue background sprite
- **THEN** the sprite's opaque pixels are red and the background outside the sprite stays blue

### Requirement: Sprite flash
A flash with a colour and a duration SHALL, for that many updates of the sprite, show the
flash colour with its alpha falling linearly from the given alpha to 0, in place of the
sprite's own colour whenever the flash's current alpha is the larger. A flash without a colour
SHALL hide the sprite for the duration. Flashes SHALL advance only when the sprite is updated.

#### Scenario: Linear decay
- **WHEN** a sprite is flashed with a colour of alpha 200 for 4 updates
- **THEN** the flash alpha is 200 before the first update, then 150, 100 and 50, and the flash is over after the fourth

#### Scenario: Flash without colour
- **WHEN** a sprite is flashed without a colour for 2 updates
- **THEN** it draws nothing until it has been updated twice

#### Scenario: No update, no progress
- **WHEN** a flashed sprite is drawn in three frames without being updated
- **THEN** the flash looks the same in all three

### Requirement: Viewport
The elements of a viewport SHALL be drawn together at the viewport's place in the draw order,
ordered among themselves by their own z, shifted by the viewport's rectangle position minus its
origin, and clipped to its rectangle. An invisible or disposed viewport SHALL hide its elements.

#### Scenario: Offset and clip
- **WHEN** a viewport has the rectangle 100, 50, 200, 100 and holds a sprite at 0, 0 with a 300 by 10 bitmap
- **THEN** the sprite's first pixel is at screen pixel (100, 50) and nothing is drawn at x 300 or beyond

#### Scenario: Viewport origin scrolls
- **WHEN** that viewport's origin x is set to 10
- **THEN** bitmap column 10 is at screen x 100

#### Scenario: Viewport z outranks element z
- **WHEN** a viewport with z 100 holds a sprite with z 0, and a sprite outside any viewport has z 50
- **THEN** where they overlap the viewport's sprite is in front

#### Scenario: Invisible viewport
- **WHEN** a viewport is made invisible
- **THEN** none of its elements is drawn

### Requirement: Viewport effects
A viewport's tone, colour and flash SHALL be applied, after its elements are drawn, to every
pixel inside its rectangle, including what elements behind the viewport drew there. The flash
SHALL follow the rules of the sprite flash, advancing when the viewport is updated. A neutral
tone, a colour of alpha 0 and no flash SHALL change nothing.

#### Scenario: Tone reaches what is behind
- **WHEN** a viewport with a full-grey tone covers part of a coloured sprite that is behind it
- **THEN** the covered part is grey and the rest of that sprite keeps its colours

#### Scenario: Elements in front are unaffected
- **WHEN** a sprite outside the viewport has a larger z than the viewport
- **THEN** the viewport's tone does not change that sprite

### Requirement: Window frame and background
A visible window with a window skin SHALL draw, at its z, a background taken from the skin's
128 by 128 pattern inside a margin of 2 pixels, stretched to that area or tiled over it as its
stretch flag says, and over it a frame built from the skin's corner and edge pieces. The frame's
alpha SHALL be scaled by the window's opacity and the background's by opacity times back opacity
over 255. A window without a skin SHALL draw neither.

#### Scenario: Background area
- **WHEN** a 160 by 96 window at 0, 0 with a single-coloured background pattern is drawn
- **THEN** screen pixels from (2, 2) to (157, 93) not covered by the frame have that colour

#### Scenario: Corners
- **WHEN** the skin's four frame corners have four different colours
- **THEN** each corner of the window shows the colour of the matching skin corner

#### Scenario: Back opacity
- **WHEN** the back opacity is 0 and the opacity is 255
- **THEN** the frame is drawn and the background is not

#### Scenario: Opacity
- **WHEN** the opacity is 0
- **THEN** neither frame nor background is drawn, and the contents still are

### Requirement: Window contents
A window's contents bitmap SHALL be drawn 16 pixels in from the window's top-left corner, shifted
by the window's origin, clipped to the area 16 pixels inside each edge, with its alpha scaled by
the contents opacity, at the window's z plus 2.

#### Scenario: Clipping
- **WHEN** a 160 by 96 window has a 300 by 300 contents bitmap
- **THEN** only its top-left 128 by 64 pixels are drawn, from screen pixel (16, 16) on

#### Scenario: Scrolling
- **WHEN** the window's origin y is 32
- **THEN** row 32 of the contents is at screen row 16 of the window

#### Scenario: Between frame and contents
- **WHEN** a sprite has the window's z plus 1 and overlaps the window
- **THEN** it is in front of the window's background and frame and behind its contents

### Requirement: Window cursor
A window with a non-empty cursor rectangle SHALL draw the skin's cursor image scaled to that
rectangle, positioned relative to the contents area, behind the contents and in front of the
background. While the window is active the cursor's alpha SHALL cycle with each update of the
window over 32 steps, falling from 255 by 8 per step for 16 steps and then rising by 8 per
step; while inactive it SHALL keep its current value.

#### Scenario: Position and size
- **WHEN** the cursor rectangle is 0, 32, 128, 32 in a window at 0, 0
- **THEN** the cursor covers screen pixels (16, 48) to (143, 79)

#### Scenario: Blink
- **WHEN** an active window is updated 16 times and then 16 more times
- **THEN** the cursor alpha is 127 after the first 16 updates and 255 again after 32

#### Scenario: Inactive window
- **WHEN** an inactive window is updated
- **THEN** the cursor alpha does not change

#### Scenario: Empty rectangle
- **WHEN** the cursor rectangle has width 0
- **THEN** no cursor is drawn

### Requirement: Window pause sign and scroll arrows
A window whose pause flag is set SHALL draw the skin's pause sign centred at its bottom edge,
stepping through the sign's four images as the window is updated. A window SHALL draw the
skin's arrow for each direction in which its contents extend beyond the visible area. Both
SHALL be drawn at the window's z plus 2.

#### Scenario: Pause sign
- **WHEN** the pause flag of a 160 by 96 window is set
- **THEN** a 16 by 16 image from the skin's pause area is drawn with its top-left corner at window pixel (72, 80)

#### Scenario: Arrow for hidden rows
- **WHEN** the contents are taller than the visible area and the window's origin y is 0
- **THEN** the down arrow is drawn and the up arrow is not

### Requirement: Frame bookkeeping
The frame count SHALL start at 0, increase by 1 with every extracted frame and be settable to
any value. The frame rate SHALL default to 40, be clamped to 10 to 120 when set, and be carried
by every extracted packet. Extraction SHALL use the scene's logical size, 640 by 480 unless the
scene was created with another.

#### Scenario: Counting
- **WHEN** three frames are extracted and the frame count is then set to 100 and one more frame is extracted
- **THEN** the frame count reads 3 before it is set and 101 at the end

#### Scenario: Frame rate limits
- **WHEN** the frame rate is set to 5 and then to 500
- **THEN** it reads back 10 and then 120, and the next packet carries 120

### Requirement: Extracted packets
Extraction SHALL return a frame packet whose bitmap deltas come from the bitmap arena for
exactly the bitmaps the packet's ops refer to, followed by the ops in draw order. Every
extracted packet SHALL be accepted by the reference compositor when the packets of one scene
are given to it in the order they were extracted.

#### Scenario: Unused bitmaps are not sent
- **WHEN** a bitmap exists that no visible element uses
- **THEN** the extracted packet has no delta for it

#### Scenario: Random scenes
- **WHEN** random scenes are built and changed with random operations on elements and bitmaps, extracting a frame after each step
- **THEN** the reference compositor renders every packet without error

### Requirement: Plane
A visible plane with a bitmap SHALL repeat that bitmap without end in both directions over its
viewport's rectangle, or the whole screen without a viewport, so that bitmap pixel (ox, oy) is
at the top-left corner of that area. Its zoom SHALL scale the repeated image about that corner.
Its opacity, blend type, colour and tone SHALL act as a sprite's do. Repeats SHALL meet without
gaps or overlaps.

#### Scenario: Tiling
- **WHEN** a plane with a 64 by 64 bitmap covers a 640 by 480 screen
- **THEN** screen pixel (x, y) shows bitmap pixel (x mod 64, y mod 64) for every x and y

#### Scenario: Scrolling
- **WHEN** the plane's origin is set to 10, 20
- **THEN** screen pixel (0, 0) shows bitmap pixel (10, 20)

#### Scenario: Negative origin
- **WHEN** the plane's origin is -1, -1
- **THEN** screen pixel (0, 0) shows bitmap pixel (63, 63)

#### Scenario: Zoom
- **WHEN** the plane's zoom is 2 in both directions and its origin is 0, 0
- **THEN** screen pixels (0, 0) to (1, 1) show bitmap pixel (0, 0) and the image repeats every 128 pixels

#### Scenario: Seams with half-transparent pixels
- **WHEN** a plane with a uniformly half-transparent bitmap is drawn over a flat background at zoom 1.5
- **THEN** every covered screen pixel has the same colour

#### Scenario: Inside a viewport
- **WHEN** the plane belongs to a viewport with the rectangle 100, 50, 200, 100
- **THEN** it is drawn only inside that rectangle, with bitmap pixel (ox, oy) at screen pixel (100, 50)

### Requirement: Tile identifiers
A tile identifier of 384 or more SHALL name the 32 by 32 tile of the tileset bitmap at column
`(id - 384) mod 8` and row `(id - 384) div 8`. An identifier from 48 to 383 SHALL name pattern
`id mod 48` of autotile `id div 48 - 1`. An identifier below 48, a tile outside its bitmap, and
a tile whose tileset or autotile bitmap is missing or disposed SHALL draw nothing.

#### Scenario: Tileset tile
- **WHEN** a map cell holds 384 + 8 + 3
- **THEN** it shows the tileset pixels from (96, 32) to (127, 63)

#### Scenario: Empty cell
- **WHEN** a map cell holds 0
- **THEN** nothing is drawn for that cell in that layer

#### Scenario: Missing tileset
- **WHEN** a tilemap has map data and no tileset bitmap
- **THEN** its tileset tiles draw nothing and extraction succeeds

### Requirement: Autotile patterns
An autotile tile SHALL be assembled from four 16 by 16 quarters of its 96-pixel-wide,
128-pixel-high sheet, chosen by the pattern number through the 48-entry table of the original
runtime, in the order top-left, top-right, bottom-left, bottom-right. A sheet that is 32 pixels
high SHALL instead supply the whole 32 by 32 tile for every pattern.

#### Scenario: Enclosed pattern
- **WHEN** a cell holds pattern 0 of an autotile
- **THEN** its four quarters come from sheet positions (32, 64), (48, 64), (32, 80) and (48, 80)

#### Scenario: Isolated pattern
- **WHEN** a cell holds pattern 47 of an autotile
- **THEN** its four quarters come from sheet positions (0, 0), (16, 0), (0, 16) and (16, 16)

#### Scenario: Every pattern
- **WHEN** each of the 48 patterns is drawn from a sheet whose 48 quarters are all different
- **THEN** each quarter of each drawn tile equals the quarter the table names

#### Scenario: Single-tile sheet
- **WHEN** an autotile sheet is 32 by 32 pixels
- **THEN** every pattern of that autotile shows the whole sheet

### Requirement: Autotile animation
An autotile sheet wider than one frame SHALL be animated: with frames 96 pixels wide (32 for a
32-pixel-high sheet), the frame shown SHALL advance by one, cyclically, with every 16 updates of
the tilemap. Without updates the frame SHALL NOT change.

#### Scenario: Four frames
- **WHEN** a tilemap with a 384-pixel-wide autotile sheet is updated 16, 32, 48 and 64 times
- **THEN** it shows frames 1, 2, 3 and then 0 again

#### Scenario: No update
- **WHEN** frames are extracted repeatedly without updating the tilemap
- **THEN** the same animation frame is shown each time

### Requirement: Tilemap layout
A visible tilemap with map data SHALL draw each cell (x, y) of each of its three layers as a
32 by 32 tile at `x * 32 - ox`, `y * 32 - oy` within its viewport's rectangle, or the screen
without a viewport, the layers in ascending order where they share a draw position in the
order. The map SHALL repeat in both directions beyond its edges.

#### Scenario: Position
- **WHEN** a 20 by 15 map is drawn with origin 0, 0
- **THEN** the tile of cell (3, 2) covers screen pixels (96, 64) to (127, 95)

#### Scenario: Scrolling by pixels
- **WHEN** the origin is 40, 0
- **THEN** the tile of cell (1, 0) starts 8 pixels left of the screen and cell (2, 0) starts at x 24

#### Scenario: Upper layer over lower
- **WHEN** layers 0 and 2 both have an opaque tile in the same cell with the same priority
- **THEN** the cell shows the tile of layer 2

#### Scenario: Wrap-around
- **WHEN** a 20 by 15 map is drawn with origin 320, 0
- **THEN** the right half of the screen shows the map's columns 0 to 9

### Requirement: Tile priorities
A tile's priority SHALL be the priorities table's value for its identifier, 0 when the table
has no such entry. Tiles of priority 0 SHALL be drawn at z 0 among the elements of the
tilemap's viewport. A tile of priority p from 1 to 5 SHALL be drawn at the z given by its top
edge's y position in the viewport plus `(p + 1) * 32`, so that it ranks among sprites by row.

#### Scenario: Ground under sprites
- **WHEN** a sprite with z 1 overlaps tiles of priority 0
- **THEN** the sprite is in front

#### Scenario: Same row
- **WHEN** a tile of priority 1 has its top edge at y 96 and a sprite with z 159 overlaps it
- **THEN** the tile is in front

#### Scenario: Row below
- **WHEN** that sprite's z is 192
- **THEN** the sprite is in front of the tile

#### Scenario: Higher priority
- **WHEN** a tile of priority 2 with its top edge at y 96 overlaps a sprite with z 191
- **THEN** the tile is in front

### Requirement: Shared tables
The tilemap SHALL show the contents its map data and priorities tables have at each extraction,
including changes made to a table after it was assigned to the tilemap.

#### Scenario: Cell changed after assignment
- **WHEN** a map cell is changed through the table that was earlier assigned as map data
- **THEN** the next extracted frame shows the new tile, without the table being assigned again

### Requirement: Sprite bush depth
A sprite with a bush depth greater than zero SHALL draw the lowest rows of its source
rectangle, as many as the bush depth, with half of the opacity of the rows above, and the rest
unchanged. A bush depth of zero or less SHALL change nothing, and one that exceeds the source
height SHALL apply to the whole sprite.

#### Scenario: Lower part is fainter
- **WHEN** an opaque white 32 by 32 sprite with bush depth 8 is drawn over black
- **THEN** its upper 24 rows are white and its lower 8 rows are mid grey

#### Scenario: With zoom
- **WHEN** that sprite is drawn with a vertical zoom of 2
- **THEN** the lower 16 screen rows are the fainter ones

#### Scenario: Zero
- **WHEN** the bush depth is 0
- **THEN** the frame equals that of a sprite without the property

### Requirement: Tilemap flash data
For every map cell whose entry in the flash data table is non-zero, a tilemap SHALL draw a
32 by 32 overlay above the cell's ground tiles in the colour the entry encodes, four bits each
for red, green and blue from the high to the low end of its lowest twelve bits, with a strength
that rises and falls as the tilemap is updated. Cells with a zero entry and a tilemap without
flash data SHALL be drawn as before.

#### Scenario: Flashing cell
- **WHEN** the flash data holds `0xF00` for one cell and the tilemap is rendered at the strongest point of the pulse
- **THEN** that cell is redder than without the entry and its neighbours are unchanged

#### Scenario: Pulse
- **WHEN** the tilemap is updated repeatedly
- **THEN** the strength of the overlay differs between at least two of the frames and repeats

#### Scenario: Scrolled map
- **WHEN** the tilemap's origin is moved by 16 pixels
- **THEN** the overlay moves with its cell

### Requirement: Unrotated sprites are stretched as in the original
A sprite whose angle is a multiple of 360 SHALL be extracted as a stretch quad with
`dw = floor(w * zoom_x)`, `dh = floor(h * zoom_y)`, `x0 = floor(x - ox * zoom_x)` and
`y0 = floor(y - oy * zoom_y)`, computed in 32-bit floating point with the zoom values rounded
to that precision first, `w` and `h` being the size of its source rectangle, and with the
mirror flag for a mirrored sprite. A zoom of 0 or less SHALL draw nothing.

#### Scenario: Measured frames
- **WHEN** each committed case of zoom, origin, mirror and source rectangle measured on the original runtime is extracted and rendered
- **THEN** the listed pixels equal the measured ones

### Requirement: Rotated sprites sample at integer offsets
A sprite with another angle SHALL be sampled so that the screen pixel at integer offset
`(dx, dy)` from the sprite's position shows the texel at the floor of
`U = ox + (dx * cos a - dy * sin a) / zoom_x`, `V = oy + (dx * sin a + dy * cos a) / zoom_y`,
with `U` replaced by `(w - 1) - U` when mirrored, and no half-pixel offset.

#### Scenario: Measured samples away from texel boundaries
- **WHEN** the committed rotation samples that the measurement marks as not on a texel boundary are rendered
- **THEN** they show the measured texel

### Requirement: Planes tile a stretched tile
A plane SHALL repeat a tile of width `tw = floor(w * zoom_x)` and height
`th = floor(h * zoom_y)`, stretched from its bitmap by the stretch rule, so that column `i` of
its viewport, or of the screen without one, shows tile column `(i + plane.ox + viewport.ox)`
modulo `tw` with the mathematical modulo, and rows likewise; the offsets are in screen pixels
and are not scaled by the zoom.

#### Scenario: Measured samples
- **WHEN** each committed plane sample measured on the original runtime is extracted and rendered
- **THEN** the listed pixels equal the measured ones

### Requirement: Bush depth and flash details of the original
The bush depth SHALL select the lowest screen rows of an unrotated sprite, as many as the bush
depth and not scaled by the vertical zoom, and draw them with the opacity shifted right by one.
A flash's alpha SHALL become `alpha * (r - 1) / r` in integer division at each update with `r`
frames remaining; a flash of duration 0 SHALL show until the first update; and the flash colour
SHALL replace the sprite's colour when its alpha is greater than or equal to the colour's.

#### Scenario: Measured samples
- **WHEN** each committed flash and bush sample measured on the original runtime is extracted and rendered
- **THEN** the listed pixels equal the measured ones

#### Scenario: Decay
- **WHEN** a flash with alpha 128 and duration 5 is updated five times
- **THEN** its alpha is 128, 102, 76, 50, 25 and then 0
