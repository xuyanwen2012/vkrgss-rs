## ADDED Requirements

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

## MODIFIED Requirements

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
