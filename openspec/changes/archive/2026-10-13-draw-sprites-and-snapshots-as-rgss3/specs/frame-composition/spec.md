## ADDED Requirements

### Requirement: Pixels of a snapshot
The reference compositor SHALL offer the pixels of a snapshot of a packet that stands alone,
by a rule: by that of RGSS2 the composed frame, brightness applied, with alpha 255 in every
pixel; by that of RGSS3 the same, except that every byte of every pixel is 0 when the
brightness of the packet is 0. The colour channels SHALL be those of "Brightness of the
frame", and the alpha of the composed frame SHALL play no part.

#### Scenario: Darkened, measured on the originals
- **WHEN** a packet whose frame is the opaque colour (200, 100, 50) is snapped at brightness 100 by each rule
- **THEN** every pixel is (78, 39, 19, 255)

#### Scenario: Full brightness
- **WHEN** the same packet is snapped at brightness 255 by each rule
- **THEN** every pixel is (200, 100, 50, 255)

#### Scenario: Brightness 0 of RGSS3, measured on the original
- **WHEN** the same packet is snapped at brightness 0 by the rule of RGSS3, and at brightness 1
- **THEN** every pixel is (0, 0, 0, 0) in the first snapshot and (0, 0, 0, 255) in the second

#### Scenario: Brightness 0 of RGSS2
- **WHEN** the same packet is snapped at brightness 0 by the rule of RGSS2
- **THEN** every pixel is (0, 0, 0, 255), as `Graphics.snap_to_bitmap` of RGSS2 gave before this change

#### Scenario: Nothing drawn, measured on the original of RGSS3
- **WHEN** a packet without an op is snapped by the rule of RGSS3 at brightness 255, and at brightness 0
- **THEN** every pixel is (0, 0, 0, 255) in the first snapshot and (0, 0, 0, 0) in the second

#### Scenario: Translucent over nothing, measured on the original of RGSS3
- **WHEN** a packet that draws a quad of the texel (255, 255, 255, 128) at opacity 255 over nothing is snapped by the rule of RGSS3 at brightness 255
- **THEN** the pixels of the quad are (128, 128, 128, 255)

#### Scenario: Every level, measured on the original of RGSS3
- **WHEN** a packet whose frame holds every channel value from 0 to 255 is snapped by the rule of RGSS3 at each brightness from 1 to 254
- **THEN** every channel `v` is `(v * b) >> 8` and every alpha 255

#### Scenario: The scene as it is and the brightness of the call
- **WHEN** a scene shows a red sprite, a frame is extracted, the sprite's bitmap is made blue and the brightness set to 100 without another frame, and the scene's snapshot packet is snapped by the rule of RGSS3
- **THEN** the pixels of the sprite are (0, 0, 99, 255), and the next extracted frame is the one it would have been without the snapshot
