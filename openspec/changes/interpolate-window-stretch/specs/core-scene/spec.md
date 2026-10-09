# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
