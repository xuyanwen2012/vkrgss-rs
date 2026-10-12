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
original of the game's runtime, so that both runtimes see the same input at the same frame.

#### Scenario: Scheduled input
- **WHEN** the schedule holds RIGHT from frame 60 to frame 100 and the probe runs in vkxp
- **THEN** `Input.press?(Input::RIGHT)` is true exactly in the frames 60 to 99, and `Input.trigger?(Input::RIGHT)` only in frame 60

#### Scenario: Repeat of RGSS2, measured on the original
- **WHEN** the probe of RGSS2 is given a schedule that holds DOWN from frame 0 on
- **THEN** its model answers `Input.repeat?(Input::DOWN)` with true in the 1st, 24th, 30th and 36th frame of the hold and in none between

### Requirement: Frame capture
For each frame number asked for, the harness SHALL obtain the picture each runtime shows after
that many frames, in the frame size of the game's runtime, counted the same way on both sides:
as the original of that runtime advances its frame count. On the original side the probe SHALL
hold the game at that frame until the harness has captured the game window from the virtual
display.

#### Scenario: Capture in vkxp
- **WHEN** frames 10 and 20 are asked for on a small generated game
- **THEN** two 640 by 480 pictures are written for vkxp, and they equal the dumps of a plain headless run of the same game at those frames

#### Scenario: Capture in the original
- **WHEN** the original runtime, Wine and Xvfb are available and frames 10 and 20 are asked for
- **THEN** two 640 by 480 pictures of the original's window are written

#### Scenario: Frame numbers of RGSS2
- **WHEN** the frame counter of the probe of RGSS2 is advanced for `wait(5)`, `fadeout(3)`, a `transition(20)` without a freeze, a `freeze` and a `transition(20)` after it
- **THEN** it has advanced by 5, 3, 0, 0 and 20

#### Scenario: Capture in the original of VX
- **WHEN** the original of VX, Wine and Xvfb are available and a frame inside a `Graphics.wait` of a VX game is asked for
- **THEN** a 544 by 416 picture of the original's window at that frame is written

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

### Requirement: Own metrics with shared fonts
When fonts are shared with `--font`, the harness SHALL run vkxp with fonts' own metrics, so
that both runtimes size the shared font by its own line box.

#### Scenario: Command line
- **WHEN** the harness builds vkxp's command line for a run with `--font`
- **THEN** it contains `--font-own-metrics`, and without `--font` it does not

### Requirement: Games with an encrypted archive
When the game directory holds the archive of the game's runtime (`Game.rgssad`, `Game.rgss2a`),
the harness SHALL add its probe to the script archive stored inside it and write the changed
archive into each copy of the game, leaving every other entry byte for byte as it was and
adding no loose script archive. The given game directory SHALL stay untouched.

#### Scenario: Probe inside the archive
- **WHEN** a generated game whose `Data` folder exists only inside `Game.rgssad` is prepared for a run
- **THEN** the copy's archive holds the probed script archive, every other entry is unchanged, and the copy has no loose `Data` folder

#### Scenario: Packed game runs in vkxp
- **WHEN** the harness runs a packed generated game with `--vkxp-only` and asks for frames 10 and 20
- **THEN** the two pictures equal those of the same game run unpacked

#### Scenario: Archive of VX
- **WHEN** a generated game of RGSS2 whose `Data` folder exists only inside `Game.rgss2a` is prepared
- **THEN** the copy's `Game.rgss2a` holds the probed `Data\Scripts.rvdata` and every other entry is unchanged

### Requirement: Packing a game copy
`cargo xtask pack-rgssad <directory>` SHALL write a `Game.rgssad` of version 1 holding every
file below the directory's `Data` and `Graphics` folders under its path with backslashes, and
SHALL then remove those two folders. It SHALL refuse a directory that already has an archive
or has no `Game.ini`.

#### Scenario: Round trip
- **WHEN** a generated game is packed and its archive is read back
- **THEN** every file of the two folders is found under its backslash path with its bytes, and the folders are gone

#### Scenario: Already packed
- **WHEN** the directory already holds `Game.rgssad`
- **THEN** the command exits with a non-zero status and changes nothing

### Requirement: Runtime of the game
The harness SHALL treat a game as RGSS1 or RGSS2 by the extension of the file its `Scripts=`
names (`.rxdata`, `.rvdata`), or by `--runtime rgss1|rgss2` when given, and SHALL end with a
message naming the option when neither decides. The runtime SHALL give the frame size (640 by
480, 544 by 416), the registry key and names of the run-time packages of the original, the
name of the game's archive and the probe.

#### Scenario: VX game
- **WHEN** a generated game with `Scripts=Data\Scripts.rvdata` and `RTP=RPGVX` is prepared for a run with `--rtp RPGVX=<directory>`
- **THEN** the registry script for the prefix sets the value `RPGVX` under `Software\Enterbrain\RGSS2\RTP` to that directory, and the probe added to the copy is that of RGSS2

#### Scenario: XP game
- **WHEN** a generated game with `Scripts=Data\Scripts.rxdata` is prepared
- **THEN** everything prepared is as before this requirement existed

#### Scenario: Unknown extension
- **WHEN** the game's `Scripts=` names `Data\Code.bin` and no option is given
- **THEN** the run ends with a non-zero status and the message names `--runtime`

### Requirement: Frames inside a transition
On the original of RGSS2 a frame inside a transition cannot be held. When a frame asked for
falls inside one, the harness SHALL keep the picture the original shows after the transition,
SHALL list the frame in the report as not comparable, and SHALL count it neither as equal nor
as different.

#### Scenario: Late frame
- **WHEN** the probe's record of a run says that frame 10 was captured late, inside a transition
- **THEN** the report lists frame 10 as not comparable and its totals do not include it

### Requirement: Original alone
With `--original-only` the harness SHALL run only the original and SHALL write its pictures
and a report that lists them without comparison. The option SHALL NOT be accepted together
with `--vkxp-only`.

#### Scenario: Both options
- **WHEN** the harness is run with `--original-only` and `--vkxp-only`
- **THEN** it ends with a usage message and a non-zero status

#### Scenario: Repeated runs
- **WHEN** the original, Wine and Xvfb are available and a VX game is run twice with `--original-only`, the same schedule and the same frames
- **THEN** the pictures of the two runs are equal byte for byte

### Requirement: Games that start threads
With `--no-threads` the probe SHALL, on both sides, run the block of `Thread.new` at once in
the caller and make `sleep` inside such a block return at once, and the report SHALL say that
the option was used. Without the option the probe SHALL NOT change threads.

#### Scenario: Block runs at once
- **WHEN** the probe with the option is followed by a script that evaluates `x = 0; Thread.new { sleep(5); x = 1 }; x`
- **THEN** the result is 1 and no five seconds pass

#### Scenario: Without the option
- **WHEN** the probe is generated without the option
- **THEN** its text does not redefine `Thread`
