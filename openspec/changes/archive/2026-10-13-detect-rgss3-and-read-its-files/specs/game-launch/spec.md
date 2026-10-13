## MODIFIED Requirements

### Requirement: Built-in scripts file
The program SHALL load the built-in scripts of the run's runtime from the file given by its
option, or from a default location under the user's data directory: `rpg_builtin.rb` for
RGSS1, `rpg_builtin_rgss2.rb` for RGSS2 and `rpg_builtin_rgss3.rb` for RGSS3. When neither
exists it SHALL end with an error that names the file and how it is made. The repository
SHALL NOT contain the built-in scripts of an original runtime. In a run of RGSS3, which ends
before a guest starts, the file SHALL be found and SHALL NOT be evaluated.

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
plain text in the file it is given, an RGSS DLL or the image of one as it lies in the memory
of the running original, in the order they appear, and write them to the output file as Ruby
source. A block SHALL be a NUL-terminated string of text that starts with `module RPG;`,
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

### Requirement: VX Ace is refused
A run whose runtime is RGSS3 SHALL end before a guest, an interpreter, a window or a device is
started, with a non-zero status and a message that names RPG Maker VX Ace and says that it is
planned and not supported. Before that message the program SHALL do, in this order, what it
does for a game of any runtime before a guest starts: resolve the run-time package that `RTP`
names, find the built-in scripts file of RGSS3, open the game's files with `Game.rgss3a` when
the directory holds it, and read the file that `Scripts` names from them. When one of these
steps fails, the run SHALL end with that step's error and a non-zero status, and the message
SHALL NOT be printed.

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
