## ADDED Requirements

### Requirement: Extraction errors are not hidden
Extraction SHALL NOT turn an error of the bitmap arena into a packet without bitmap changes;
the error SHALL reach the caller.

#### Scenario: Arena error
- **WHEN** a test makes the bitmap arena fail while a frame is extracted
- **THEN** extraction reports the error instead of returning a packet

### Requirement: Bound of the quads of a plane
A plane SHALL emit at most one quad per screen pixel it covers, also when its stretch step is
not exact.

#### Scenario: Smallest tile
- **WHEN** a plane whose bitmap is 1 by 1 at zoom 1 covers a screen of 640 by 480, and another of 300 by 300 at zoom 0.01 does
- **THEN** each extracted packet holds at most 307200 quads for the plane
