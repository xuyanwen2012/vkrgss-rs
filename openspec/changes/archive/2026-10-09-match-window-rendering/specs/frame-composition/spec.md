# Spec Delta

## ADDED Requirements

### Requirement: Window blend arithmetic of the original
A quad in the window blend mode SHALL combine a source channel `s` with the channel underneath
`d` at effective alpha `a` as `(s * (a + 1) + d * (256 - a)) >> 8`, leaving the pixel unchanged
for `a` equal to 0. In each row, when the part of the quad that is visible inside its clip and
the frame has an odd number of pixels, the last of them SHALL use the normal mode's formula
instead. CPU reference and GPU SHALL agree bit for bit.

#### Scenario: Measured samples
- **WHEN** each committed window blend sample measured on the original runtime is evaluated
- **THEN** the result equals the measured value

#### Scenario: Odd row
- **WHEN** a window-mode quad's visible part is 5 pixels wide
- **THEN** its first four columns use the window formula and the fifth the normal formula
