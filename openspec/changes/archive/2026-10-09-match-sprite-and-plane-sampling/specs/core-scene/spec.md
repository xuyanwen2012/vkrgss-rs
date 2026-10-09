# Spec Delta

## ADDED Requirements

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
