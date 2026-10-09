# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: Rotated sprites sample at integer offsets
**Reason**: Replaced by the exact rule of "Rotated sprites as in the original".
**Migration**: None.
