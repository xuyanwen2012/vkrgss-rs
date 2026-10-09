# Spec Delta

## ADDED Requirements

### Requirement: Stretch sampling of quads
A stretch quad with destination rectangle `(x0, y0, dw, dh)` and a source rectangle of size
`(w, h)` SHALL cover the pixels of the destination rectangle inside its clip, and pixel
`(x0 + i, y0 + j)` SHALL show the source texel at column `(i * ((w << 20) / dw)) >> 20` and
row `(j * ((h << 20) / dh)) >> 20`, the column counted from the right edge when the mirror
flag is set. CPU reference and GPU SHALL agree bit for bit.

#### Scenario: Measured samples
- **WHEN** each committed sprite zoom sample measured on the original runtime is drawn as a stretch quad
- **THEN** every listed screen pixel shows the measured texel

#### Scenario: Clipped
- **WHEN** a stretch quad's destination rectangle starts 100 pixels left of its clip rectangle
- **THEN** the first visible column shows the texel of `i = 100`, not of `i = 0`

#### Scenario: Unscaled
- **WHEN** the destination size equals the source size
- **THEN** every pixel shows the texel at the same offset

#### Scenario: Empty destination
- **WHEN** the destination width or height is 0 or less
- **THEN** nothing is drawn
