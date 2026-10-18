## ADDED Requirements

### Requirement: The copy of a game keeps its file times
The oracle SHALL give every file of its copy of a game the modification time of its source, and
the files it writes itself a fixed time, on every platform.

#### Scenario: Times of the copy
- **WHEN** a game whose two save files have different modification times is copied by the oracle on Linux and on macOS
- **THEN** each copied file has the modification time of its source on both
