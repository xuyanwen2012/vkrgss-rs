## ADDED Requirements

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

### Requirement: Experimental runtime
A run of RGSS2 SHALL say once, as a warning of the program's diagnostics, that support for
RPG Maker VX is experimental. A run of RGSS1 SHALL NOT say it.

#### Scenario: VX
- **WHEN** a generated game of RGSS2 whose only script returns at once is run headless
- **THEN** the diagnostics hold one line that says VX is experimental, and the exit status is 0

#### Scenario: XP
- **WHEN** a generated game of RGSS1 is run headless
- **THEN** the diagnostics hold no such line

### Requirement: VX Ace is refused
A run whose runtime is RGSS3 SHALL end before the guest starts, with a non-zero status and a
message that names RPG Maker VX Ace and says that it is planned and not supported.

#### Scenario: Ace game
- **WHEN** a directory with `Library=System\RGSS301.dll` and `Scripts=Data\Scripts.rvdata2` is run
- **THEN** the program exits with a non-zero status, the message says VX Ace is not supported, and no script has run

## MODIFIED Requirements

### Requirement: Built-in scripts file
The program SHALL load the built-in scripts of the run's runtime from the file given by its
option, or from a default location under the user's data directory: `rpg_builtin.rb` for RGSS1
and `rpg_builtin_rgss2.rb` for RGSS2. When neither exists it SHALL end with an error that names
the file and how it is made. The repository SHALL NOT contain the built-in scripts of an
original runtime.

#### Scenario: Missing file
- **WHEN** the program is run without the option and no file exists at the default location
- **THEN** it exits with a non-zero status and the message names `cargo xtask extract-rpg`

#### Scenario: Missing file of VX
- **WHEN** a game of RGSS2 is run without the option and no `rpg_builtin_rgss2.rb` exists at the default location
- **THEN** it exits with a non-zero status and the message names that file and `docs/running.md`

### Requirement: Encrypted archive of the game
The program SHALL read a game whose data is in the archive of its runtime, `Game.rgssad` or
`Game.rgss2a`, and SHALL end with an error naming the archive when it is damaged.

#### Scenario: Game in an archive
- **WHEN** a generated game whose `Data` folder exists only inside `Game.rgssad` is run headless
- **THEN** it runs like the same game with loose files

#### Scenario: Game of VX in an archive
- **WHEN** a generated game of RGSS2 whose `Data` folder exists only inside `Game.rgss2a` is run headless
- **THEN** it runs like the same game with loose files
