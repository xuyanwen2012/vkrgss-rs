# Spec Delta

## MODIFIED Requirements

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

### Requirement: Bush depth and flash details of the original
The bush depth SHALL select the lowest screen rows of a sprite, as many as the bush depth and
not scaled by the vertical zoom, and draw them with the opacity shifted right by one. For an
unrotated sprite of source height `h` drawn `dh` rows high, with `bd = min(depth, dh)` and
`sb = bd * h / dh` in integer division, the upper `dh - bd` rows SHALL be the stretch of the
source rows above the lowest `sb` and the lower `bd` rows the stretch of those `sb` rows, each
stretched on its own, and with `sb = 0` the lower rows SHALL not be drawn. For a sprite with
an angle the faint rows SHALL be those of its unclipped bounding box, whatever texels they
show.
A flash's alpha SHALL become `alpha * (r - 1) / r` in integer division at each update with `r`
frames remaining; a flash of duration 0 SHALL show until the first update; and the flash colour
SHALL replace the sprite's colour when its alpha is greater than or equal to the colour's.

#### Scenario: Measured samples
- **WHEN** each committed flash and bush sample measured on the original runtime is extracted and rendered
- **THEN** the listed pixels equal the measured ones

#### Scenario: Decay
- **WHEN** a flash with alpha 128 and duration 5 is updated five times
- **THEN** its alpha is 128, 102, 76, 50, 25 and then 0

#### Scenario: Zoomed bush rows
- **WHEN** a sprite of 16 rows is drawn with vertical zoom 2 and bush depth 3, and with vertical zoom 2 and bush depth 1
- **THEN** the first shows the source rows 0 0 1 1 up to 13 13 14 and then row 15 three times faint, and the second is 31 rows high, ending with row 15 once, with no faint row

#### Scenario: Turned bush rows
- **WHEN** the measured case of a 24 by 16 sprite at (240, 60) with origin (12, 8), zoom 2, angle 30 and bush depth 8 is extracted and rendered
- **THEN** screen rows 35 to 77 are at full opacity and rows 78 to 85 faint

### Requirement: Rotated sprites as in the original
A sprite with an angle other than exactly 0 SHALL be drawn as the original computes it, in
double precision: its rotated corners, rounded with `floor(v + 0.5)`, give a bounding box
whose last row and column are not drawn; in each row the source position is computed at the
box's left edge and advanced by a constant step per pixel; a pixel shows the texel at the
truncated position when that is inside the source rectangle. Opacity, blend mode, tone and
colour SHALL apply as for an unrotated sprite. Inside a viewport the bounding box
SHALL be computed from the position less the viewport's origin and placed at the viewport's
corner, while the source positions are computed from the position itself. A sprite with an
angle whose source rectangle does not lie entirely inside its bitmap SHALL not be drawn.

#### Scenario: Measured frames
- **WHEN** each committed rotation case measured on the original runtime (right angles, 360, generic angles, zoom, mirror, a source rectangle, a sprite partly off screen) is extracted and rendered
- **THEN** every pixel of the listed region equals the measured one

#### Scenario: Full turn is not the unrotated path
- **WHEN** a sprite has angle 360
- **THEN** it is drawn by this rule, with the last row and column of its box missing

#### Scenario: Viewport with an origin
- **WHEN** each committed case of a turned sprite in a viewport measured on the original runtime (origins (0,0), (13,7), (-9,-5) and (100,-50), viewports partly off screen) is extracted and rendered
- **THEN** every pixel of the listed rows equals the measured one, and the cases with the origin (100,-50) draw nothing

#### Scenario: Source rectangle leaving the bitmap
- **WHEN** a sprite with angle 30 has the source rectangle (-1, 0, 24, 16) on a bitmap of 24 by 16
- **THEN** nothing of it is drawn

### Requirement: Unrotated sprites are stretched as in the original
A sprite whose angle is a multiple of 360 SHALL be extracted as a stretch quad with
`dw = floor(w * zoom_x)`, `dh = floor(h * zoom_y)`, `x0 = floor(x - ox * zoom_x)` and
`y0 = floor(y - oy * zoom_y)`, computed in 32-bit floating point with the zoom values rounded
to that precision first, `w` and `h` being the size of its source rectangle, and with the
mirror flag for a mirrored sprite. A zoom of 0 or less SHALL draw nothing. When the sprite is stretched on
either axis and its source rectangle leaves the bitmap, the columns and rows SHALL be those
the original draws for a cut rectangle: with `a` the first index of the rectangle inside the
bitmap, the first column is `a * dw / w` and the walk starts anew at `a`; a rectangle cut on
the far side ends at column `(b * dw - 1) / w`; a mirrored rectangle is mirrored before it is
cut.

#### Scenario: Measured frames
- **WHEN** each committed case of zoom, origin, mirror and source rectangle measured on the original runtime is extracted and rendered
- **THEN** the listed pixels equal the measured ones

#### Scenario: Cut source rectangles
- **WHEN** each row of the table of measured columns in the design (source rectangles leaving a 24 by 16 bitmap on each side, zooms 2, 1 by 2, 2 by 1 and 2.5 by 3, mirrored and not) is extracted and rendered
- **THEN** the columns and rows drawn and the texels they show are those of the table
