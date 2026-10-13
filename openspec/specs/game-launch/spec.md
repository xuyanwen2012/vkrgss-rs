# game-launch Specification

## Purpose
Defines how a game is started with the `vkxp` program: its command line, the windowed and
headless modes, how it ends, the fonts it draws text with, and the tool that provides the
built-in scripts a game needs.

## Requirements

### Requirement: Starting a game
`vkxp <game directory>` SHALL run the game in that directory: with the directory as the working
directory of the scripts, the host on the main thread and the scripts on the interpreter
thread, connected by the protocol link. Without a directory argument the current directory
SHALL be used. A directory without a game configuration file SHALL end the program with an
error naming what is missing, before any window is opened.

#### Scenario: Game ends by itself
- **WHEN** a game whose scripts draw three frames and return is run headless
- **THEN** the program exits with status 0

#### Scenario: Not a game directory
- **WHEN** the program is given a directory without `Game.ini`
- **THEN** it prints an error naming `Game.ini` and exits with a non-zero status

#### Scenario: Relative files
- **WHEN** a script writes a file with a relative name
- **THEN** the file appears in the game directory

### Requirement: Exit status
The program SHALL exit with status 0 when the game ends normally, including by `exit`, by the
user closing the window and by the frame limit of a headless run, and with a non-zero status
when a script error ends it, when the host fails, or when a frame is rejected in a headless
run. A script error's report SHALL be on the error stream.

#### Scenario: Script error
- **WHEN** a game script raises an exception nobody rescues
- **THEN** the report naming the script and line is on the error stream and the exit status is non-zero

#### Scenario: Frame limit
- **WHEN** a game that never ends is run with `--headless --frames 10`
- **THEN** the program exits with status 0 after ten frames

### Requirement: Options
The program SHALL accept options that name: a directory for each RTP name
(`--rtp NAME=DIRECTORY`, repeatable), the built-in scripts file, the GPU by index or name, font
directories (repeatable), the text encoding of the game configuration file, debug mode, and
enabling of process functions. An unknown option or a malformed value SHALL end the program
with a usage message and a non-zero status.

#### Scenario: RTP directory
- **WHEN** a game that names the RTP `Standard` and loads a picture found only there is run with `--rtp Standard=<directory>`
- **THEN** the picture loads

#### Scenario: RTP not given
- **WHEN** that game is run without the option
- **THEN** the program ends with an error naming `Standard` and the option to pass

#### Scenario: Unknown option
- **WHEN** the program is run with `--no-such-option`
- **THEN** it prints a usage message and exits with a non-zero status

### Requirement: Headless mode
With `--headless` the program SHALL run without a window on a virtual clock, SHALL stop after
the number of frames given by `--frames` if the game has not ended, SHALL take input from a
script given by `--input`, and SHALL write the frames named by `--dump FRAME=FILE` as PNG files.
Two runs with the same options SHALL write identical files.

#### Scenario: Frame dump
- **WHEN** a game that shows a red sprite at 10, 20 is run with `--headless --frames 5 --dump 3=out.png`
- **THEN** `out.png` is 640 by 480 and its pixel at (10, 20) is red

#### Scenario: Scripted input
- **WHEN** a game that moves a sprite right while RIGHT is held is run headless with an input script holding RIGHT from frame 2 to frame 6
- **THEN** the sprite in the dump of frame 8 is further right than in the dump of frame 1

#### Scenario: Repeatable
- **WHEN** the same headless run is made twice
- **THEN** the dumped files are byte-identical

### Requirement: Fonts
Text SHALL be drawn with the font engine, using the fonts in the directories named with
`--font-dir`, then those in the game's `Fonts` directory when it exists, then the fonts
installed on the system. `Font.exist?` SHALL answer from the same engine.

#### Scenario: Font from a directory
- **WHEN** a game draws text in a family found only in a directory given with `--font-dir`, in a headless run
- **THEN** the dumped frame has ink where the text was drawn, and `Font.exist?` of that family is `true` in the game's scripts

#### Scenario: Unknown family
- **WHEN** a script asks `Font.exist?("NoSuchFamilyAnywhere")`
- **THEN** the answer is `false`

### Requirement: Built-in scripts file
The program SHALL load the built-in scripts of the run's runtime from the file given by its
option, or from a default location under the user's data directory: `rpg_builtin.rb` for
RGSS1, `rpg_builtin_rgss2.rb` for RGSS2 and `rpg_builtin_rgss3.rb` for RGSS3. When neither
exists it SHALL end with an error that names the file and how it is made. The repository
SHALL NOT contain such scripts. In a run of RGSS3 the file SHALL be found and not evaluated.

#### Scenario: Missing file
- **WHEN** the program is run without the option and no file exists at the default location
- **THEN** it exits with a non-zero status and the message names `cargo xtask extract-rpg`

#### Scenario: Missing file of VX
- **WHEN** a game of RGSS2 is run without the option and no `rpg_builtin_rgss2.rb` exists at the default location
- **THEN** it exits with a non-zero status and the message names that file and `docs/running.md`

#### Scenario: Missing file of VX Ace
- **WHEN** a generated game of RGSS3 is run headless without the option and no `rpg_builtin_rgss3.rb` exists at the default location
- **THEN** it exits with a non-zero status, the message names that file and `docs/running.md`, and the error stream does not say that VX Ace is planned

#### Scenario: Default location of VX Ace
- **WHEN** the same game is run without the option and a file `rpg_builtin_rgss3.rb` exists in the directory `vkxp` of the user's data directory
- **THEN** no message about a built-in scripts file is printed and the run ends with the message that VX Ace is planned and not supported

### Requirement: Extraction tool
`cargo xtask extract-rpg <DLL> <OUTPUT>` SHALL find the built-in Ruby script blocks stored as
plain text in the file it is given, an RGSS DLL or its image in the memory of the running
original, in their order, and write them to the output file as Ruby source. A block SHALL be a NUL-terminated string of text that starts with `module RPG;`,
`def rgss_main` or `def rgss_stop`. It SHALL fail with a clear message when the file contains
no such block, as a DLL still compressed by its protector does.

#### Scenario: Blocks found
- **WHEN** the tool is run on a file that contains two NUL-terminated strings starting with `module RPG;`
- **THEN** the output file holds both, the first before the second

#### Scenario: Nothing found
- **WHEN** the tool is run on a file without such a string
- **THEN** it exits with a non-zero status and writes no output file

#### Scenario: Image of RGSS3
- **WHEN** the tool is run on a generated file that holds, between binary data and in this order, NUL-terminated strings that start with `module RPG;end;`, `def rgss_main`, `Encoding.default_external`, `def rgss_stop` and `$RGSS_SCRIPTS = load_data`
- **THEN** the output file holds the first, the second and the fourth string in that order, each followed by a line break, and nothing of the other two

#### Scenario: Function text that is not a string of its own
- **WHEN** the file holds `def rgss_main` followed by text that a byte other than NUL ends
- **THEN** that text is not written

### Requirement: Audio options
A windowed run SHALL play audio through the default output device. `--no-audio` SHALL disable
it. `--soundfont FILE` SHALL name the SoundFont for MIDI; without it the program SHALL look in
the usual system locations and, finding none, play MIDI as silence with one report.

#### Scenario: Disabled
- **WHEN** the program is run with `--no-audio`
- **THEN** no audio device is opened

#### Scenario: SoundFont missing
- **WHEN** `--soundfont` names a file that does not exist
- **THEN** the program ends with an error naming the file before any window is opened

### Requirement: Seed, preload and replay options
The program SHALL accept `--seed N`, `--preload FILE` (repeatable), `--record FILE` and
`--replay FILE`. Without `--seed` a headless run SHALL use seed 0 and a windowed run a seed
taken from the clock; a replay SHALL use the seed stored in the recording unless `--seed` is
given. `--record` together with `--replay` SHALL be refused with the usage message.

#### Scenario: Headless default
- **WHEN** a game that draws a sprite at a `rand` position is run headless twice without `--seed`
- **THEN** the dumped frames are byte-identical

#### Scenario: Record and replay together
- **WHEN** both `--record` and `--replay` are given
- **THEN** the program prints the usage message and exits with a non-zero status

### Requirement: Encrypted archive of the game
The program SHALL read a game whose data is in the archive of its runtime, `Game.rgssad`,
`Game.rgss2a` or `Game.rgss3a`, and SHALL end with an error naming the archive when it is
damaged.

#### Scenario: Game in an archive
- **WHEN** a generated game whose `Data` folder exists only inside `Game.rgssad` is run headless
- **THEN** it runs like the same game with loose files

#### Scenario: Game of VX in an archive
- **WHEN** a generated game of RGSS2 whose `Data` folder exists only inside `Game.rgss2a` is run headless
- **THEN** it runs like the same game with loose files

#### Scenario: Game of VX Ace in an archive
- **WHEN** a generated game of RGSS3 whose `Data` folder exists only inside a generated `Game.rgss3a` of version 3 is run headless
- **THEN** the error stream names no missing file, and the run ends as the same game with loose files does: with the message that VX Ace is planned and not supported

#### Scenario: Damaged archive of VX Ace
- **WHEN** that game is run with its `Game.rgss3a` cut inside the entry table
- **THEN** the program exits with a non-zero status, the message names `Game.rgss3a`, and the error stream does not say that VX Ace is planned

### Requirement: Configuration files
The program SHALL read settings from a user configuration file and from `vkxp.toml` in the game
directory when they exist, with the game's file overriding the user's and command-line options
overriding both. Every setting that has a command-line option SHALL be settable in the files.
`--config FILE` SHALL name the user file and `--no-config` SHALL make the program read neither.

#### Scenario: RTP from the file
- **WHEN** the user file names the directory of the RTP `Standard` and a game that needs it is run without `--rtp`
- **THEN** the game finds its RTP files

#### Scenario: Precedence
- **WHEN** the user file sets seed 1, the game's file sets seed 2 and the command line gives no seed
- **THEN** the run uses seed 2, and with `--seed 3` it uses seed 3

#### Scenario: Ignored
- **WHEN** the program is run with `--no-config` and a user file with an error exists
- **THEN** the run starts normally

### Requirement: Configuration errors
A configuration file that does not parse, or that holds an unknown key or a value of the wrong
type, SHALL end the program with an error naming the file and the key before any window is
opened.

#### Scenario: Unknown key
- **WHEN** a configuration file holds a key the program does not know
- **THEN** the program exits with a non-zero status and names the file and the key

### Requirement: Window options
The program SHALL accept `--filter auto|nearest|linear|sharp` and `--fullscreen`, and the same
settings in the configuration files, and SHALL refuse another filter name with the usage
message.

#### Scenario: Unknown filter
- **WHEN** the program is run with `--filter cubic`
- **THEN** it prints the usage message and exits with a non-zero status

#### Scenario: From the file
- **WHEN** a configuration file sets the filter to `linear` and no option is given
- **THEN** the run uses linear, and `--filter nearest` overrides it

### Requirement: Own font metrics option
The program SHALL accept `--font-own-metrics`, and the configuration key `font_own_metrics`,
which make the game's text use fonts' own line boxes.

#### Scenario: Option
- **WHEN** a generated game draws text with a replaced font name and is run headless with and without `--font-own-metrics` and a font directory holding a test font
- **THEN** the two dumps differ in the text's size

### Requirement: Diagnostics
Diagnostics of the running program SHALL be events with a level, written to the error stream.
By default warnings and errors SHALL be shown, each as a line that starts with `vkxp: `. Text
the game produces and the program's usage and fatal error message SHALL be shown at every
level.

#### Scenario: Default level
- **WHEN** a game runs without `--verbose` and without `VKXP_LOG`
- **THEN** the error stream has no informational line

#### Scenario: Script error at every level
- **WHEN** a game whose script raises runs with `VKXP_LOG=error`
- **THEN** the error stream still carries the script error report

### Requirement: Diagnostic level
The option `--verbose` SHALL also show informational and debug events. The environment
variable `VKXP_LOG` SHALL set the level or a per-module filter and take precedence over
`--verbose`; a value that cannot be parsed SHALL end the program with a message and a non-zero
status.

#### Scenario: Verbose
- **WHEN** the same game runs headless with `--verbose`
- **THEN** the error stream names the built-in scripts file that was loaded

#### Scenario: Level from the environment
- **WHEN** the game runs with `VKXP_LOG=error` and `--verbose`
- **THEN** the error stream has no informational line

#### Scenario: Malformed filter
- **WHEN** the program runs with `VKXP_LOG` set to `=`
- **THEN** it ends with a message naming the variable and a non-zero status

### Requirement: Frame statistics option
The program SHALL accept `--frame-stats`, which requests the frame time statistics of the
host session, in windowed and in headless mode.

#### Scenario: Option accepted
- **WHEN** the program is run with `--frame-stats` in headless mode
- **THEN** it runs the game and writes the summary to the error stream

### Requirement: Guest in a process
`vkxp` SHALL run the guest either as a thread of its own process or in a child process. In a
child process the guest SHALL exchange with the host, over a socket that other users of the
machine cannot reach, the statement of its version at the start of each connection it
opens, the messages of the protocol, and nothing else. A run SHALL write the same frames in
both forms, and its exit status SHALL follow the same rules.

#### Scenario: Same frames
- **WHEN** a generated game that draws, moves a sprite with scripted input and makes a transition is run headless once with `--guest thread` and once with `--guest process`, dumping the same frames
- **THEN** both runs exit with status 0 and the dumped files of the two runs are byte for byte equal

#### Scenario: Script error
- **WHEN** a game whose script raises an error is run headless with `--guest process`
- **THEN** the error stream names the error as it does with `--guest thread` and the exit status is not 0

#### Scenario: Guest process dies
- **WHEN** the guest process of a headless run is killed by a signal while the game runs
- **THEN** `vkxp` ends with a status other than 0 and its error stream names the signal

#### Scenario: Frame limit
- **WHEN** a game that never ends is run headless with `--guest process --frames 5`
- **THEN** `vkxp` exits with status 0 and no guest process is left running

### Requirement: Guest option
The form SHALL be chosen by `--guest thread` or `--guest process`, by the environment variable
`VKXP_GUEST` and by the configuration key `guest`, the command line before the environment and
the environment before the configuration files. Without a choice the guest SHALL be a process
in a window and a thread in a headless run.

#### Scenario: Unknown value
- **WHEN** `vkxp` is started with `--guest fiber`
- **THEN** it exits with an error that names `thread` and `process`

#### Scenario: Environment
- **WHEN** `vkxp` is started with `VKXP_GUEST=process` and without `--guest`
- **THEN** the guest runs in a child process

#### Scenario: Headless default
- **WHEN** `vkxp` is started headless without a choice
- **THEN** the guest runs as a thread

### Requirement: States option and parked copies
`--states SCHEDULE` SHALL give a headless run its state requests, as `frame:save N` and
`frame:load N` entries separated by commas, and SHALL be an error with the guest as a thread.
States SHALL be kept as copies of the guest process, and when `vkxp` ends no process of the
run SHALL be left. The exit status SHALL follow the rules of a run without states.

#### Scenario: Thread form
- **WHEN** `vkxp` is started headless with `--states "5:save 0"` and without `--guest process`
- **THEN** it exits with an error that names `--guest process`

#### Scenario: Malformed schedule
- **WHEN** `--states "5:keep 0"` or a slot above 9 is given
- **THEN** `vkxp` exits with an error that names the entry

#### Scenario: Nothing left behind
- **WHEN** a headless run that saves into three slots, saves again into one of them and loads once ends
- **THEN** none of the processes that attached during the run exists afterwards

#### Scenario: Script error after a load
- **WHEN** the game raises a script error some frames after a state was loaded
- **THEN** the error is reported and the exit status is not 0

### Requirement: Socket directories of ended runs
Starting a guest process SHALL remove the socket directories that earlier runs of the same
user left behind and whose process no longer exists, and SHALL leave every other entry of
the directory they are in alone.

#### Scenario: Left by a killed run
- **WHEN** the base directory holds a socket directory named after a process that no longer exists, one named after a running process and a file of another name, and a run with `--guest process` starts
- **THEN** the first is gone and the other two are still there

### Requirement: Version option
`vkxp --version` and `vkxp -V` SHALL print `vkxp` and the version of the package on one line
to the standard output and exit with status 0, without opening a device or a game.

#### Scenario: Version
- **WHEN** `vkxp --version` is run with no Vulkan driver selectable and no game directory
- **THEN** the standard output is `vkxp 0.1.0` followed by a line break and the exit status is 0

#### Scenario: Listed in the usage
- **WHEN** `vkxp --help` is run
- **THEN** the text lists `--version`

### Requirement: Help option
`vkxp --help` and `vkxp -h` SHALL print the usage text to the standard output and exit with
status 0, without opening a device or a game. An unknown option SHALL keep ending with a
status other than 0 and the usage on the error stream.

#### Scenario: Help
- **WHEN** `vkxp --help` is run with no Vulkan driver selectable and no game directory
- **THEN** the standard output holds the usage text, which lists `--help` and `--version`, the error stream is empty and the exit status is 0

#### Scenario: Unknown option
- **WHEN** `vkxp --no-such-option` is run
- **THEN** the exit status is 2 and the error stream names the option

### Requirement: Confined guest process
A guest process SHALL restrict itself, before the first script runs, so that it can write,
create, remove and rename only below the game directory, can execute no file and, where the
kernel has the rule, can open no TCP connection; reading SHALL stay unrestricted. Where the
kernel cannot restrict, the run SHALL say so once and go on. The host process SHALL NOT be
restricted.

#### Scenario: Write inside
- **WHEN** a script of a game run with `--guest process` writes a file below the game directory
- **THEN** the file is written

#### Scenario: Write outside
- **WHEN** a script of that game opens a file for writing in a directory beside the game directory
- **THEN** `Errno::EACCES` is raised and no file is there

#### Scenario: States
- **WHEN** a headless run with `--guest process` and `--states "40:save 0, 80:load 0"` is made with and without the restriction
- **THEN** both runs dump the same frames

### Requirement: Sandbox option
`--sandbox on|off`, the environment variable `VKXP_SANDBOX` and the configuration key
`sandbox` SHALL choose whether a guest process is confined, with the command line before
the environment and the environment before the files; the default SHALL be on. With
`--allow-process-functions`, and for a guest on a thread, nothing SHALL be confined, and
the run SHALL say so once.

#### Scenario: Off
- **WHEN** the game of "Write outside" is run with `--sandbox off`
- **THEN** the file is written

#### Scenario: Unknown value
- **WHEN** the program is started with `--sandbox maybe`
- **THEN** it ends with an error that names the two values

### Requirement: Runtime of the run
The program SHALL take the runtime from `--runtime rgss1|rgss2|rgss3` when the option is given
and otherwise from the game configuration file. When the file does not decide, the program
SHALL end before any window is opened, with a non-zero status and a message that names the
values it read and the option.

#### Scenario: Detected
- **WHEN** a generated game with `Scripts=Data\Scripts.rxdata` is run headless without the option
- **THEN** it runs as before this requirement existed

#### Scenario: Conflict
- **WHEN** a game with `Library=RGSS104E.dll` and `Scripts=Data\Scripts.rvdata` is run without the option
- **THEN** the program exits with a non-zero status and the message names `--runtime`

#### Scenario: Option decides
- **WHEN** that game is run with `--runtime rgss2`
- **THEN** no such message is printed and the game is started as a game of RGSS2

#### Scenario: Bad value
- **WHEN** the program is run with `--runtime vx`
- **THEN** it prints a usage message and exits with a non-zero status

### Requirement: VX Ace is refused
A run whose runtime is RGSS3 SHALL end before a guest, an interpreter, a window or a device is
started, with a non-zero status and a message that names RPG Maker VX Ace and says that it is
planned and not supported. Before it the program SHALL, in this order, resolve the run-time package,
find the built-in scripts file, open the game's files with `Game.rgss3a` when there is one,
and read the file that `Scripts` names. A step that fails SHALL end the run with its own
error and without that message.

#### Scenario: Ace game
- **WHEN** a generated game with `Library=System\RGSS301.dll`, `Scripts=Data\Scripts.rvdata2`, an empty `RTP`, a loose script archive and a built-in scripts file named with the option is run headless
- **THEN** the program exits with a non-zero status, the message says VX Ace is not supported, and no script has run

#### Scenario: Library under the other name
- **WHEN** the same game names `Library=System\RGSS300.dll`
- **THEN** the run ends the same way

#### Scenario: Run-time package not given
- **WHEN** the game's configuration file has `RTP=RPGVXAce` and the run has no `--rtp` option
- **THEN** the program exits with a non-zero status, the message names `RPGVXAce` and `--rtp`, and the error stream does not say that VX Ace is planned

#### Scenario: Run-time package given
- **WHEN** that game is run with `--rtp RPGVXAce=<an existing directory>`
- **THEN** the run ends with the message that VX Ace is planned and not supported

#### Scenario: No script archive
- **WHEN** the game has neither a loose `Data/Scripts.rvdata2` nor a `Game.rgss3a` that holds it
- **THEN** the program exits with a non-zero status, the message names `Data\Scripts.rvdata2`, and the error stream does not say that VX Ace is planned

#### Scenario: Runtime named by the option
- **WHEN** a generated game of RGSS1 is run headless with `--runtime rgss3` and its built-in scripts file
- **THEN** the program exits with a non-zero status, the message says VX Ace is not supported, and no script has run

### Requirement: Frame size of the run
The program SHALL give the host the frame size of the run's runtime as the size the window
starts with, and a frame dumped by a headless run SHALL have the size of the game's frame.

#### Scenario: Dump of a game of VX
- **WHEN** a generated game of RGSS2 that calls `Graphics.update` runs headless with a dump of its first frame
- **THEN** the written picture is 544 by 416

#### Scenario: Dump of a game of XP
- **WHEN** a generated game of RGSS1 runs the same way
- **THEN** the written picture is 640 by 480

### Requirement: Supported runtimes say nothing about their state
A run of RGSS1 or of RGSS2 SHALL NOT say that support for its runtime is experimental, and
the usage message SHALL name `rgss1` and `rgss2` without such a word.

#### Scenario: VX
- **WHEN** a generated game of RGSS2 whose only script returns at once is run headless
- **THEN** the diagnostics hold no line with the word "experimental", and the exit status is 0

#### Scenario: Usage
- **WHEN** the usage message is read
- **THEN** the line of `--runtime` says of `rgss2` only that it is RPG Maker VX, and of `rgss3` that it is not supported yet

### Requirement: Version of a guest process
A guest process SHALL state the version of its executable as the first bytes of each
connection it opens to the program, before its first message. The program SHALL refuse a
connection that states another version than its own, or none, with an error that names its
own version and the version stated; when it is the connection of the guest process the
program has just started, the run SHALL end with that error.

#### Scenario: Same version
- **WHEN** a game is run headless with `--guest process` and a states schedule that saves and loads
- **THEN** the guest process and its copies attach and the run exits with status 0

#### Scenario: Other version
- **WHEN** a process connects to the socket of the program, states the version `0.0.0` and sends an attachment
- **THEN** the attachment is refused with an error that contains `0.0.0` and the version of the program

#### Scenario: No statement
- **WHEN** a process connects to the socket of the program and sends an attachment without stating a version
- **THEN** the attachment is refused with an error that names the version of the program

#### Scenario: Statement and message in one write
- **WHEN** a guest writes the statement of the program's version and its attachment to the connection at once
- **THEN** the program reads the statement and then the attachment, whole

### Requirement: Executable of the guest process
The program SHALL take the executable it starts as its guest process from the runtime of
the game. For RGSS1 and for RGSS2 it SHALL be the executable of the program itself.

#### Scenario: XP and VX
- **WHEN** the executable of the guest process is asked for RGSS1 and for RGSS2
- **THEN** both answers are the path of the running executable
