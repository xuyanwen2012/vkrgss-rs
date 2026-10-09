# Spec Delta

## ADDED Requirements

### Requirement: Window background as in the original
A window with a skin SHALL draw the skin's 128 by 128 background into the rectangle inset by 2
pixels: stretched without interpolation by the stretch rule, with the measured choice between
the numerators 128 and 127, or tiled from the inset corner when its stretch flag is off. Its
alpha SHALL be `(skin_alpha * ((back_opacity * opacity) / 255) + 127) / 255`.

#### Scenario: Measured samples
- **WHEN** each committed background sample measured on the original runtime is extracted and rendered
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

## REMOVED Requirements

### Requirement: Window frame and background
**Reason**: Replaced by the measured rules in "Window background as in the original" and "Window frame as in the original".
**Migration**: None.

### Requirement: Window contents
**Reason**: Replaced by "Window contents and arrows as in the original".
**Migration**: None.

### Requirement: Window cursor
**Reason**: Replaced by "Window cursor as in the original".
**Migration**: None.

### Requirement: Window pause sign and scroll arrows
**Reason**: Replaced by "Window pause sign as in the original" and "Window contents and arrows as in the original".
**Migration**: None.
