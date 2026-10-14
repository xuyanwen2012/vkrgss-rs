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
front. Among elements with equal z, the one that was created more recently SHALL appear in
front; assigning a z does not change the order among equals. A viewport takes its place among
the top-level elements by its own z and its own creation.

#### Scenario: Larger z in front
- **WHEN** two overlapping opaque sprites have z 1 and z 2
- **THEN** the overlap shows the sprite with z 2, whichever was created first

#### Scenario: Equal z
- **WHEN** two overlapping opaque sprites have the same z
- **THEN** the overlap shows the one created later

#### Scenario: Moving to an occupied z
- **WHEN** the earlier-created of two sprites is moved to another z and then back to the shared z
- **THEN** the overlap still shows the later-created sprite

#### Scenario: Setting the same z again
- **WHEN** the earlier-created of two sprites with equal z has its z set to the value it already has
- **THEN** the overlap still shows the later-created sprite

#### Scenario: Measured orders
- **WHEN** each row of the unit-test table of the design (sprites, a plane, windows, viewports and a tilemap, with z taken away and back) is built and extracted
- **THEN** the elements are drawn in the order the table gives for the original

#### Scenario: Viewport older than a top-level sprite
- **WHEN** a viewport is created, then a top-level sprite, then a sprite in the viewport, all with z 0
- **THEN** the top-level sprite is drawn over the sprite in the viewport

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
rectangle left to right in place. A zoom of 0 SHALL draw nothing. A sprite whose angle is
exactly 0 SHALL be stretched as the unrotated rule says; every other angle, multiples of 360
included, SHALL follow the requirement "Rotated sprites as in the original".

#### Scenario: Zoom
- **WHEN** a sprite with a 2 by 1 bitmap holding pixels A and B is at 10, 10 with zoom 2 in x
- **THEN** screen pixels (10, 10) and (11, 10) are A and (12, 10) and (13, 10) are B

#### Scenario: Quarter turn
- **WHEN** a sprite with an opaque 200 by 150 bitmap has angle 90
- **THEN** its pixels equal the committed frame measured on the original runtime for that case

#### Scenario: Half turn
- **WHEN** a sprite with a 200 by 150 bitmap, origin (100, 75), at (320, 240) has angle 180
- **THEN** screen pixel (320, 240) shows texel (100, 74) and the sprite covers 29651 pixels, as measured

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
tilemap's viewport. A tile of priority p from 1 to 5 in map row R SHALL be drawn at
`z = 32 * R - oy + 32 + 32 * p` with `oy` the tilemap's origin; the viewport's origin and
rectangle do not enter. A tile whose priority is not 0 to 5 SHALL not be drawn.

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

#### Scenario: Viewport origin
- **WHEN** a tile of priority 1 in map row 1 is drawn with the tilemap's oy 0 in a viewport whose oy is 10
- **THEN** its z is 96

#### Scenario: Measured rows
- **WHEN** the rows (R, p, oy) = (1, 1, 7), (2, 2, 32), (3, 3, 109), (-1, 5, -20) and (11, 5, 300) are drawn
- **THEN** their z is 89, 128, 115, 180 and 244

#### Scenario: Priority out of range
- **WHEN** the priorities table holds 6, 9, -1 or 100 for a tile on the map
- **THEN** the tile is not drawn, neither as ground nor in a row

### Requirement: Shared tables
The tilemap SHALL show the contents its map data and priorities tables have at each extraction,
including changes made to a table after it was assigned to the tilemap.

#### Scenario: Cell changed after assignment
- **WHEN** a map cell is changed through the table that was earlier assigned as map data
- **THEN** the next extracted frame shows the new tile, without the table being assigned again

### Requirement: Sprite bush depth
A sprite with a bush depth greater than zero SHALL draw the lowest screen rows of the box it
covers, as many as the bush depth and not scaled by the vertical zoom, with half of the
opacity of the rows above, and the rest unchanged. A bush depth of zero or less SHALL change
nothing, and one that exceeds the height of the box SHALL apply to the whole sprite.

#### Scenario: Lower part is fainter
- **WHEN** an opaque white 32 by 32 sprite with bush depth 8 is drawn over black
- **THEN** its upper 24 rows are white and its lower 8 rows are mid grey

#### Scenario: With zoom
- **WHEN** that sprite is drawn with a vertical zoom of 2
- **THEN** the lower 8 screen rows are the fainter ones

#### Scenario: Zero
- **WHEN** the bush depth is 0
- **THEN** the frame equals that of a sprite without the property

### Requirement: Tilemap flash data
For every visible cell whose entry in the flash data table is 0 or more and has a non-zero
nibble among its lowest twelve bits, a tilemap SHALL add to what lies below the ground layer's
top, per channel and saturating at 255, the nibble (red, green, blue from the high to the low
end of those bits) times `m = 4 + [0, 1, 2, 3, 4, 3, 2, 1][(u / 4) % 8]`, with `u` the number
of updates of that tilemap.

#### Scenario: Flashing cell
- **WHEN** the flash data holds `0xF00` for one cell and the tilemap is rendered
- **THEN** that cell is redder than without the entry and its neighbours are unchanged

#### Scenario: Pulse
- **WHEN** a cell with `0xFFF` over black is drawn after 0, 4, 16, 28 and 32 updates
- **THEN** each channel is 60, 75, 120, 75 and 60

#### Scenario: Measured samples
- **WHEN** the entries `0x7ABC` and `0xFFF` over (50, 100, 150) are drawn after 4 updates, `0xF00` over (204, 51, 110) after 44, and `-1` and `0x1000` over any colour
- **THEN** the results are (100, 155, 210), (125, 175, 225), (255, 51, 110), and the colour unchanged for the last two

#### Scenario: Scrolled map
- **WHEN** the tilemap's origin is moved by 16 pixels
- **THEN** the overlay moves with its cell

### Requirement: Flash entries and flash tables of another size
Bits 12 to 14 of a flash entry SHALL be ignored and a negative entry SHALL draw nothing. A
flash table of another size than the map SHALL be indexed from the first visible column and
row, wrapped by the map's size, counted on modulo the flash table's size.

#### Scenario: Flash table of another size
- **WHEN** a map of 7 by 5 cells has a flash table of 5 by 4 and ox is 0, 40, 264, -50 and -300
- **THEN** the screen columns from the one that holds pixel ox use the flash columns starting 0, 1, 1, 0 and 4, each counted on modulo 5

### Requirement: Unrotated sprites are stretched as in the original
A sprite whose angle is a multiple of 360 SHALL be extracted as a stretch quad with
`dw = floor(w * zoom_x)`, `dh = floor(h * zoom_y)`, `x0 = floor(x - ox * zoom_x)` and
`y0 = floor(y - oy * zoom_y)`, computed in 32-bit floating point with the zoom values rounded
to that precision first, `w` and `h` being the size of its source rectangle, and with the
mirror flag for a mirrored sprite. A zoom of 0 or less SHALL draw nothing.

#### Scenario: Measured frames
- **WHEN** each committed case of zoom, origin, mirror and source rectangle measured on the original runtime is extracted and rendered
- **THEN** the listed pixels equal the measured ones

### Requirement: Stretched sprites with a cut source rectangle
When an unrotated sprite is stretched on either axis and its source rectangle leaves the
bitmap, the columns and rows SHALL be those the original draws for a cut rectangle: with `a`
the first index of the rectangle inside the bitmap, the first column is `a * dw / w` and the
walk starts anew at `a`; a rectangle cut on the far side ends at column `(b * dw - 1) / w`; a
mirrored rectangle is mirrored before it is cut.

#### Scenario: Cut source rectangles
- **WHEN** each row of the table of measured columns in the design (source rectangles leaving a 24 by 16 bitmap on each side, zooms 2, 1 by 2, 2 by 1 and 2.5 by 3, mirrored and not) is extracted and rendered
- **THEN** the columns and rows drawn and the texels they show are those of the table

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
The bush depth SHALL select the lowest screen rows of a sprite, as many as the bush depth and
not scaled by the vertical zoom, and draw them with the opacity shifted right by one.
A flash's alpha SHALL become `alpha * (r - 1) / r` in integer division at each update with `r`
frames remaining; a flash of duration 0 SHALL show until the first update; and the flash colour
SHALL replace the sprite's colour when its alpha is greater than or equal to the colour's.

#### Scenario: Measured samples
- **WHEN** each committed flash and bush sample measured on the original runtime is extracted and rendered
- **THEN** the listed pixels equal the measured ones

#### Scenario: Decay
- **WHEN** a flash with alpha 128 and duration 5 is updated five times
- **THEN** its alpha is 128, 102, 76, 50, 25 and then 0

### Requirement: Bush rows of zoomed and turned sprites
For an
unrotated sprite of source height `h` drawn `dh` rows high, with `bd = min(depth, dh)` and
`sb = bd * h / dh` in integer division, the upper `dh - bd` rows SHALL be the stretch of the
source rows above the lowest `sb` and the lower `bd` rows the stretch of those `sb` rows, each
stretched on its own, and with `sb = 0` the lower rows SHALL not be drawn. For a sprite with
an angle the faint rows SHALL be those of its unclipped bounding box, whatever texels they
show.

#### Scenario: Zoomed bush rows
- **WHEN** a sprite of 16 rows is drawn with vertical zoom 2 and bush depth 3, and with vertical zoom 2 and bush depth 1
- **THEN** the first shows the source rows 0 0 1 1 up to 13 13 14 and then row 15 three times faint, and the second is 31 rows high, ending with row 15 once, with no faint row

#### Scenario: Turned bush rows
- **WHEN** the measured case of a 24 by 16 sprite at (240, 60) with origin (12, 8), zoom 2, angle 30 and bush depth 8 is extracted and rendered
- **THEN** screen rows 35 to 77 are at full opacity and rows 78 to 85 faint

### Requirement: Window background as in the original
A window with a skin SHALL draw the skin's 128 by 128 background into the rectangle inset by 2
pixels: stretched by the stretch rule without interpolation when the numerator is 128,
stretched with the measured bilinear rule when the numerator is 127, or tiled from the inset
corner when its stretch flag is off. Its alpha SHALL be
`(skin_alpha * ((back_opacity * opacity) / 255) + 127) / 255`.

#### Scenario: Measured samples
- **WHEN** each committed background sample measured on the original runtime is extracted and rendered
- **THEN** the listed pixels equal the measured ones

#### Scenario: Bilinear class, measured
- **WHEN** each committed sample of a background stretched with the numerator 127, measured on the original runtime with a non-ramp skin, is extracted and rendered
- **THEN** the listed pixels equal the measured ones

### Requirement: Window frame as in the original
The frame SHALL be composed into a bitmap of the window's size: the four edges tiled from the
skin's 32-pixel edge pieces over columns and rows 8 to size minus 9, in the order top, bottom,
left, right, then the four 16-pixel corners, each drawn with `blt`; and that bitmap SHALL be
drawn with alpha `(bitmap_alpha * opacity + 127) / 255`. Back opacity SHALL NOT affect it.

#### Scenario: Measured samples
- **WHEN** each committed frame sample, including windows smaller than 32 pixels, is extracted and rendered
- **THEN** the listed pixels equal the measured ones

### Requirement: Window contents and arrows as in the original
Contents texel `(i - 16 + ox, j - 16 + oy)` SHALL appear at window pixel `(i, j)` inside the
interior, with the contents opacity as its only opacity. With a contents bitmap set, the arrows
SHALL appear by the measured conditions at the measured positions in the order down, left,
right, up, unaffected by any opacity.

#### Scenario: Measured samples
- **WHEN** each committed contents and arrow sample is extracted and rendered
- **THEN** the listed pixels equal the measured ones

### Requirement: Window cursor as in the original
The cursor SHALL be drawn at window position `(16 + x, 16 + y)` of its rectangle, not moved by
the window's origin, clipped by the window, below the contents, from the measured corner, edge
and centre pieces. Its opacity SHALL be `max(0, contents_opacity - 8 * m)` with `m = c` for
`c <= 16` and `32 - c` above, where the counter `c` starts at 0, advances modulo 32 with each
update of an active window, and becomes 12 when the window is made inactive.

#### Scenario: Blink sequence
- **WHEN** an active window with contents opacity 255 is updated 32 times
- **THEN** the cursor opacity goes 247, 239, down to 127 at the sixteenth update, and back up to 255

#### Scenario: Inactive
- **WHEN** a window is made inactive
- **THEN** its cursor opacity is 159 at contents opacity 255

#### Scenario: Measured samples
- **WHEN** each committed cursor sample is extracted and rendered
- **THEN** the listed pixels equal the measured ones

### Requirement: Window pause sign as in the original
The pause sign SHALL be drawn at `(W / 2 - 8, H - 16)` above the down arrow and the cursor,
with alpha `min(255, 32 * p)` and image `(p / 8) mod 4`, where `p` is 0 when `pause` is
assigned and advances with each update while pausing. No opacity SHALL affect it.

#### Scenario: Fade in
- **WHEN** `pause` is set and the window is updated once, twice and eight times
- **THEN** the sign's alpha is 32, 64 and 255, it shows its first image until the eighth update and its second from then, and before the first update it is not drawn

### Requirement: Bilinear window stretch of the original
A window stretch with the reduced numerator `N` (the source size minus 1) to `dw` by `dh`
SHALL compute, for destination pixel `(i, j)`, `ux = i * ((N << 32) / dw)` and `uy` likewise in
64-bit arithmetic, take the texel at the integer parts and its right and lower neighbours, and
weight them by the next 8 bits of `ux` and `uy` with `p + (((q - p) * f) >> 8)`, horizontally
first, alpha included. A texel with alpha 0 SHALL lend no colour. The centre of the cursor
SHALL use this rule with `N = 27`.

#### Scenario: Worked example
- **WHEN** a 128-texel background row holds 70 at texel 63 and 69 at texel 64 and is stretched to 476 pixels
- **THEN** destination pixel 238 is 69, not the nearest texel's 70

#### Scenario: Transparent neighbour
- **WHEN** one of the two texels of a row has alpha 0
- **THEN** the row's colour is the other texel's and its alpha is interpolated

#### Scenario: Cursor centre, measured
- **WHEN** each committed cursor sample measured with a non-ramp skin is extracted and rendered
- **THEN** the listed pixels equal the measured ones

### Requirement: Assigning active and pause
Assigning a window's `active` or `pause` from a script SHALL change the cursor and pause
counters as the measured rules say at the moment of the assignment, including two assignments
without an update or frame between them.

#### Scenario: Off and on at once
- **WHEN** a script evaluates `w.active = false; w.active = true` on an active window whose blink counter is 5, with nothing in between
- **THEN** the counter is 12 afterwards

### Requirement: Rotated sprites as in the original
A sprite with an angle other than exactly 0 SHALL be drawn as the original computes it, in
double precision: its rotated corners, rounded with `floor(v + 0.5)`, give a bounding box
whose last row and column are not drawn; in each row the source position is computed at the
box's left edge and advanced by a constant step per pixel; a pixel shows the texel at the
truncated position when that is inside the source rectangle. Opacity, blend mode, tone and
colour SHALL apply as for an unrotated sprite.

#### Scenario: Measured frames
- **WHEN** each committed rotation case measured on the original runtime (right angles, 360, generic angles, zoom, mirror, a source rectangle, a sprite partly off screen) is extracted and rendered
- **THEN** every pixel of the listed region equals the measured one

#### Scenario: Full turn is not the unrotated path
- **WHEN** a sprite has angle 360
- **THEN** it is drawn by this rule, with the last row and column of its box missing

### Requirement: Turned sprites in viewports and with cut source rectangles
Inside a viewport the bounding box of a sprite with an angle SHALL be computed from the
position less the viewport's origin and placed at the viewport's corner, while the source
positions are computed from the position itself. A sprite with an angle whose source rectangle
does not lie entirely inside its bitmap SHALL not be drawn.

#### Scenario: Viewport with an origin
- **WHEN** each committed case of a turned sprite in a viewport measured on the original runtime (origins (0,0), (13,7), (-9,-5) and (100,-50), viewports partly off screen) is extracted and rendered
- **THEN** every pixel of the listed rows equals the measured one, and the cases with the origin (100,-50) draw nothing

#### Scenario: Source rectangle leaving the bitmap
- **WHEN** a sprite with angle 30 has the source rectangle (-1, 0, 24, 16) on a bitmap of 24 by 16
- **THEN** nothing of it is drawn

### Requirement: Tiles are blended one by one
Where several tiles of a tilemap cover the same screen pixel, each SHALL be blended onto the
frame by itself with the sprite blend formula, in ascending priority and, within one priority,
in the order of the map's layers; tiles of one priority SHALL NOT be composed with each other
first.

#### Scenario: Two translucent tiles of one priority, measured
- **WHEN** layer 0 holds an opaque tile of colour (6, 14, 2) with priority 0 and layers 1 and 2 hold the same translucent tile of colour (138, 130, 150) and alpha 142 with priority 2
- **THEN** the pixel is (111, 107, 120)

#### Scenario: Opaque map unchanged
- **WHEN** a map whose tiles are opaque or sit on an opaque tile of their priority is rendered
- **THEN** the frame equals the one before this change

### Requirement: Tilemaps are extracted as tilemap ops
A tilemap SHALL be extracted as tilemap layer ops and not as bitmaps composed by the guest:
for priority 0 one op per map layer that has such tiles in view, at z 0; for each priority
from 1 to 5, one op per visible map row and map layer that has such tiles in that row, clipped
to the row's 32 screen rows and drawn at the row's z, within a priority in the order of the
map's layers. The tables SHALL be sent when they are first used and again only when their
content has changed.

#### Scenario: Same pictures as before
- **WHEN** the tilemap test scenes of this crate (scrolled, wrapped, with autotiles, priorities, translucent stacked tiles and a viewport) are extracted and rendered
- **THEN** every frame equals its committed expectation, which was recorded while the former cached-bitmap implementation still produced the same frame

#### Scenario: Scrolling uploads nothing
- **WHEN** a tilemap's origin moves by 100 pixels over ten frames with unchanged tables and bitmaps
- **THEN** the packets after the first carry no table and no bitmap change for the tilemap

#### Scenario: Table change
- **WHEN** one cell of the map table is changed through the shared table
- **THEN** the next packet replaces that table and the frame shows the new tile

#### Scenario: Rows among sprites
- **WHEN** a sprite stands between two map rows that hold tiles of priority 1
- **THEN** the upper row's tiles are behind it and the lower row's in front, as before

### Requirement: Brightness of the scene
The scene SHALL keep a brightness that starts at 255, SHALL clamp a value that is set to 0 to
255, and SHALL write it into every extracted packet.

#### Scenario: Default
- **WHEN** a frame is extracted from a new scene
- **THEN** the packet carries the brightness 255

#### Scenario: Limits
- **WHEN** the brightness is set to -5 and then to 300
- **THEN** it reads back 0 and then 255, and the next packet carries 255

#### Scenario: Carried by the packet
- **WHEN** the brightness is set to 100 and two frames are extracted
- **THEN** both packets carry the brightness 100

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

### Requirement: Tilemap of RGSS2 in the scene
A tilemap of RGSS2 SHALL be extracted as two plane ops among the elements of its viewport, the
lower at z 0 and the upper at z 200, from the origin `ox + viewport.ox`, `oy + viewport.oy`
with the map wrapped, whatever its `visible` is. Its counter SHALL advance only by its
update, giving the animation step `counter / 30` and the flash level
`4 + [0, 1, 2, 3, 4, 3, 2, 1][(counter / 4) % 8]`.

#### Scenario: Two planes, measured on the original
- **WHEN** a tilemap with a star tile in layer 2 shares a viewport with sprites at z 100 and z 300
- **THEN** the first sprite is under the star tile and over the other tiles, and the second is over both

#### Scenario: Invisible, measured on the original
- **WHEN** `visible = false` is set on a tilemap
- **THEN** the next frame shows the tilemap as before

#### Scenario: Counter, measured on the original
- **WHEN** a tilemap is updated 29 times, then once more, with frames in between that do not update it
- **THEN** the animation step is 0 until the thirtieth update and 1 after it

#### Scenario: Scrolling
- **WHEN** ox is -40 on a map 5 cells wide
- **THEN** the first cell is 3 and the plane's corner is 24 pixels left of the viewport's corner

### Requirement: Latched viewport origin of a tilemap of RGSS2
A tilemap of RGSS2 SHALL take the ox and oy of its viewport as they were at its last refresh:
a write of `ox`, `oy`, `map_data`, `flash_data`, `passages` or a bitmap slot, an update, or a
change of the viewport's ox or oy; assigning another viewport SHALL NOT refresh.

#### Scenario: Other viewport, measured on the original
- **WHEN** a tilemap in a viewport with ox 0 is assigned a viewport with ox 16 and a frame is taken, then `ox = ox` is evaluated and another is taken
- **THEN** the first frame shows the map at the origin of the old viewport and the second moved by 16

### Requirement: Draw order of RGSS2
In a scene of RGSS2 elements SHALL be drawn in ascending order of (z, y, creation), where y is
the integer y of a sprite and 0 for every other element, and creation is the order in which
the elements were made, which nothing renews. A viewport SHALL take its place among the
top-level elements by (its z, 0, its creation), and its own elements SHALL be ordered by the
same key among themselves. A scene of RGSS1 SHALL keep "Draw order".

#### Scenario: y breaks the tie, measured on the original
- **WHEN** two overlapping sprites of equal z are made, the first with y 9 and the second with y 0
- **THEN** the first is drawn over the second

#### Scenario: The property, not the place, measured on the original
- **WHEN** an older sprite has `y = 20, oy = 20` and a younger one of equal z has `y = 0`
- **THEN** the older is drawn over the younger

#### Scenario: Viewport, measured on the original
- **WHEN** a sprite in a viewport has y 50 and a younger top-level sprite of the viewport's z has y 0
- **THEN** the top-level sprite is drawn over the one in the viewport

#### Scenario: Window, measured on the original
- **WHEN** an older sprite with y 50 and a younger window have equal z, and likewise with y -50
- **THEN** the sprite is over the window in the first case and under it in the second

#### Scenario: Cases of the measurement
- **WHEN** each committed case of the measurement's order topic is extracted and composed
- **THEN** every pixel equals the expected picture, and the same scenes in RGSS1 keep the order of RGSS1

### Requirement: Viewport flash of RGSS2
In a scene of RGSS2 a flashing viewport SHALL apply its tone, then its colour, then the flash
colour at its current alpha, and a flash without a colour SHALL, while it lasts, draw none of
the viewport's elements and apply neither its tone nor its colour. A scene of RGSS1 SHALL keep
"Viewport effects".

#### Scenario: Colour and flash, measured on the original
- **WHEN** each committed case of a viewport with a colour and a flash is composed
- **THEN** every pixel equals the expected picture

#### Scenario: Flash without a colour, measured on the original
- **WHEN** a viewport with a tone and a sprite flashes with `nil` for 5 updates
- **THEN** the frames of those updates show what is behind the viewport unchanged

### Requirement: Plane tile size of RGSS2
In a scene of RGSS2 the tile of a plane SHALL be `floor(w * zoom_x)` by `floor(h * zoom_y)`
pixels computed in double precision. A scene of RGSS1 SHALL keep "Planes tile a stretched
tile".

#### Scenario: Measured on the original
- **WHEN** a plane of a bitmap 90 high has `zoom_y = 0.7`
- **THEN** it repeats every 62 rows in RGSS2 and every 63 rows in RGSS1

### Requirement: Sprite placement of RGSS2
In a scene of RGSS2 an unrotated sprite SHALL start at `x0 = trunc(x - ox * zoom_x)` and be
`dw = floor(x0 + w * zoom_x) - x0` wide, likewise in y, in double precision and in the
coordinates of its viewport, and SHALL draw nothing when a zoom or a size is 0 or less. An
unstretched mirrored sprite whose source rectangle leaves the bitmap SHALL be cut first and
mirrored inside what remains. A scene of RGSS1 SHALL keep its rules.

#### Scenario: Truncation, measured on the original
- **WHEN** a sprite has `x = 0, ox = 5, zoom_x = 2.5`
- **THEN** its left edge is at -12 in RGSS2 and at -13 in RGSS1

#### Scenario: Width depends on the place, measured on the original
- **WHEN** a sprite 90 wide with `zoom_x = 0.7` stands at x 0 and another at x 272
- **THEN** the first is 62 and the second 63 pixels wide

#### Scenario: Mirrored cut, measured on the original
- **WHEN** a sprite of a bitmap 24 by 16 has `src_rect = Rect.new(-5, 6, 18, 17)` and `mirror = true`
- **THEN** its columns 5 to 17 show the texels 12 down to 0 and the other columns nothing

#### Scenario: Cases of the measurement
- **WHEN** each committed case of placement and size is extracted and composed
- **THEN** every pixel equals the expected picture

### Requirement: Bush rows of RGSS2
In a scene of RGSS2 the bush rows of a sprite SHALL be drawn at the opacity
`opacity * bush_opacity / 255`, truncated, and not at all when that is below 0 or above 255,
without the sprite's tone, colour and flash colour and, when the sprite is unrotated, without
its mirror, from the source rows rule 3 of this change's design names. A scene of RGSS1 SHALL
keep its rules.

#### Scenario: Opacity, measured on the original
- **WHEN** bush rows are drawn at `opacity 255, bush_opacity 128`, at `161, 300` and at `255, 256`
- **THEN** their opacity is 128, 189, and they are not drawn

#### Scenario: Depth beyond the height, measured on the original
- **WHEN** a sprite of 16 rows at `zoom_y = 2.25` has `bush_depth` 41 and then 37
- **THEN** its 36 rows show the source rows of the two committed measured sequences

#### Scenario: No tone on bush rows, measured on the original
- **WHEN** a sprite with a tone, a colour and a bush depth is composed
- **THEN** the bush rows show the texels without tone and colour and the other rows with them

### Requirement: Wave of a sprite of RGSS2
In a scene of RGSS2 each screen row of an unrotated sprite with a positive wave amplitude
SHALL be moved horizontally by the offset rule 5 of this change's design gives, with the texel
rows of that rule, and a negative amplitude SHALL squeeze the sprite as that rule says. The
sprite's update SHALL add `wave_speed / wave_length` to the phase modulo 360, also at
amplitude 0 and when invisible, and SHALL leave the phase when the length is 0.

#### Scenario: Offsets, measured on the original
- **WHEN** the offset is computed for each committed row of the wave samples measured on the original runtime of VX
- **THEN** every offset equals the measured value

#### Scenario: Exact half
- **WHEN** the amplitude is 13 at 330 degrees, and 21 at 30 degrees
- **THEN** the offsets are -6 and 10

#### Scenario: Phase
- **WHEN** a sprite with the defaults is updated once, and one with `wave_length = 64`
- **THEN** the phases are 2.0 and 5.625

#### Scenario: Cases of the measurement
- **WHEN** each committed wave case (zoom, mirror, source rectangle, viewports, bush, negative amplitude) is extracted and composed
- **THEN** every pixel equals the expected picture

### Requirement: Whole sprites of RGSS2 as measured
The CPU reference and the Vulkan backend SHALL draw scenes of sprites of RGSS2 as the model of
the measurement does.

#### Scenario: Random scenes
- **WHEN** twenty committed scenes of the measurement's held-out generator are composed by both backends
- **THEN** every frame equals its expected picture, byte for byte

### Requirement: Tilemap of RGSS3 in the scene
The scene SHALL be able to create a tilemap of RGSS3. While it is visible it SHALL be
extracted as the tilemap of RGSS2 is: two plane ops at z 0 and z 200 with the same origin,
wrapping, counter, animation step and flash level, but naming the rule set of RGSS3, with
its flags table in the place of the passages table and its map table sent with every layer
it has.

#### Scenario: Two planes of the rule set of RGSS3
- **WHEN** a tilemap of RGSS3 with a map of 4 by 3 by 4 elements and a flags table is extracted
- **THEN** the packet has a lower and an upper plane op that both name the rule set of RGSS3, the map table with its 48 elements and the flags table

#### Scenario: Star tiles among sprites, measured on the original
- **WHEN** a tilemap of RGSS3 has the tile 300 with the flag 0x10 in layer 2 of one cell and in layer 0 of another, and shares a viewport with sprites at z 100 and z 300 that cover both cells
- **THEN** the sprite at z 100 is under the tile of layer 2 and over the tile of layer 0, and the sprite at z 300 is over both

#### Scenario: A shadow from layer 3, measured on the original
- **WHEN** a tilemap of RGSS3 over a sprite of level 200 at z -10 has 15 in layer 3 of cell (1, 1) of an otherwise empty map
- **THEN** the frame shows 99 in the 32 by 32 pixels of that cell and 200 around it

#### Scenario: Counter and scrolling as RGSS2
- **WHEN** a tilemap of RGSS3 on a map 5 cells wide has ox -40 and is updated 30 times
- **THEN** the first cell is 3, the plane's corner is 24 pixels left of the viewport's corner, and the animation step is 1

#### Scenario: Latch as RGSS2, measured on the original
- **WHEN** a tilemap of RGSS3 in a viewport with ox 0 is assigned a viewport with ox 16 and a frame is extracted, then the tilemap is refreshed and another is extracted
- **THEN** the first frame has the origin of the old viewport and the second is moved by 16

### Requirement: Visibility of a tilemap of RGSS3
While the `visible` of a tilemap of RGSS3 is false, the scene SHALL extract no plane op for
it, so that neither its tiles, its shadows nor its flash are drawn. Its update counter SHALL
go on. A tilemap of RGSS2 SHALL be extracted whatever its `visible` is, as before.

#### Scenario: Hidden, measured on the original
- **WHEN** a tilemap of RGSS3 with tiles, a star tile, a shadow and a flash table is extracted, then `visible` is set to false and it is extracted again
- **THEN** the first packet has two plane ops and the second has none

#### Scenario: Back, measured on the original
- **WHEN** such a tilemap is updated 30 times while `visible` is false and `visible` is then set to true
- **THEN** the next packet has two plane ops whose animation step is 1

#### Scenario: RGSS2 stays, measured on the original
- **WHEN** `visible = false` is set on a tilemap of RGSS2
- **THEN** the next packet has its two plane ops as before

### Requirement: Tilemaps of RGSS1 and RGSS2 stay as they are
Adding the tilemap of RGSS3 SHALL NOT change what a tilemap of RGSS1 or of RGSS2 extracts:
a tilemap of RGSS2 names the rule set of RGSS2 in its plane ops, and no expected value of an
existing test of tilemaps is edited.

#### Scenario: Existing tests
- **WHEN** the tests of `crates/vkxp-core/src/tilemap/tests.rs` and `crates/vkxp-core/src/tilemap/rgss2/tests.rs` that existed before this change run
- **THEN** they pass with their expected values as they were, and every line that `crates/vkxp-core/tests/data/tilemap-frames.txt` had is unchanged

#### Scenario: Rule set of RGSS2
- **WHEN** a tilemap created as one of RGSS2 is extracted
- **THEN** both its plane ops name the rule set of RGSS2
