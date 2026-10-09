# oracle-harness Specification

## Purpose
Defines the developer tool that runs one game in the original RGSS runtime and in vkxp under
the same conditions and reports how their frames differ.

## Requirements

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

### Requirement: Concurrent runs
Several harness runs with different output directories SHALL be able to run at the same time
on one machine: each SHALL use a virtual display of its own, chosen by the display server, and
SHALL stop only the display server and Wine server that it started.

#### Scenario: Display chosen by the server
- **WHEN** the harness starts its virtual display while another display server already runs on the number the harness would have probed first
- **THEN** it obtains another number from the server it started and connects to that

#### Scenario: Server that fails to start
- **WHEN** the display server exits without announcing a display
- **THEN** the run ends with an error that says so, within the start-up timeout

### Requirement: Archives with large identifiers
The harness SHALL read script archives whose entries have identifiers stored as Bignums and
SHALL write them back unchanged.

#### Scenario: Bignum identifier
- **WHEN** an archive whose second entry has the identifier 3000000000 is read, a probe is added and the result is read again
- **THEN** the entries after the probe are byte for byte as before

### Requirement: Shared fonts
With one or more `--font FILE` options the harness SHALL make both runtimes draw text with
those files: it SHALL install each file into the Wine prefix of the run, register replacements
from the common game font names to the family name read from the first file, and pass vkxp a
font directory holding the same files. A file that does not exist or is not a font SHALL end
the run with an error naming it. Without the option the harness SHALL print that text is not
comparable.

#### Scenario: Family name
- **WHEN** a font file is given whose family name is `Test Sans`
- **THEN** the registry script prepared for the prefix maps `SimHei` and `黑体` to `Test Sans` and the vkxp command line names a font directory that holds a copy of the file

#### Scenario: Not a font
- **WHEN** `--font` names a text file
- **THEN** the run ends with an error naming the file before either runtime starts

#### Scenario: No font given
- **WHEN** the harness runs without `--font`
- **THEN** its output has a line saying that text is not comparable
