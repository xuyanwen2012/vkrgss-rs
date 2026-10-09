# Spec Delta

## Purpose
Defines how vkxp finds the files of a game: reading the game configuration file, locating the
run-time packages it names, and resolving the paths scripts use to real files on a
case-sensitive file system.

## ADDED Requirements

### Requirement: Game configuration file
The game configuration file `Game.ini` SHALL be read from the game directory, found regardless
of the letter case of its name. From its `Game` section the values `Library`, `Scripts`, `Title`,
`RTP1`, `RTP2` and `RTP3` SHALL be returned, with section and key names matched without regard
to case, surrounding white space removed and both line-ending styles accepted. A missing file
or a missing `Scripts` value SHALL be an error; other missing values SHALL be empty.

#### Scenario: Typical file
- **WHEN** a file with `Library=RGSS103J.dll`, `Scripts=Data\Scripts.rxdata`, a title and `RTP1=Standard` is read
- **THEN** those four values are returned and `RTP2` and `RTP3` are empty

#### Scenario: Case and line endings
- **WHEN** the file is named `game.INI`, uses `[GAME]` and `scripts=`, and has CRLF line endings
- **THEN** it is read with the same result

#### Scenario: No scripts entry
- **WHEN** the file has no `Scripts` value
- **THEN** reading fails with an error that names the missing key

### Requirement: Text encoding of the configuration file
Values of the configuration file SHALL be decoded as UTF-8 when the file is valid UTF-8, with a
leading byte-order mark ignored. Otherwise they SHALL be decoded with the first of GBK,
Shift-JIS and Big5 in which the whole file decodes without error, and an encoding named by the
caller SHALL take precedence over this detection.

#### Scenario: UTF-8 title
- **WHEN** the title is stored as UTF-8
- **THEN** it is returned unchanged

#### Scenario: GBK title
- **WHEN** the title bytes are `B9 A4 B3 CC`
- **THEN** the title is the two characters U+5DE5 U+7A0B

#### Scenario: Caller names the encoding
- **WHEN** the caller names Shift-JIS and the bytes are valid in both GBK and Shift-JIS
- **THEN** the Shift-JIS reading is returned

### Requirement: Run-time package directories
The run-time package names of the configuration file SHALL be resolved to directories through a
table supplied by the caller, in the order `RTP1`, `RTP2`, `RTP3`, skipping empty names. A name
that the table does not contain, or whose directory does not exist, SHALL be an error naming it.

#### Scenario: Known package
- **WHEN** `RTP1` is `Standard` and the table maps `Standard` to an existing directory
- **THEN** that directory is the first run-time package directory

#### Scenario: Unknown package
- **WHEN** the configuration names a package the table does not contain
- **THEN** resolution fails with an error naming that package

### Requirement: Search order
A script-style path SHALL be looked up first in the game directory and then in each run-time
package directory in order, and the first match SHALL be returned. Data files SHALL be looked
up in the game directory only.

#### Scenario: Game overrides the package
- **WHEN** a picture exists both in the game directory and in a run-time package
- **THEN** the one in the game directory is returned

#### Scenario: Falls back to the package
- **WHEN** a picture exists only in the second run-time package
- **THEN** that file is returned

#### Scenario: Data is not taken from a package
- **WHEN** a data file exists only in a run-time package
- **THEN** the lookup fails

### Requirement: Path matching
Both `/` and `\` SHALL separate path components. Each component SHALL match a directory entry
without regard to letter case, preferring an entry that matches exactly when several differ only
in case. A path that would leave the directory it is searched in SHALL NOT match anything.

#### Scenario: Backslashes and case
- **WHEN** `graphics\TITLES\001-title01.PNG` is looked up and the file is `Graphics/Titles/001-Title01.png`
- **THEN** that file is returned

#### Scenario: Exact case wins
- **WHEN** a directory holds both `a.png` and `A.png` and `A.png` is looked up
- **THEN** `A.png` is returned

#### Scenario: Escaping the root
- **WHEN** `Graphics/../../secret.png` is looked up
- **THEN** the lookup fails even if such a file exists outside the game directory

### Requirement: Extension search
When the path as given names no existing file, the lookup SHALL try it with each extension of
the requested kind appended, in a fixed order: `png`, `jpg`, `jpeg`, `bmp` for images; `mid`,
`ogg`, `wav`, `mp3`, `wma` for audio; none for data. Within one directory a file matching the
path as given SHALL win over one found by appending an extension.

#### Scenario: Extension omitted
- **WHEN** the image `Graphics/Titles/001-Title01` is looked up and only `001-Title01.jpg` exists
- **THEN** that file is returned

#### Scenario: Order of extensions
- **WHEN** both `Fog.png` and `Fog.jpg` exist and the image `Fog` is looked up
- **THEN** `Fog.png` is returned

#### Scenario: Name that contains a dot
- **WHEN** the image `Ver1.5` is looked up and `Ver1.5.png` exists
- **THEN** `Ver1.5.png` is returned

### Requirement: Lookup failure
A lookup that finds nothing SHALL fail with an error that carries the path exactly as it was
requested. Reading a resolved file SHALL return its complete contents or an error naming the
file.

#### Scenario: Missing file
- **WHEN** `Graphics/Pictures/NoSuchPicture` is looked up
- **THEN** the error contains `Graphics/Pictures/NoSuchPicture`

#### Scenario: Read
- **WHEN** an existing audio file is looked up and read
- **THEN** the bytes returned equal the file's contents
