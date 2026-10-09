# Spec Delta

## Purpose
Defines the developer tool that runs one game in the original RGSS runtime and in vkxp under
the same conditions and reports how their frames differ.

## ADDED Requirements

### Requirement: Probe injection
The harness SHALL work on a copy of the game directory and SHALL add one probe script as the
first entry of the copy's script archive, leaving the other entries and the original directory
unchanged. An archive written by the harness SHALL be readable by the original runtime and by
vkxp.

#### Scenario: Archive round trip
- **WHEN** an archive with three scripts is read, a probe is added and the result is read again
- **THEN** it holds four scripts, the probe first, and the three others byte for byte as before

#### Scenario: Source untouched
- **WHEN** a harness run has finished
- **THEN** no file of the given game directory has changed

### Requirement: Same conditions on both sides
The probe SHALL seed the random number generator with a fixed value and SHALL answer
`Input.press?`, `Input.trigger?`, `Input.repeat?`, `Input.dir4` and `Input.dir8` from the
input schedule given to the harness, by frame number, with the trigger and repeat rules of the
original, so that both runtimes see the same input at the same frame.

#### Scenario: Scheduled input
- **WHEN** the schedule holds RIGHT from frame 60 to frame 100 and the probe runs in vkxp
- **THEN** `Input.press?(Input::RIGHT)` is true exactly in the frames 60 to 99, and `Input.trigger?(Input::RIGHT)` only in frame 60

### Requirement: Frame capture
For each frame number asked for, the harness SHALL obtain the 640 by 480 picture each runtime
shows after that many frames, counted the same way on both sides. On the original side the
probe SHALL hold the game at that frame until the harness has captured the game window from
the virtual display.

#### Scenario: Capture in vkxp
- **WHEN** frames 10 and 20 are asked for on a small generated game
- **THEN** two 640 by 480 pictures are written for vkxp, and they equal the dumps of a plain headless run of the same game at those frames

#### Scenario: Capture in the original
- **WHEN** the original runtime, Wine and Xvfb are available and frames 10 and 20 are asked for
- **THEN** two 640 by 480 pictures of the original's window are written

### Requirement: Report
The harness SHALL write, for every captured frame, the two pictures, a difference picture, the
number of differing pixels and the largest difference of any channel, and a summary file
listing these numbers for all frames. It SHALL exit with status 0 when both runs completed,
whatever the differences are.

#### Scenario: Identical pictures
- **WHEN** two equal pictures are compared
- **THEN** the report says 0 differing pixels and a largest difference of 0

#### Scenario: One pixel differs
- **WHEN** the pictures differ in one pixel by 3 in the red channel
- **THEN** the report says 1 differing pixel and a largest difference of 3

### Requirement: Missing prerequisites
When Wine, Xvfb, the DLL or the game directory is missing, the harness SHALL end with an error
that names what is missing, and SHALL be able to run the vkxp side alone when asked to.

#### Scenario: No DLL
- **WHEN** the harness is run with a DLL path that does not exist
- **THEN** it exits with a non-zero status and names the path
