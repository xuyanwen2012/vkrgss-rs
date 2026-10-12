## ADDED Requirements

### Requirement: Brightness of the frame
After the last op of a packet, a compositor SHALL replace each colour channel `v` of every
pixel of the frame by `(v * b) >> 8`, where `b` is the brightness of the packet, unless `b` is
255, in which case the frame SHALL stay as composed. The alpha of the frame SHALL NOT change.

#### Scenario: Measured on the original runtime of VX
- **WHEN** a frame whose pixels have the level 200 in every colour channel is composed at brightness 100
- **THEN** every colour channel of the frame is 78

#### Scenario: Identity
- **WHEN** any packet is composed at brightness 255
- **THEN** the frame equals the frame of the same packet before this change, byte for byte

#### Scenario: Black
- **WHEN** a frame of any content is composed at brightness 0
- **THEN** every colour channel of the frame is 0

#### Scenario: Every level
- **WHEN** frames that hold every channel value from 0 to 255 are composed at each brightness from 0 to 254
- **THEN** every channel equals `(v * b) >> 8`, in the reference and in the Vulkan backend alike
