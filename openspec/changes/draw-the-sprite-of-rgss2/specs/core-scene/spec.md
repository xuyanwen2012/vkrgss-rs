## ADDED Requirements

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
