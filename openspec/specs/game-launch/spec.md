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
The program SHALL load the built-in scripts from the file given by its option, or from a
default location under the user's data directory, and when neither exists SHALL end with an
error that names the tool which creates the file. The repository SHALL NOT contain the
built-in scripts of the original runtime.

#### Scenario: Missing file
- **WHEN** the program is run without the option and no file exists at the default location
- **THEN** it exits with a non-zero status and the message names `cargo xtask extract-rpg`

### Requirement: Extraction tool
`cargo xtask extract-rpg <DLL> <OUTPUT>` SHALL find the built-in Ruby script blocks stored as
plain text in an RGSS DLL, in the order they appear, and write them to the output file as Ruby
source. It SHALL fail with a clear message when the DLL contains no such block, as a DLL still
compressed by its protector does.

#### Scenario: Blocks found
- **WHEN** the tool is run on a file that contains two NUL-terminated strings starting with `module RPG;`
- **THEN** the output file holds both, the first before the second

#### Scenario: Nothing found
- **WHEN** the tool is run on a file without such a string
- **THEN** it exits with a non-zero status and writes no output file

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
The program SHALL read a game whose data is in `Game.rgssad`, and SHALL end with an error
naming the archive when it is damaged.

#### Scenario: Game in an archive
- **WHEN** a generated game whose `Data` folder exists only inside `Game.rgssad` is run headless
- **THEN** it runs like the same game with loose files

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
