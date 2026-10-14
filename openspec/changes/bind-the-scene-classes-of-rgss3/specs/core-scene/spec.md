## ADDED Requirements

### Requirement: Disposing a viewport without its elements
The scene SHALL offer, beside disposing a viewport with its elements, disposing a viewport
alone: the viewport SHALL be gone, and every sprite, plane, window and tilemap in it SHALL
stay alive, keep its viewport handle and be drawn nowhere until it is moved out. A disposed
or unknown handle SHALL change nothing. Disposing a viewport with its elements SHALL stay as
it is.

#### Scenario: The elements stay
- **WHEN** a sprite, a plane, a window of RGSS3 and a tilemap of RGSS3 are created in a viewport, a sprite is created without one, and the viewport is disposed alone
- **THEN** the viewport is disposed, none of the five elements is, the four report the viewport handle, and their properties can be read and set

#### Scenario: Drawn nowhere
- **WHEN** each of the four had drawn opaque pixels inside the viewport's rectangle and a frame is extracted and composed after the viewport is disposed alone
- **THEN** the frame is extracted without an error and shows only the sprite without a viewport

#### Scenario: Moved out
- **WHEN** the sprite of that viewport is then moved out of any viewport
- **THEN** it is drawn on the screen at its position, and moving it into the disposed viewport fails with the disposed error of a viewport

#### Scenario: Twice and a reused slot
- **WHEN** the same handle is disposed alone again, and a new viewport that takes its slot is created
- **THEN** nothing changes, and the four elements are not in the new viewport

#### Scenario: With the elements, as before
- **WHEN** the tests of `vkxp-core` that dispose a viewport with its elements run
- **THEN** they pass with their expected values unedited
