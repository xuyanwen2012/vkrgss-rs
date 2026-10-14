## REMOVED Requirements

### Requirement: VX Ace is refused
**Reason**: A game of RGSS3 runs, in the guest executable `vkxp-guest-rgss3`. What the program does before the guest starts (the run-time package, the built-in scripts file, the archive, the script archive) is unchanged and is held by the requirements "A game of VX Ace runs", "Built-in scripts file" and "Encrypted archive of the game".
**Migration**: None. A run of RGSS3 that ended with the message that VX Ace is planned now starts the game. The function `refuse_rgss3` and its tests are gone.

## MODIFIED Requirements

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
- **WHEN** the same game is run without the option and the stand-in built-in scripts of the tests are the file `rpg_builtin_rgss3.rb` in the directory `vkxp` of the user's data directory
- **THEN** no message about a built-in scripts file is printed, the scripts of the game run and the exit status is 0

#### Scenario: A file of VX Ace that is not Ruby
- **WHEN** that file holds text that does not parse
- **THEN** the run ends with a non-zero status and a report of a `SyntaxError`, and no script of the game has run

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

### Requirement: Guest option
The form SHALL be chosen by `--guest thread` or `--guest process`, by the environment variable
`VKXP_GUEST` and by the configuration key `guest`, the command line before the environment and
the environment before the configuration files. Without a choice the guest SHALL be a process
in a window and in every run of RGSS3, and a thread in a headless run of RGSS1 or RGSS2. A
choice of `thread` for a game of RGSS3 SHALL end the run with an error naming `--guest process`.

#### Scenario: Unknown value
- **WHEN** `vkxp` is started with `--guest fiber`
- **THEN** it exits with an error that names `thread` and `process`

#### Scenario: Environment
- **WHEN** `vkxp` is started with `VKXP_GUEST=process` and without `--guest`
- **THEN** the guest runs in a child process

#### Scenario: Headless default
- **WHEN** `vkxp` is started headless without a choice on a game of RGSS1 or of RGSS2
- **THEN** the guest runs as a thread

#### Scenario: Headless default of VX Ace
- **WHEN** a generated game of RGSS3 is run headless with `--verbose` and without a choice
- **THEN** the diagnostics say that a guest process attached, and the exit status is 0

#### Scenario: Thread with VX Ace
- **WHEN** a generated game of RGSS3 is run headless with `--guest thread`, and again with `VKXP_GUEST=thread` and no option
- **THEN** each run exits with a non-zero status before a script has run, and its error stream names RPG Maker VX Ace and `--guest process`

### Requirement: Supported runtimes say nothing about their state
A run of RGSS1 or of RGSS2 SHALL NOT say that support for its runtime is experimental, and
the usage message SHALL name `rgss1` and `rgss2` without such a word.

#### Scenario: VX
- **WHEN** a generated game of RGSS2 whose only script returns at once is run headless
- **THEN** the diagnostics hold no line with the word "experimental", and the exit status is 0

#### Scenario: Usage
- **WHEN** the usage message is read
- **THEN** the line of `--runtime` says of `rgss2` only that it is RPG Maker VX, and of `rgss3` that it is RPG Maker VX Ace and experimental

### Requirement: Executable of the guest process
The program SHALL take the executable it starts as its guest process from the runtime of
the game: itself for RGSS1 and RGSS2, and for RGSS3 the file `vkxp-guest-rgss3` in the
directory of its own executable file. When that file is missing, the run SHALL end before a
guest, a window or a device is started, with an error that names the file and the directory.

#### Scenario: XP and VX
- **WHEN** the executable of the guest process is asked for RGSS1 and for RGSS2
- **THEN** both answers are the path of the running executable

#### Scenario: VX Ace
- **WHEN** it is asked for RGSS3
- **THEN** the answer is the path `vkxp-guest-rgss3` in the directory that holds the running executable

#### Scenario: Missing guest executable
- **WHEN** a copy of the program that lies alone in a directory is run headless on a generated game of RGSS3
- **THEN** it exits with a non-zero status, its error stream names `vkxp-guest-rgss3` and that directory, and no script has run

#### Scenario: A guest executable that ends at once
- **WHEN** a file `vkxp-guest-rgss3` that only exits with status 3 lies beside that copy and the game is run again
- **THEN** the program exits with a non-zero status and its error stream says how the guest process ended, with the status 3

#### Scenario: XP and VX need no second file
- **WHEN** that copy, alone in its directory, runs a generated game of RGSS1 and one of RGSS2 headless with `--guest process`
- **THEN** both runs exit with status 0

## ADDED Requirements

### Requirement: A game of VX Ace runs
A run whose runtime is RGSS3 SHALL play the game in a guest process of `vkxp-guest-rgss3`,
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

### Requirement: Experimental runtime says so
A run of RGSS3 SHALL say once, as a warning of the program's diagnostics, that support for
RPG Maker VX Ace is experimental, whatever the form of the guest was chosen by. The guest
process SHALL NOT say it again.

#### Scenario: VX Ace
- **WHEN** a generated game of RGSS3 whose only script returns at once is run headless without `--verbose`
- **THEN** the error stream holds exactly one line with the word "experimental", that line names RPG Maker VX Ace, and the exit status is 0

#### Scenario: XP
- **WHEN** a generated game of RGSS1 is run headless with `--guest process`
- **THEN** the error stream holds no such line

### Requirement: An executable runs the runtimes of its interpreter
An executable SHALL play a game only when its interpreter is that of the game's runtime:
Ruby 1.8.7 for RGSS1 and RGSS2, Ruby 1.9.2 for RGSS3. Given a guest record of another
runtime it SHALL end with a non-zero status and a message that names the product of that
runtime, before it opens a game file, starts an interpreter or connects.

#### Scenario: The guest executable of RGSS3 given a game of XP
- **WHEN** `vkxp-guest-rgss3 --guest-role <file>` is run with a record of its own version whose game directory holds a `Game.ini` of RGSS1 and whose socket nobody listens on
- **THEN** it exits with a non-zero status and its error stream names RPG Maker XP and not the socket

#### Scenario: The program given a record of VX Ace
- **WHEN** `vkxp --guest-role <file>` is run with such a record of a game of RGSS3
- **THEN** it exits with a non-zero status and its error stream names RPG Maker VX Ace and not the socket

#### Scenario: Refused records of the guest executable
- **WHEN** `vkxp-guest-rgss3 --guest-role <file>` is run with a path at which no file is, with a record that states the version `0.0.0`, and with a file that holds the line of its version followed by bytes that are no record
- **THEN** each run exits with a non-zero status and names the file, and the second names `0.0.0` and the version of the executable

#### Scenario: Not started as a guest
- **WHEN** `vkxp-guest-rgss3` is run without arguments, and with `--help`
- **THEN** each run exits with a non-zero status and one line on its error stream that names `vkxp`, and nothing is written to the standard output

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
