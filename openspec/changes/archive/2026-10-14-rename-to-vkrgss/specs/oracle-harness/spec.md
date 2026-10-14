## RENAMED Requirements

- FROM: `### Requirement: The vkxp side of a game of RGSS3`
- TO: `### Requirement: The vkrgss side of a game of RGSS3`

## MODIFIED Requirements

### Requirement: Probe injection
The harness SHALL work on a copy of the game directory and SHALL add one probe script as the
first entry of the copy's script archive, leaving the other entries and the original directory
unchanged. An archive written by the harness SHALL be readable by the original runtime and by
vkrgss.

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
In RGSS3 a button SHALL be taken as its Symbol or its Integer of RGSS2, anything else as false.

#### Scenario: Scheduled input
- **WHEN** the schedule holds RIGHT from frame 60 to frame 100 and the probe runs in vkrgss
- **THEN** `Input.press?(Input::RIGHT)` is true exactly in the frames 60 to 99, and `Input.trigger?(Input::RIGHT)` only in frame 60

#### Scenario: Repeat of RGSS2, measured on the original
- **WHEN** the probe of RGSS2 is given a schedule that holds DOWN from frame 0 on
- **THEN** its model answers `Input.repeat?(Input::DOWN)` with true in the 1st, 24th, 30th and 36th frame of the hold and in none between

#### Scenario: Buttons of RGSS3
- **WHEN** the probe of RGSS3 runs in vkrgss with a schedule that holds RIGHT from frame 60 to frame 100, and the game asks in frame 70
- **THEN** `Input.press?(:RIGHT)` and `Input.press?(6)` are true, and `Input.press?(:right)`, `Input.press?("RIGHT")`, `Input.press?(nil)` and `Input.press?(:LEFT)` are false

#### Scenario: Repeat of RGSS3, measured on the original
- **WHEN** the probe of RGSS3 is given a schedule that holds DOWN from frame 0 on
- **THEN** its model answers `Input.repeat?(:DOWN)` with true in the 1st, 24th, 30th and 36th frame of the hold and in none between

### Requirement: Frame capture
For each frame number asked for, the harness SHALL obtain the picture each runtime shows after
that many frames, at the size of the game's screen at that frame, counted the same way on both
sides: as the original of that runtime advances its frame count. On the original side the
probe SHALL hold the game at that frame until the harness has captured the game window of
that size from the virtual display.

#### Scenario: Capture in vkxp
- **WHEN** frames 10 and 20 are asked for on a small generated game
- **THEN** two 640 by 480 pictures are written for vkrgss, and they equal the dumps of a plain headless run of the same game at those frames

#### Scenario: Capture in the original
- **WHEN** the original runtime, Wine and Xvfb are available and frames 10 and 20 are asked for
- **THEN** two 640 by 480 pictures of the original's window are written

#### Scenario: Frame numbers of RGSS2
- **WHEN** the frame counter of the probe of RGSS2 is advanced for `wait(5)`, `fadeout(3)`, a `transition(20)` without a freeze, a `freeze` and a `transition(20)` after it
- **THEN** it has advanced by 5, 3, 0, 0 and 20

#### Scenario: Frame numbers of RGSS3
- **WHEN** the frame counter of the probe of RGSS3 is advanced for `wait(5)`, `fadeout(3)`, a `transition(20)` without a freeze, a `freeze` and a `transition(20)` after it
- **THEN** it has advanced by 5, 3, 0, 0 and 20

#### Scenario: Capture in the original of VX
- **WHEN** the original of VX, Wine and Xvfb are available and a frame inside a `Graphics.wait` of a VX game is asked for
- **THEN** a 544 by 416 picture of the original's window at that frame is written

#### Scenario: Screen resized in RGSS3
- **WHEN** the probe of RGSS3 of the original's side runs in vkrgss with frames 2 and 5 asked for, the game calls `Graphics.resize_screen(640, 480)` after frame 2, and the test answers the requests in place of the harness
- **THEN** the request of frame 2 gives the size 544 by 416 and that of frame 5 the size 640 by 480

#### Scenario: Size from the request
- **WHEN** the harness reads the request `5 640x480` of a game of RGSS3, and the request `5` of a game of RGSS2
- **THEN** it looks for a window of 640 by 480 for the first and of 544 by 416 for the second

### Requirement: Missing prerequisites
When Wine, Xvfb, the DLL or the game directory is missing, the harness SHALL end with an error
that names what is missing, and SHALL be able to run the vkrgss side alone when asked to.

#### Scenario: No DLL
- **WHEN** the harness is run with a DLL path that does not exist
- **THEN** it exits with a non-zero status and names the path

### Requirement: Shared fonts
With one or more `--font FILE` options the harness SHALL make both runtimes draw text with
those files: it SHALL install each file into the Wine prefix of the run, register replacements
from the common game font names to the family name read from the first file, and pass vkrgss a
font directory holding the same files. A file that does not exist or is not a font SHALL end
the run with an error naming it. Without the option the harness SHALL print that text is not
comparable.

#### Scenario: Family name
- **WHEN** a font file is given whose family name is `Test Sans`
- **THEN** the registry script prepared for the prefix maps `SimHei` and `黑体` to `Test Sans` and the vkrgss command line names a font directory that holds a copy of the file

#### Scenario: Not a font
- **WHEN** `--font` names a text file
- **THEN** the run ends with an error naming the file before either runtime starts

#### Scenario: No font given
- **WHEN** the harness runs without `--font`
- **THEN** its output has a line saying that text is not comparable

### Requirement: Own metrics with shared fonts
When fonts are shared with `--font`, the harness SHALL run vkrgss with fonts' own metrics, so
that both runtimes size the shared font by its own line box.

#### Scenario: Command line
- **WHEN** the harness builds vkrgss's command line for a run with `--font`
- **THEN** it contains `--font-own-metrics`, and without `--font` it does not

### Requirement: Games with an encrypted archive
When the game directory holds the archive of the game's runtime (`Game.rgssad`, `Game.rgss2a`,
`Game.rgss3a`), the harness SHALL add its probe to the script archive stored inside it and
write the changed archive into each copy of the game, leaving every other entry byte for byte
as it was, keeping the seed and the keys of an archive of version 3, and adding no loose
script archive. The given game directory SHALL stay untouched.

#### Scenario: Probe inside the archive
- **WHEN** a generated game whose `Data` folder exists only inside `Game.rgssad` is prepared for a run
- **THEN** the copy's archive holds the probed script archive, every other entry is unchanged, and the copy has no loose `Data` folder

#### Scenario: Packed game runs in vkxp
- **WHEN** the harness runs a packed generated game with `--vkrgss-only` and asks for frames 10 and 20
- **THEN** the two pictures equal those of the same game run unpacked

#### Scenario: Archive of VX
- **WHEN** a generated game of RGSS2 whose `Data` folder exists only inside `Game.rgss2a` is prepared
- **THEN** the copy's `Game.rgss2a` holds the probed `Data\Scripts.rvdata` and every other entry is unchanged

#### Scenario: Archive of VX Ace
- **WHEN** a generated game of RGSS3 whose `Data` folder exists only inside `Game.rgss3a` is prepared
- **THEN** the copy's `Game.rgss3a` has the seed of the game's archive, every entry keeps its name, key and place, `Data\Scripts.rvdata2` holds the probed script archive, every other entry's data is unchanged, and the copy has no loose `Data` folder

#### Scenario: Packed game of RGSS3 runs in vkxp
- **WHEN** the harness runs a generated game of RGSS3 packed into `Game.rgss3a` with `--vkrgss-only` and asks for frames 10 and 20
- **THEN** the two pictures equal those of the same game run unpacked

### Requirement: Frames inside a transition
On the original of RGSS2 and of RGSS3 a frame inside a transition cannot be held. When a frame
asked for falls inside one, the harness SHALL keep the picture the original shows after the
transition, SHALL list the frame in the report as not comparable, and SHALL count it neither
as equal nor as different.

#### Scenario: Late frame
- **WHEN** the probe's record of a run says that frame 10 was captured late, inside a transition
- **THEN** the report lists frame 10 as not comparable and its totals do not include it

#### Scenario: Late frame of RGSS3
- **WHEN** the probe of RGSS3 of the original's side runs in vkrgss a game that shows frame 0 by `update`, frames 1 to 4 by `wait(4)`, 5 to 8 by `fadeout(4)`, then `freeze`, frames 9 to 18 by `transition(10)` and updates from frame 19 on, frames 2, 7, 12, 18 and 20 are asked for, and the test answers the requests
- **THEN** only the request of frame 12 is marked late

### Requirement: Original alone
With `--original-only` the harness SHALL run only the original and SHALL write its pictures
and a report that lists them without comparison. The option SHALL NOT be accepted together
with `--vkrgss-only`.

#### Scenario: Both options
- **WHEN** the harness is run with `--original-only` and `--vkrgss-only`
- **THEN** it ends with a usage message and a non-zero status

#### Scenario: Repeated runs
- **WHEN** the original, Wine and Xvfb are available and a VX game is run twice with `--original-only`, the same schedule and the same frames
- **THEN** the pictures of the two runs are equal byte for byte

### Requirement: The DLL of a game of RGSS3
For a game of RGSS3 the harness SHALL put the DLL at the path that `Library=` of `Game.ini`
names in the copy of the original's side, and nowhere else; without `--dll` it SHALL use the
file the game has at that path. A missing file, a missing `Library=` line or a path that
leaves the game directory SHALL end the run with an error naming it, before Wine starts.

#### Scenario: DLL given
- **WHEN** a generated game of RGSS3 with `Library=System\RGSS300.dll` and a folder `system` is prepared for the original with `--dll` naming a file
- **THEN** the copy's `system/RGSS300.dll` holds the bytes of that file and the folder of `Game.exe` holds no DLL that the game did not have

#### Scenario: DLL of the game
- **WHEN** such a game that has its own `System/RGSS300.dll` is prepared without `--dll`
- **THEN** the check made before Wine starts passes, and the copy holds that file at that path unchanged

#### Scenario: No DLL
- **WHEN** such a game without a file at that path is run without `--dll` and without `--vkrgss-only`
- **THEN** the run ends with a non-zero status and a message that names `System\RGSS300.dll` and `--dll`, and no Wine process is started

#### Scenario: Path outside the game
- **WHEN** the game's `Game.ini` says `Library=..\RGSS300.dll`
- **THEN** the run ends with a non-zero status and a message that names the path

### Requirement: The vkrgss side of a game of RGSS3
For a game of RGSS3 the harness SHALL build the guest executable of RGSS3 together with
vkrgss, so that vkrgss finds it beside itself, and SHALL run vkrgss on the copy as for the other
runtimes.

#### Scenario: Capture in vkxp of RGSS3
- **WHEN** frames 10 and 20 of a generated game of RGSS3 are asked for with `--vkrgss-only`
- **THEN** two 544 by 416 pictures are written for vkrgss, and they equal the dumps of a plain headless run of the same game at those frames
