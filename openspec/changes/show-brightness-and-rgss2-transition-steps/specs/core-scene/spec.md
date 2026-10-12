## ADDED Requirements

### Requirement: Brightness of the scene
The scene SHALL keep a brightness that starts at 255, SHALL clamp a value that is set to 0 to
255, and SHALL write it into every extracted packet.

#### Scenario: Default
- **WHEN** a frame is extracted from a new scene
- **THEN** the packet carries the brightness 255

#### Scenario: Limits
- **WHEN** the brightness is set to -5 and then to 300
- **THEN** it reads back 0 and then 255, and the next packet carries 255

#### Scenario: Carried by the packet
- **WHEN** the brightness is set to 100 and two frames are extracted
- **THEN** both packets carry the brightness 100
