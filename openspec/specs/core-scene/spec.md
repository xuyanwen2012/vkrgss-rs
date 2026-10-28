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
- **WHEN** the tests of `crates/vkrgss-core/src/tilemap/tests.rs` and `crates/vkrgss-core/src/tilemap/rgss2/tests.rs` that existed before this change run
- **THEN** they pass with their expected values as they were, and every line that `crates/vkrgss-core/tests/data/tilemap-frames.txt` had is unchanged

#### Scenario: Rule set of RGSS2
- **WHEN** a tilemap created as one of RGSS2 is extracted
- **THEN** both its plane ops name the rule set of RGSS2

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
- **WHEN** the scenes `g6-skin_sizes_t` and `g8-smallskin_t_0` of `crates/vkrgss-core/tests/data/window-rgss2-cases.txt` are composed
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

### Requirement: Extraction errors are not hidden
Extraction SHALL NOT turn an error of the bitmap arena into a packet without bitmap changes;
the error SHALL reach the caller.

#### Scenario: Arena error
- **WHEN** a test makes the bitmap arena fail while a frame is extracted
- **THEN** extraction reports the error instead of returning a packet

### Requirement: Bound of the quads of a plane
A plane SHALL emit at most one quad per screen pixel it covers, also when its stretch step is
not exact.

#### Scenario: Smallest tile
- **WHEN** a plane whose bitmap is 1 by 1 at zoom 1 covers a screen of 640 by 480, and another of 300 by 300 at zoom 0.01 does
- **THEN** each extracted packet holds at most 307200 quads for the plane
