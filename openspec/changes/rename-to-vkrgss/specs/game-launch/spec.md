## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Executable of the guest process
The program SHALL take the executable it starts as its guest process from the runtime of
the game: itself for RGSS1 and RGSS2, and for RGSS3 the file `vkrgss-guest-rgss3` in the
directory of its own executable file. When that file is missing, the run SHALL end before a
guest, a window or a device is started, with an error that names the file and the directory.

#### Scenario: XP and VX
- **WHEN** the executable of the guest process is asked for RGSS1 and for RGSS2
- **THEN** both answers are the path of the running executable

#### Scenario: VX Ace
- **WHEN** it is asked for RGSS3
- **THEN** the answer is the path `vkrgss-guest-rgss3` in the directory that holds the running executable

#### Scenario: Missing guest executable
- **WHEN** a copy of the program that lies alone in a directory is run headless on a generated game of RGSS3
- **THEN** it exits with a non-zero status, its error stream names `vkrgss-guest-rgss3` and that directory, and no script has run

#### Scenario: A guest executable that ends at once
- **WHEN** a file `vkrgss-guest-rgss3` that only exits with status 3 lies beside that copy and the game is run again
- **THEN** the program exits with a non-zero status and its error stream says how the guest process ended, with the status 3

#### Scenario: XP and VX need no second file
- **WHEN** that copy, alone in its directory, runs a generated game of RGSS1 and one of RGSS2 headless with `--guest process`
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
