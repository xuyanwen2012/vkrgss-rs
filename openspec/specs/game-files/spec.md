# game-files Specification

## Purpose
Defines how vkxp finds the files of a game: reading the game configuration file, locating the
run-time packages it names, and resolving the paths scripts use to real files on a
case-sensitive file system.

## Requirements

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

### Requirement: Damaged archive
An archive whose header is not that of RGSSAD version 1, or whose entry table runs past the end
of the file, SHALL make opening the game's files fail with an error that names the archive.

#### Scenario: Wrong header
- **WHEN** `Game.rgssad` starts with other bytes than the RGSSAD version 1 header
- **THEN** opening the game's files fails and the error names the file

#### Scenario: Truncated
- **WHEN** an entry declares more bytes than the file has left
- **THEN** opening the game's files fails and the error names the file

### Requirement: Arbitrary archive bytes
Opening an archive and reading its entries SHALL end in a result or an error for arbitrary
input bytes, without a panic, and without allocating for a declared length that exceeds the
bytes present.

#### Scenario: Random tail
- **WHEN** a valid header is followed by arbitrary bytes
- **THEN** opening returns a directory or an error, and reading each listed entry returns bytes or an error

#### Scenario: Damaged valid archive
- **WHEN** a valid archive has bytes flipped or is cut at an arbitrary length
- **THEN** opening and reading return results or errors and do not panic

#### Scenario: Huge declared length
- **WHEN** an entry declares a length of four gigabytes in a file of one kilobyte
- **THEN** an error is returned

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

### Requirement: Lookups see changes made during the run
A lookup in the game directory or a run-time package SHALL find a file or directory that was
made, and SHALL NOT find one that was removed or renamed away, since an earlier lookup in the
same directory, whoever made the change and without a call that announces it.

#### Scenario: New file in a directory already looked into
- **WHEN** a lookup of `Graphics/Pictures/a` has succeeded, a file `Graphics/Pictures/b.png` is then written by other code, and `Graphics/Pictures/b` is looked up
- **THEN** it is found

#### Scenario: New directory
- **WHEN** a lookup of `Save/1` has failed because `Save` does not exist, the directory and a file `Save/1.cdat` are then made, and `Save/1.cdat` is looked up
- **THEN** it is found

#### Scenario: Removed file
- **WHEN** a file that a lookup found is deleted and looked up again
- **THEN** the lookup fails with the error for a missing file

#### Scenario: A picture a script wrote
- **WHEN** a script of a generated game evaluates `Dir.mkdir("Save")`, writes the bytes of a PNG picture to `Save/1.cdat` with `File.open("Save/1.cdat", "wb")`, and then `Bitmap.new("Save/1.cdat")`
- **THEN** a bitmap of the picture's size is returned
