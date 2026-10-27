# game-launch Specification

## Purpose
Defines how a game is started with the `vkrgss` program: its command line, the windowed and
headless modes, how it ends, the fonts it draws text with, and the tool that provides the
built-in scripts a game needs.

## Requirements

### Requirement: Starting a game
`vkrgss <game directory>` SHALL run the game in that directory: with the directory as the working
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
SHALL NOT contain such scripts.

#### Scenario: Missing file
- **WHEN** the program is run without the option and no file exists at the default location
- **THEN** it exits with a non-zero status and the message names `cargo xtask extract-rpg`

#### Scenario: Missing file of VX
- **WHEN** a game of RGSS2 is run without the option and no `rpg_builtin_rgss2.rb` exists at the default location
- **THEN** it exits with a non-zero status and the message names that file and `docs/running.md`

#### Scenario: Missing file of VX Ace
- **WHEN** a generated game of RGSS3 is run headless without the option and no `rpg_builtin_rgss3.rb` exists at the default location
- **THEN** it exits with a non-zero status, the message names that file and `docs/running.md`, and no guest process was started

#### Scenario: Default location of VX Ace
- **WHEN** the same game is run without the option and the stand-in built-in scripts of the tests are the file `rpg_builtin_rgss3.rb` in the directory `vkrgss` of the user's data directory
- **THEN** no message about a built-in scripts file is printed, the scripts of the game run and the exit status is 0

#### Scenario: A file of VX Ace that is not Ruby
- **WHEN** that file holds text that does not parse
- **THEN** the run ends with a non-zero status and a report of a `SyntaxError`, and no script of the game has run

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
- **WHEN** a generated game of RGSS3 whose script archive and one data file exist only inside a generated `Game.rgss3a` of version 3 is run headless, its script reading the data file with `load_data` and dumping a frame
- **THEN** it exits with status 0 and writes the same frame and the same value as the same game with loose files

#### Scenario: Damaged archive of VX Ace
- **WHEN** that game is run with its `Game.rgss3a` cut inside the entry table
- **THEN** the program exits with a non-zero status, the message names `Game.rgss3a`, and no script has run

### Requirement: Configuration files
The program SHALL read settings from a user configuration file and from `vkrgss.toml` in the game
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
By default warnings and errors SHALL be shown, each as a line that starts with `vkrgss: `. Text
the game produces and the program's usage and fatal error message SHALL be shown at every
level.

#### Scenario: Default level
- **WHEN** a game runs without `--verbose` and without `VKRGSS_LOG`
- **THEN** the error stream has no informational line

#### Scenario: Script error at every level
- **WHEN** a game whose script raises runs with `VKRGSS_LOG=error`
- **THEN** the error stream still carries the script error report

### Requirement: Diagnostic level
The option `--verbose` SHALL also show informational and debug events. The environment
variable `VKRGSS_LOG` SHALL set the level or a per-module filter and take precedence over
`--verbose`; a value that cannot be parsed SHALL end the program with a message and a non-zero
status.

#### Scenario: Verbose
- **WHEN** the same game runs headless with `--verbose`
- **THEN** the error stream names the built-in scripts file that was loaded

#### Scenario: Level from the environment
- **WHEN** the game runs with `VKRGSS_LOG=error` and `--verbose`
- **THEN** the error stream has no informational line

#### Scenario: Malformed filter
- **WHEN** the program runs with `VKRGSS_LOG` set to `=`
- **THEN** it ends with a message naming the variable and a non-zero status

### Requirement: Frame statistics option
The program SHALL accept `--frame-stats`, which requests the frame time statistics of the
host session, in windowed and in headless mode.

#### Scenario: Option accepted
- **WHEN** the program is run with `--frame-stats` in headless mode
- **THEN** it runs the game and writes the summary to the error stream

### Requirement: Guest in a process
`vkrgss` SHALL run the guest either as a thread of its own process or in a child process. In a
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
- **THEN** `vkrgss` ends with a status other than 0 and its error stream names the signal

#### Scenario: Frame limit
- **WHEN** a game that never ends is run headless with `--guest process --frames 5`
- **THEN** `vkrgss` exits with status 0 and no guest process is left running

### Requirement: Guest option
The form SHALL be chosen by `--guest thread` or `--guest process`, by the environment variable
`VKRGSS_GUEST` and by the configuration key `guest`, the command line before the environment and
the environment before the configuration files. Without a choice the guest SHALL be a process
in a window and in every run of RGSS3, and a thread in a headless run of RGSS1 or RGSS2. A
choice of `thread` for a game of RGSS3 SHALL end the run with an error naming `--guest process`.

#### Scenario: Unknown value
- **WHEN** `vkrgss` is started with `--guest fiber`
- **THEN** it exits with an error that names `thread` and `process`

#### Scenario: Environment
- **WHEN** `vkrgss` is started with `VKRGSS_GUEST=process` and without `--guest`
- **THEN** the guest runs in a child process

#### Scenario: Headless default
- **WHEN** `vkrgss` is started headless without a choice on a game of RGSS1 or of RGSS2
- **THEN** the guest runs as a thread

#### Scenario: Headless default of VX Ace
- **WHEN** a generated game of RGSS3 is run headless with `--verbose` and without a choice
- **THEN** the diagnostics say that a guest process attached, and the exit status is 0

#### Scenario: Thread with VX Ace
- **WHEN** a generated game of RGSS3 is run headless with `--guest thread`, and again with `VKRGSS_GUEST=thread` and no option
- **THEN** each run exits with a non-zero status before a script has run, and its error stream names RPG Maker VX Ace and `--guest process`

### Requirement: States option and parked copies
`--states SCHEDULE` SHALL give a headless run its state requests, as `frame:save N` and
`frame:load N` entries separated by commas, and SHALL be an error with the guest as a thread.
States SHALL be kept as copies of the guest process, and when `vkrgss` ends no process of the
run SHALL be left. The exit status SHALL follow the rules of a run without states.

#### Scenario: Thread form
- **WHEN** `vkrgss` is started headless with `--states "5:save 0"` and without `--guest process`
- **THEN** it exits with an error that names `--guest process`

#### Scenario: Malformed schedule
- **WHEN** `--states "5:keep 0"` or a slot above 9 is given
- **THEN** `vkrgss` exits with an error that names the entry

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
`vkrgss --version` and `vkrgss -V` SHALL print `vkrgss` and the version of the package on one line
to the standard output and exit with status 0, without opening a device or a game.

#### Scenario: Version
- **WHEN** `vkrgss --version` is run with no Vulkan driver selectable and no game directory
- **THEN** the standard output is `vkrgss 0.1.0` followed by a line break and the exit status is 0

#### Scenario: Listed in the usage
- **WHEN** `vkrgss --help` is run
- **THEN** the text lists `--version`

### Requirement: Help option
`vkrgss --help` and `vkrgss -h` SHALL print the usage text to the standard output and exit with
status 0, without opening a device or a game. An unknown option SHALL keep ending with a
status other than 0 and the usage on the error stream.

#### Scenario: Help
- **WHEN** `vkrgss --help` is run with no Vulkan driver selectable and no game directory
- **THEN** the standard output holds the usage text, which lists `--help` and `--version`, the error stream is empty and the exit status is 0

#### Scenario: Unknown option
- **WHEN** `vkrgss --no-such-option` is run
- **THEN** the exit status is 2 and the error stream names the option

### Requirement: Confined guest process
A guest process SHALL restrict itself, before the first script runs, so that it can write,
create, remove and rename only below the game directory, can execute no file and, where the
kernel has the rule, can open no TCP connection; reading SHALL stay unrestricted. On Linux and
Android it SHALL also deny, with `EPERM`, a fixed list of system calls no game needs. Where the
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

#### Scenario: Denied system calls
- **WHEN** a script of a game run with `--guest process` calls `system("true")` and `Process.exec("true")`, and code in the guest opens an `AF_INET` socket
- **THEN** `system` starts nothing, `Process.exec` raises `Errno::EPERM`, and the socket fails with `EPERM`, while an `AF_UNIX` socket, file I/O, threads and `fork` work

### Requirement: Sandbox option
`--sandbox on|off`, the environment variable `VKRGSS_SANDBOX` and the configuration key
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
A run of RGSS1, RGSS2 or RGSS3 SHALL NOT say that support for its runtime is experimental,
and the usage message SHALL name `rgss1`, `rgss2` and `rgss3` without such a word.

#### Scenario: VX
- **WHEN** a generated game of RGSS2 whose only script returns at once is run headless
- **THEN** the diagnostics hold no line with the word "experimental", and the exit status is 0

#### Scenario: VX Ace
- **WHEN** a generated game of RGSS3 whose only script returns at once is run headless
- **THEN** the diagnostics hold no line with the word "experimental", and the exit status is 0

#### Scenario: Usage
- **WHEN** the usage message is read
- **THEN** the line of `--runtime` says of `rgss2` only that it is RPG Maker VX, and of `rgss3` only that it is RPG Maker VX Ace

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
the game, in the directory of its own executable file: `vkrgss-guest-rgss12` for RGSS1 and
RGSS2, `vkrgss-guest-rgss3` for RGSS3. The program SHALL NOT start itself as a guest. When that file is missing, the run SHALL end before a
guest, a window or a device is started, with an error that names the file and the directory.

#### Scenario: XP and VX
- **WHEN** the executable of the guest process is asked for RGSS1 and for RGSS2
- **THEN** both answers are the path `vkrgss-guest-rgss12` in the directory that holds the running executable

#### Scenario: VX Ace
- **WHEN** it is asked for RGSS3
- **THEN** the answer is the path `vkrgss-guest-rgss3` in the directory that holds the running executable

#### Scenario: Missing guest executable
- **WHEN** a copy of the program that lies alone in a directory is run headless on a generated game of RGSS3
- **THEN** it exits with a non-zero status, its error stream names `vkrgss-guest-rgss3` and that directory, and no script has run

#### Scenario: A guest executable that ends at once
- **WHEN** a file `vkrgss-guest-rgss3` that only exits with status 3 lies beside that copy and the game is run again
- **THEN** the program exits with a non-zero status and its error stream says how the guest process ended, with the status 3

#### Scenario: XP and VX need their guest executable
- **WHEN** that copy, alone in its directory, runs a generated game of RGSS1 with `--guest process`
- **THEN** it exits with a non-zero status, its error stream names `vkrgss-guest-rgss12` and that directory, and no script has run

#### Scenario: XP and VX need no second file
- **WHEN** that copy, alone in its directory, runs a generated game of RGSS1 and one of RGSS2 headless with `--guest thread`
- **THEN** both runs exit with status 0

### Requirement: Record of a guest process
The program SHALL hand a guest process what it needs of the run (the game directory, the
settings a guest reads, the seed, the socket, the diagnostic level) in a record: a file it
writes into the private directory of the socket before it starts the process, and names as
the only value of the hidden option `--guest-role`. A guest process SHALL NOT read the
options of the program's command line or a configuration file.

#### Scenario: Arguments of the guest process
- **WHEN** the program prepares the command that starts a guest process
- **THEN** the arguments of the command are `--guest-role` and the path of a file in the directory of the socket, and no other

#### Scenario: Configuration files are read once
- **WHEN** a game is run headless with `--guest process --verbose` and a user configuration file, and without a configuration file of the game
- **THEN** the run exits with status 0 and its error stream says exactly once that a configuration file was read

#### Scenario: Every setting
- **WHEN** a record with a value for every setting, two run-time packages, two font directories, two preload scripts and a path that is not UTF-8 is written and then read by the same version
- **THEN** what is read equals what was written

#### Scenario: Settings of the run
- **WHEN** the game that loads a picture of a run-time package and the game that a preload script named by a relative path patches are each run headless with `--guest process`
- **THEN** each run exits with status 0 and shows what it shows with the guest on a thread: the picture is loaded, and the dumped frame has the patch

#### Scenario: Level of the guest process
- **WHEN** a game is run headless with `--guest process --allow-process-functions --verbose` and without `VKRGSS_LOG`
- **THEN** the error stream has the informational line in which the guest process says that it is not confined

### Requirement: Refused guest record
A record SHALL start with a line that states the version of the program that wrote it. An
executable whose first argument is `--guest-role` SHALL end with a non-zero status and a
message on the error stream that names the file, before it opens a game or a connection,
when the record cannot be read, is not a record or is of another version than its own; for
another version the message SHALL name both. Elsewhere on the command line the option
SHALL be an unknown option.

#### Scenario: No such file
- **WHEN** `vkrgss --guest-role <path>` is run with a path at which no file is
- **THEN** it exits with a non-zero status and its error stream names the path

#### Scenario: The value of version 0.2.0
- **WHEN** `vkrgss --guest-role 7=/tmp/guest.sock` is run by hand
- **THEN** it exits with a non-zero status and its error stream names `7=/tmp/guest.sock` as a record that cannot be read

#### Scenario: Other version
- **WHEN** `vkrgss --guest-role <file>` is run with a record that states the version `0.0.0`
- **THEN** it exits with a non-zero status and its error stream contains `0.0.0` and the version of the program

#### Scenario: Not a record
- **WHEN** `vkrgss --guest-role <file>` is run with a file that holds the line of the program's version followed by bytes that are no record
- **THEN** it exits with a non-zero status and its error stream names the file

#### Scenario: Among other arguments
- **WHEN** `vkrgss <game directory> --guest-role <file>` is run
- **THEN** the exit status is 2 and the error stream names `--guest-role` as an unknown option

### Requirement: A game of VX Ace runs
A run whose runtime is RGSS3 SHALL play the game in a guest process of `vkrgss-guest-rgss3`,
over the interpreter of RGSS3, with the frame size, the dumps, the frame limit, the input
script and the exit status rules of any run. Before the guest starts the program SHALL
resolve the run-time package and find the built-in scripts file, and a step that fails
SHALL end the run with its own error.

#### Scenario: Ace game
- **WHEN** a generated game with `Library=System\RGSS301.dll`, `Scripts=Data\Scripts.rvdata2`, an empty `RTP` and one script that writes `RUBY_VERSION` to a file, shows a red sprite of 32 by 32 at (10, 20) for three frames and returns is run headless with a dump of frame 2
- **THEN** the exit status is 0, the file holds `1.9.2`, and the dumped picture is 544 by 416 with a red pixel at (10, 20) and none at (9, 20)

#### Scenario: Library under the other name
- **WHEN** the same game names `Library=System\RGSS300.dll`
- **THEN** the run ends the same way

#### Scenario: Runtime named by the option
- **WHEN** a generated game of RGSS1 whose script writes `RUBY_VERSION` to a file is run headless with `--runtime rgss3` and the stand-in built-in scripts of RGSS3
- **THEN** the exit status is 0 and the file holds `1.9.2`

#### Scenario: Run-time package not given
- **WHEN** the game's configuration file has `RTP=RPGVXAce` and the run has no `--rtp` option
- **THEN** the program exits with a non-zero status, the message names `RPGVXAce` and `--rtp`, and no script has run

#### Scenario: No script archive
- **WHEN** the game has neither a loose `Data/Scripts.rvdata2` nor a `Game.rgss3a` that holds it
- **THEN** the program exits with a non-zero status and the message names `Data\Scripts.rvdata2`

#### Scenario: Script error
- **WHEN** line 2 of the second script of a generated game of RGSS3, named `Scene_Title`, raises a `RuntimeError` with the message `boom`
- **THEN** the error stream holds `Script 'Scene_Title' line 2: RuntimeError occurred.` and `boom`, and the exit status is not 0

#### Scenario: Frame limit and input
- **WHEN** a generated game of RGSS3 that never ends and moves a sprite 4 pixels per frame while `Input.press?(Input::RIGHT)` is true is run headless with `--frames 12`, `--input "2:RIGHT, 6:"` and dumps of the frames 1 and 8
- **THEN** the exit status is 0, no guest process is left, and the sprite is further right in the second picture than in the first

#### Scenario: Two runs
- **WHEN** a generated game of RGSS3 that places a sprite by `rand` is run twice headless with the same seed and a dump of the same frame
- **THEN** the two pictures are byte for byte equal

### Requirement: An executable runs the runtimes of its interpreter
An executable SHALL play a game only when its interpreter is that of the game's runtime:
Ruby 1.8.7 for RGSS1 and RGSS2, Ruby 1.9.2 for RGSS3. Given a guest record of another
runtime it SHALL end with a non-zero status and a message that names the product of that
runtime, before it opens a game file, starts an interpreter or connects.

#### Scenario: The guest executable of RGSS3 given a game of XP
- **WHEN** `vkrgss-guest-rgss3 --guest-role <file>` is run with a record of its own version whose game directory holds a `Game.ini` of RGSS1 and whose socket nobody listens on
- **THEN** it exits with a non-zero status and its error stream names RPG Maker XP and not the socket

#### Scenario: The program given a record of VX Ace
- **WHEN** `vkrgss --guest-role <file>` is run with such a record of a game of RGSS3
- **THEN** it exits with a non-zero status and its error stream names RPG Maker VX Ace and not the socket

#### Scenario: Refused records of the guest executable
- **WHEN** `vkrgss-guest-rgss3 --guest-role <file>` is run with a path at which no file is, with a record that states the version `0.0.0`, and with a file that holds the line of its version followed by bytes that are no record
- **THEN** each run exits with a non-zero status and names the file, and the second names `0.0.0` and the version of the executable

#### Scenario: Not started as a guest
- **WHEN** `vkrgss-guest-rgss3` is run without arguments, and with `--help`
- **THEN** each run exits with a non-zero status and one line on its error stream that names `vkrgss`, and nothing is written to the standard output

### Requirement: States and the sandbox with a game of RGSS3
A guest process of RGSS3 SHALL be saved, loaded and confined as one of the other runtimes
is. A request to save a state while a Ruby thread other than the main one is alive SHALL be
answered as a save that failed, with one line of the guest on the error stream that names
the slot and the thread as the reason, and the run SHALL go on.

#### Scenario: A state is loaded
- **WHEN** a generated game of RGSS3 that moves a sprite by a counter of frames and places it by `rand` is run headless with `--guest process` once without states and once with `--states "60:save 0, 100:load 0"`
- **THEN** both runs exit with status 0, and each of the frames 101 to 130 of the second run is byte for byte the frame 40 earlier of the first

#### Scenario: With and without the sandbox
- **WHEN** the run with states is made with `--sandbox on` and with `--sandbox off`
- **THEN** both runs dump the same pictures, and none of the processes that attached exists afterwards

#### Scenario: A script thread is alive
- **WHEN** a generated game of RGSS3 whose first script evaluates `$t = Thread.new { sleep }` is run with `--guest process --states "5:save 0, 10:load 0" --frames 20`
- **THEN** the error stream holds one line of the guest that names slot 0 and a thread, the load of the empty slot is reported, and the exit status is 0

#### Scenario: A script thread has ended
- **WHEN** the first script evaluates `Thread.new { 1 + 1 }.value` instead and the same schedule is run
- **THEN** no such line is written, the diagnostics say that the state was saved and that it was loaded, and the exit status is 0

#### Scenario: Write outside
- **WHEN** a script of a generated game of RGSS3 opens a file for writing in a directory beside the game directory, on a kernel that can restrict
- **THEN** `Errno::EACCES` is raised and no file is there, while a file below the game directory is written

#### Scenario: A script thread is confined too
- **WHEN** the script does the same from inside `Thread.new { ... }.value`
- **THEN** the result is the same

### Requirement: Fonts of a game of RGSS3
For a game of RGSS3 the text engine SHALL take the fonts of the directories named with
`--font-dir`, then of the game's `Fonts` directory, then of the `Fonts` directory of each
run-time package of the game in the order of the packages, each when it exists, then the
fonts installed on the system. For RGSS1 and RGSS2 the list SHALL stay that of the
requirement "Fonts".

#### Scenario: The order of the directories
- **WHEN** the font directories are made for a game of RGSS3 with one `--font-dir`, a game `Fonts` directory and two run-time packages of which only the second has a `Fonts` directory
- **THEN** they are the `--font-dir`, the game's `Fonts` and the second package's `Fonts`, in this order

#### Scenario: VX
- **WHEN** the font directories are made for a game of RGSS2 with the same directories
- **THEN** they are the `--font-dir` and the game's `Fonts`, as before

#### Scenario: A font of the run-time package
- **WHEN** a text backend is made for a game of RGSS3 whose run-time package has a `Fonts` directory holding a copy of a font of the test fonts of `vkrgss-text`
- **THEN** that font's family name exists exactly in the backend

### Requirement: Glyph rule by the library of a game of RGSS3
The guest of a game of RGSS3 SHALL read the version record of the file its `Library` names,
found in the game directory as a data file is, and SHALL set the mask rules of its text
engine to those of 3.0.0.1 for a version whose first three numbers are 3.0.0, and to those
of 3.0.1.1 for any other version, a file without a record, and no file. RGSS1 and RGSS2
SHALL keep the plain rules. The guest record SHALL not change.

#### Scenario: The table of the decision
- **WHEN** the mask rules are chosen for a game of RGSS3 with the versions 3.0.0.1, 3.0.0.7, 3.0.1.1 and 2.0.2.1 and without a version, and for a game of RGSS2 with 3.0.0.1
- **THEN** they are `Rgss3Early`, `Rgss3Early`, `Rgss3`, `Rgss3`, `Rgss3` and `Plain`

#### Scenario: The file named by the configuration
- **WHEN** the guest reads the version of a game whose `Game.ini` has `Library=System\RGSS300.dll` and whose file `system/rgss300.dll` holds a version record 3.0.0.1
- **THEN** the version is 3.0.0.1

#### Scenario: No version
- **WHEN** the guest reads the version of games with an empty `Library`, with a `Library` that names no file, and with a file of 1000 zero bytes
- **THEN** each gives no version, the guest goes on, and its log names the reason in one line

#### Scenario: The name of the file does not count
- **WHEN** the guest reads `System\RGSS300.dll` holding a version record 3.0.1.1
- **THEN** the version is 3.0.1.1 and the mask rules are `Rgss3`

### Requirement: Metric rule of a game of RGSS3
The guest of a game of RGSS3 SHALL set the metric rule of its text engine to that of RGSS3,
whatever version the DLL of the game has. A game of RGSS1 or RGSS2 SHALL keep the plain
metric rule. The mask rules SHALL stay as "Glyph rule by the library of a game of RGSS3"
sets them, and the guest record SHALL not change.

#### Scenario: The table
- **WHEN** the metric rule is chosen for a game of RGSS3 with the versions 3.0.0.1 and 3.0.1.1 and without a version, and for games of RGSS1 and RGSS2
- **THEN** it is the rule of RGSS3 for the first three and the plain rule for the last two

#### Scenario: The backend of a game
- **WHEN** a text backend is made for a game of RGSS3 and for a game of RGSS2 over the test fonts of `vkrgss-text`
- **THEN** its engine has the metric rule of RGSS3 in the first and the plain rule in the second

### Requirement: Title of a game of RGSS3
The title of a game of RGSS3 whose `Game.ini` has no `Title=` value, or an empty one, SHALL
be `Untitled`, for the window the program opens and for the title the guest sends, as
measured on the original runtime. `Game.ini` itself SHALL not be changed, and games of
RGSS1 and RGSS2 SHALL keep their titles.

#### Scenario: Without a title, measured on the original
- **WHEN** the inputs of a guest are read for a game directory whose `Game.ini` names `System\RGSS301.dll` and `Data\Scripts.rvdata2` and has no `Title=`
- **THEN** the title of the configuration is `Untitled`, and the file is unchanged

#### Scenario: Empty value
- **WHEN** the same `Game.ini` has `Title=` without a value
- **THEN** the title of the configuration is `Untitled`

#### Scenario: XP and VX
- **WHEN** the inputs are read for games of RGSS1 and RGSS2 without `Title=`
- **THEN** their titles are empty, as before

### Requirement: Files under the former name
When the user configuration file `vkrgss/config.toml` of the configuration directory, the
game's `vkrgss.toml` or a default built-in scripts file under `vkrgss/` of the data
directory does not exist and the same file with `vkxp` in the place of `vkrgss` does, the
program SHALL read that file and write one warning that names both paths. When both exist,
the new one SHALL be read without such a warning. A file named on the command line SHALL be
read as named.

#### Scenario: User file of version 0.3.0
- **WHEN** a generated game of RGSS1 is run headless with no `vkrgss/config.toml` in the configuration directory and a `vkxp/config.toml` there that sets seed 1
- **THEN** the run uses seed 1, the exit status is 0 and the error stream has exactly one warning, which names both paths

#### Scenario: Both user files
- **WHEN** the same game is run while `vkxp/config.toml` sets seed 1 and `vkrgss/config.toml` sets seed 2
- **THEN** the run uses seed 2 and no line of the error stream names the path of `vkxp/config.toml`

#### Scenario: Game file of version 0.3.0
- **WHEN** a game directory holds `vkxp.toml` that sets seed 2 and no `vkrgss.toml`, and the game is run without a user file
- **THEN** the run uses seed 2 and the error stream has exactly one warning, which names both files

#### Scenario: Built-in scripts of version 0.3.0
- **WHEN** a generated game of RGSS1, and one of RGSS3, is run headless without `--builtin-scripts` while its default file exists under `vkxp/` of the data directory and not under `vkrgss/`
- **THEN** each run loads that file, exits with status 0 and has exactly one warning that names both paths, also with `--guest process`

#### Scenario: No built-in scripts file
- **WHEN** a generated game of RGSS1 is run without `--builtin-scripts` and the file exists under neither name
- **THEN** it exits with a non-zero status and the message names the path under `vkrgss/` and `cargo xtask extract-rpg`, and not the path under `vkxp/`

#### Scenario: Named file
- **WHEN** the program is run with `--config FILE` while only `vkxp/config.toml` exists in the configuration directory
- **THEN** it reads `FILE` alone and writes no warning about the former name

### Requirement: Environment variables under the former name
When `VKRGSS_LOG`, `VKRGSS_GUEST`, `VKRGSS_SANDBOX` or `VKRGSS_PRESENT_TIMING` is not set
and the variable of the same name with `VKXP_` in the place of `VKRGSS_` is, the program
SHALL use the value of that one and write one warning event that names both variables. When
both are set, the new one SHALL be used without such a warning. No other variable of the
former name SHALL be read.

#### Scenario: Level under the former name
- **WHEN** a generated game of RGSS1 is run headless with `VKXP_LOG=info` and without `VKRGSS_LOG`
- **THEN** the error stream has informational lines and exactly one warning, which names `VKXP_LOG` and `VKRGSS_LOG`

#### Scenario: Guest under the former name
- **WHEN** a generated game of RGSS1 is run headless with `VKXP_GUEST=process` and with neither `--guest` nor `VKRGSS_GUEST`
- **THEN** the guest runs in a child process, the exit status is 0 and one warning names `VKXP_GUEST` and `VKRGSS_GUEST`

#### Scenario: Both names
- **WHEN** the same game is run with `VKRGSS_GUEST=thread` and `VKXP_GUEST=process`
- **THEN** the guest runs as a thread and no line of the error stream names `VKXP_GUEST`

#### Scenario: Sandbox under the former name
- **WHEN** the game of "Write outside" is run with `--guest process`, `VKXP_SANDBOX=off` and without `VKRGSS_SANDBOX`
- **THEN** the file is written and one warning names `VKXP_SANDBOX` and `VKRGSS_SANDBOX`

#### Scenario: Other variables
- **WHEN** the allow-list of the former name is read
- **THEN** the only variables of the former name it allows in the sources of the program are `VKXP_LOG`, `VKXP_GUEST`, `VKXP_SANDBOX` and `VKXP_PRESENT_TIMING`

### Requirement: Identity of a process that attaches
The program SHALL take the process number and user of every connection to the guest socket
from the kernel, SHALL refuse a connection whose user is not its own, whose number is not
positive or differs from the number its attachment states, SHALL require the first
connection to come from the guest process it started, and SHALL send signals only to
numbers taken this way.

#### Scenario: Stated number differs
- **WHEN** a process connects to the guest socket and attaches stating the number of another process, or the number 0
- **THEN** the connection is refused with a warning, and no signal is sent to the stated number

#### Scenario: First connection from another process
- **WHEN** the first process that attaches is not the child the program started
- **THEN** the start of the guest fails with an error that says so, and the child is ended

#### Scenario: Copies still attach
- **WHEN** a headless run with `--guest process` and `--states "40:save 0, 80:load 0"` runs
- **THEN** it dumps the same frames as before the change and no process of the run is left

### Requirement: Signals of a confined guest process
A confined guest process on Linux and Android SHALL be refused, with `EPERM`, the system calls
`kill`, `tkill`, `rt_sigqueueinfo`, `rt_tgsigqueueinfo` and `pidfd_send_signal`, and where
the kernel has Landlock's signal scope it SHALL signal no process outside its Landlock
domain. Signals a guest sends its own threads SHALL still arrive.

#### Scenario: Signal to another process
- **WHEN** code in a process that has installed the guest's filter calls `kill` with the number of another process of the same user, or with 0
- **THEN** the call fails with `EPERM` and the other process is still running

#### Scenario: Own threads
- **WHEN** that process calls `raise(SIGUSR1)` with a handler installed, and a thread of it sends `SIGUSR1` to another of its threads with `pthread_kill`
- **THEN** both signals are handled

#### Scenario: Script ends by its signal
- **WHEN** a script run with the confinement does not rescue `Interrupt` and gets SIGINT, over Ruby 1.8.7 and over Ruby 1.9.2
- **THEN** the process ends by SIGINT, as before the change

### Requirement: State answers are made by the program
Of the messages a running guest process sends, the program SHALL hand on no attachment
and no answer about states except a failed save; it SHALL make the answers saved, loaded
and failed load itself. A guest that cannot be copied still answers its own failed save.

#### Scenario: Guest sends a loaded answer
- **WHEN** the running guest process sends a loaded answer for slot 0 without a load request
- **THEN** the host receives no loaded answer, and the next frame of the guest receives its tick

#### Scenario: Guest that cannot be copied
- **WHEN** a guest process started with process functions allowed is asked to save a state
- **THEN** the host receives its failed save answer and says that no state could be saved

### Requirement: Load while a slot is refilled
A load of the slot whose copy was just resumed SHALL wait until the resumed guest has left
its new copy, and then load it; it SHALL fail only if no new copy attaches.

#### Scenario: Load again at once
- **WHEN** the relay is asked to load slot 0, and asked again before the resumed guest's new copy has attached
- **THEN** after that copy attaches the state of slot 0 is loaded again and the host receives a loaded answer, not a failed one
