## ADDED Requirements

### Requirement: Runtime of a game
The runtime of a game (RGSS1, RGSS2 or RGSS3) SHALL be derived from two values of the
configuration file: a `Library` whose file name is `RGSS`, a digit 1 to 3, two digits, an
optional letter and `.dll` gives that digit's runtime, and a `Scripts` ending in `.rxdata`,
`.rvdata` or `.rvdata2` gives RGSS1, RGSS2 or RGSS3. Letter case SHALL NOT matter. When both
give a runtime and differ, or neither gives one, the result SHALL be an error naming both values.

#### Scenario: Both agree
- **WHEN** the file has `Library=RGSS202E.dll` and `Scripts=Data\Scripts.rvdata`
- **THEN** the runtime is RGSS2

#### Scenario: One source
- **WHEN** the file has `Library=Player.dll` and `Scripts=Data\Scripts.rxdata`, or an empty `Library` and the same `Scripts`
- **THEN** the runtime is RGSS1

#### Scenario: Library in a folder
- **WHEN** the file has `Library=System\RGSS301.dll` and `Scripts=Data\Scripts.rvdata2`
- **THEN** the runtime is RGSS3

#### Scenario: Conflict
- **WHEN** the file has `Library=RGSS104E.dll` and `Scripts=Data\Scripts.rvdata`
- **THEN** the result is an error that names both values

#### Scenario: Nothing recognised
- **WHEN** the file has `Library=Player.dll` and `Scripts=Data\Code.bin`
- **THEN** the result is an error that names both values

## MODIFIED Requirements

### Requirement: Game configuration file
The game configuration file `Game.ini` SHALL be read from the game directory, found regardless
of the letter case of its name. From its `Game` section the values `Library`, `Scripts`, `Title`,
`RTP`, `RTP1`, `RTP2` and `RTP3` SHALL be returned, with section and key names matched without
regard to case, surrounding white space removed and both line-ending styles accepted. A missing
file or a missing `Scripts` value SHALL be an error; other missing values SHALL be empty.

#### Scenario: Typical file
- **WHEN** a file with `Library=RGSS103J.dll`, `Scripts=Data\Scripts.rxdata`, a title and `RTP1=Standard` is read
- **THEN** those four values are returned and `RTP`, `RTP2` and `RTP3` are empty

#### Scenario: File of VX
- **WHEN** a file with `RTP=RPGVX`, `Library=RGSS202E.dll` and `Scripts=Data\Scripts.rvdata` is read
- **THEN** `RTP` is `RPGVX` and `RTP1` to `RTP3` are empty

#### Scenario: Case and line endings
- **WHEN** the file is named `game.INI`, uses `[GAME]` and `scripts=`, and has CRLF line endings
- **THEN** it is read with the same result

#### Scenario: No scripts entry
- **WHEN** the file has no `Scripts` value
- **THEN** reading fails with an error that names the missing key

### Requirement: Run-time package directories
The run-time package names of a game SHALL be `RTP1`, `RTP2`, `RTP3` in that order for RGSS1
and the one value `RTP` for RGSS2. They SHALL be resolved to directories through a table
supplied by the caller, skipping empty names. A name that the table does not contain, or whose
directory does not exist, SHALL be an error naming it.

#### Scenario: Known package
- **WHEN** `RTP1` is `Standard` and the table maps `Standard` to an existing directory
- **THEN** that directory is the first run-time package directory

#### Scenario: Package of VX
- **WHEN** the runtime is RGSS2, `RTP` is `RPGVX`, `RTP1` is `Standard`, and the table maps only `RPGVX`
- **THEN** the directory of `RPGVX` is the only run-time package directory

#### Scenario: Unknown package
- **WHEN** the configuration names a package the table does not contain
- **THEN** resolution fails with an error naming that package

### Requirement: Encrypted archive
When the game directory holds the archive of the game's runtime, `Game.rgssad` for RGSS1 and
`Game.rgss2a` for RGSS2 (name compared without regard to case), the game's files SHALL include
the entries of that RGSSAD version 1 archive, decrypted. Entry names SHALL be matched like
loose file names. An entry SHALL take precedence over a loose file of the same name, and loose
files over the RTPs. The archive of another runtime SHALL be ignored.

#### Scenario: Data from the archive
- **WHEN** an archive holds `Data\Scripts.rxdata` and the data file `Data/Scripts.rxdata` is read
- **THEN** the entry's decrypted bytes are returned

#### Scenario: Image with extension search
- **WHEN** an archive holds `Graphics\Titles\Title.png` and the image `graphics/titles/title` is read
- **THEN** the entry's bytes are returned

#### Scenario: Precedence
- **WHEN** the archive and the game directory both hold `Data/Map001.rxdata` with different contents
- **THEN** the archive's contents are returned

#### Scenario: No archive
- **WHEN** the game directory holds no archive
- **THEN** files resolve exactly as before

#### Scenario: Archive of VX
- **WHEN** the runtime is RGSS2 and `Game.rgss2a` holds `Data\Scripts.rvdata`
- **THEN** reading the data file `Data/Scripts.rvdata` returns the entry's decrypted bytes

#### Scenario: Archive of another runtime
- **WHEN** the runtime is RGSS1 and the directory holds only `Game.rgss2a`
- **THEN** files resolve as if there were no archive
