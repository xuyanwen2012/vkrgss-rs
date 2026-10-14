## ADDED Requirements

### Requirement: Archives of RGSSAD version 3
The harness SHALL read and write `Game.rgss3a`, RGSSAD version 3, keeping the seed, the order
of the entries, the key of every entry and the end record of the table, so that an archive
whose data follow the table in table order is written back byte for byte. It SHALL refuse a
file that is not a complete archive of version 3.

#### Scenario: Round trip
- **WHEN** an archive with seed 0x2a38 and three entries with the keys 41, 18467 and 6334 is written and read back
- **THEN** the seed, the names, the keys and the data of the three entries are as written, in their order

#### Scenario: Byte for byte
- **WHEN** an archive whose end record holds twelve arbitrary bytes after its first word is read and written again
- **THEN** the result equals the archive byte for byte

#### Scenario: Format
- **WHEN** an archive with seed S and one entry is written
- **THEN** its first eight bytes are `RGSSAD\0\3`, the next four are S in little endian, and the first four words of the table XORed with S * 9 + 3 give the offset of the data, their size, the entry's key and the length of its name

#### Scenario: Not an archive of version 3
- **WHEN** an archive of version 1, or an archive of version 3 cut off inside its table, is read
- **THEN** the reader returns an error

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
- **WHEN** such a game without a file at that path is run without `--dll` and without `--vkxp-only`
- **THEN** the run ends with a non-zero status and a message that names `System\RGSS300.dll` and `--dll`, and no Wine process is started

#### Scenario: Path outside the game
- **WHEN** the game's `Game.ini` says `Library=..\RGSS300.dll`
- **THEN** the run ends with a non-zero status and a message that names the path

### Requirement: The vkxp side of a game of RGSS3
For a game of RGSS3 the harness SHALL build the guest executable of RGSS3 together with
vkxp, so that vkxp finds it beside itself, and SHALL run vkxp on the copy as for the other
runtimes.

#### Scenario: Capture in vkxp of RGSS3
- **WHEN** frames 10 and 20 of a generated game of RGSS3 are asked for with `--vkxp-only`
- **THEN** two 544 by 416 pictures are written for vkxp, and they equal the dumps of a plain headless run of the same game at those frames

### Requirement: Probe of RGSS3
The probe of RGSS3 SHALL be one text for both sides that differs only in the value naming the
side, SHALL consist of ASCII characters only, and SHALL press no key: like the other probes it
answers `Input` from its model of the schedule on both sides.

#### Scenario: One text
- **WHEN** the probe of RGSS3 is generated for both sides with the same schedule and frames
- **THEN** the two texts differ only in the line that names the side, and every byte of both is below 128

## MODIFIED Requirements

### Requirement: Runtime of the game
The harness SHALL treat a game as RGSS1, RGSS2 or RGSS3 by the extension of the file its
`Scripts=` names (`.rxdata`, `.rvdata`, `.rvdata2`), or by `--runtime rgss1|rgss2|rgss3` when
given, and SHALL end with a message naming the option when neither decides. The runtime SHALL
give the frame size at the start (640 by 480, 544 by 416, 544 by 416), the registry key and
names of the run-time packages of the original, the name of the game's archive and the probe.

#### Scenario: VX game
- **WHEN** a generated game with `Scripts=Data\Scripts.rvdata` and `RTP=RPGVX` is prepared for a run with `--rtp RPGVX=<directory>`
- **THEN** the registry script for the prefix sets the value `RPGVX` under `Software\Enterbrain\RGSS2\RTP` to that directory, and the probe added to the copy is that of RGSS2

#### Scenario: VX Ace game
- **WHEN** a generated game with `Scripts=Data\Scripts.rvdata2` and `RTP=RPGVXAce` is prepared for a run with `--rtp RPGVXAce=<directory>`
- **THEN** the registry script for the prefix sets the value `RPGVXAce` under `Software\Enterbrain\RGSS3\RTP` to that directory, and the probe added to the copy is that of RGSS3

#### Scenario: XP game
- **WHEN** a generated game with `Scripts=Data\Scripts.rxdata` is prepared
- **THEN** everything prepared is as before this requirement existed

#### Scenario: Unknown extension
- **WHEN** the game's `Scripts=` names `Data\Code.bin` and no option is given
- **THEN** the run ends with a non-zero status and the message names `--runtime`

### Requirement: Same conditions on both sides
The probe SHALL seed the random number generator with a fixed value and SHALL answer
`Input.press?`, `Input.trigger?`, `Input.repeat?`, `Input.dir4` and `Input.dir8` from the
input schedule given to the harness, by frame number, with the trigger and repeat rules of the
original of the game's runtime, so that both runtimes see the same input at the same frame.
In RGSS3 a button SHALL be taken as its Symbol or its Integer of RGSS2, anything else as false.

#### Scenario: Scheduled input
- **WHEN** the schedule holds RIGHT from frame 60 to frame 100 and the probe runs in vkxp
- **THEN** `Input.press?(Input::RIGHT)` is true exactly in the frames 60 to 99, and `Input.trigger?(Input::RIGHT)` only in frame 60

#### Scenario: Repeat of RGSS2, measured on the original
- **WHEN** the probe of RGSS2 is given a schedule that holds DOWN from frame 0 on
- **THEN** its model answers `Input.repeat?(Input::DOWN)` with true in the 1st, 24th, 30th and 36th frame of the hold and in none between

#### Scenario: Buttons of RGSS3
- **WHEN** the probe of RGSS3 runs in vkxp with a schedule that holds RIGHT from frame 60 to frame 100, and the game asks in frame 70
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
- **THEN** two 640 by 480 pictures are written for vkxp, and they equal the dumps of a plain headless run of the same game at those frames

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
- **WHEN** the probe of RGSS3 of the original's side runs in vkxp with frames 2 and 5 asked for, the game calls `Graphics.resize_screen(640, 480)` after frame 2, and the test answers the requests in place of the harness
- **THEN** the request of frame 2 gives the size 544 by 416 and that of frame 5 the size 640 by 480

#### Scenario: Size from the request
- **WHEN** the harness reads the request `5 640x480` of a game of RGSS3, and the request `5` of a game of RGSS2
- **THEN** it looks for a window of 640 by 480 for the first and of 544 by 416 for the second

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
- **WHEN** the harness runs a packed generated game with `--vkxp-only` and asks for frames 10 and 20
- **THEN** the two pictures equal those of the same game run unpacked

#### Scenario: Archive of VX
- **WHEN** a generated game of RGSS2 whose `Data` folder exists only inside `Game.rgss2a` is prepared
- **THEN** the copy's `Game.rgss2a` holds the probed `Data\Scripts.rvdata` and every other entry is unchanged

#### Scenario: Archive of VX Ace
- **WHEN** a generated game of RGSS3 whose `Data` folder exists only inside `Game.rgss3a` is prepared
- **THEN** the copy's `Game.rgss3a` has the seed of the game's archive, every entry keeps its name, key and place, `Data\Scripts.rvdata2` holds the probed script archive, every other entry's data is unchanged, and the copy has no loose `Data` folder

#### Scenario: Packed game of RGSS3 runs in vkxp
- **WHEN** the harness runs a generated game of RGSS3 packed into `Game.rgss3a` with `--vkxp-only` and asks for frames 10 and 20
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
- **WHEN** the probe of RGSS3 of the original's side runs in vkxp a game that shows frame 0 by `update`, frames 1 to 4 by `wait(4)`, 5 to 8 by `fadeout(4)`, then `freeze`, frames 9 to 18 by `transition(10)` and updates from frame 19 on, frames 2, 7, 12, 18 and 20 are asked for, and the test answers the requests
- **THEN** only the request of frame 12 is marked late

### Requirement: The original side has no GPU device and no network
Every Wine process that the oracle starts SHALL run inside a `bwrap` sandbox in which
`/dev/dri` is an empty directory and the network is unshared, with VA-API variables that name
no driver and GStreamer plugin directories below the output directory without the plugins `va`, `vaapi`, `qsv`, `msdk`, `vulkan`, `kms`, `opengl`,
`qmlgl`, `nvcodec`, `video4linux2` and `v4l2codecs`. The oracle SHALL NOT start a Wine process
outside such a sandbox.

#### Scenario: The command that is built
- **WHEN** the oracle builds the command of any of its Wine programs
- **THEN** the program of the command is `bwrap`, its arguments hold `--tmpfs /dev/dri`, `--unshare-net` and `--die-with-parent` before the Wine program, and they do not hold `--unshare-pid`

#### Scenario: VA-API
- **WHEN** the environment of such a command is read
- **THEN** `LIBVA_DRIVERS_PATH` names a directory that does not exist and `LIBVA_DRIVER_NAME` is set

#### Scenario: GStreamer
- **WHEN** the plugin directories are made from a stand-in system directory that holds `libgstogg.so`, `libgstva.so`, `libgstvaapi.so`, `libgstvulkan.so` and `libgstmsdk.so`, and a Wine command is built
- **THEN** the directories hold `libgstogg.so` and no other of those files, `GST_PLUGIN_SYSTEM_PATH_1_0` names only them, `GST_REGISTRY_1_0` names a file below the output directory, and `GST_PLUGIN_PATH` and `GST_PLUGIN_PATH_1_0` are removed

#### Scenario: bwrap is missing
- **WHEN** the original side is asked for and `bwrap` is not on `PATH`
- **THEN** the command ends with an error that names `bwrap`, before any Wine process starts
