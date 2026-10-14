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
and the one value `RTP` for RGSS2 and for RGSS3. They SHALL be resolved to directories through
a table supplied by the caller, skipping empty names. A name that the table does not contain,
or whose directory does not exist, SHALL be an error naming it.

#### Scenario: Known package
- **WHEN** `RTP1` is `Standard` and the table maps `Standard` to an existing directory
- **THEN** that directory is the first run-time package directory

#### Scenario: Package of VX
- **WHEN** the runtime is RGSS2, `RTP` is `RPGVX`, `RTP1` is `Standard`, and the table maps only `RPGVX`
- **THEN** the directory of `RPGVX` is the only run-time package directory

#### Scenario: Package of VX Ace
- **WHEN** the runtime is RGSS3, `RTP` is `RPGVXAce`, `RTP1` is `Standard`, and the table maps only `RPGVXAce`
- **THEN** the directory of `RPGVXAce` is the only run-time package directory

#### Scenario: Game of VX Ace without a package
- **WHEN** the runtime is RGSS3 and `RTP` is empty
- **THEN** there is no run-time package directory and no error

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
`ogg`, `wav`, `mp3`, `wma` for audio, and `ogg`, `mid`, `mp3`, `wav`, `wma` for the audio of
a game of RGSS3; none for data. Within one directory a file matching the path as given SHALL
win over one found by appending an extension.

#### Scenario: Extension omitted
- **WHEN** the image `Graphics/Titles/001-Title01` is looked up and only `001-Title01.jpg` exists
- **THEN** that file is returned

#### Scenario: Order of extensions
- **WHEN** both `Fog.png` and `Fog.jpg` exist and the image `Fog` is looked up
- **THEN** `Fog.png` is returned

#### Scenario: Name that contains a dot
- **WHEN** the image `Ver1.5` is looked up and `Ver1.5.png` exists
- **THEN** `Ver1.5.png` is returned

#### Scenario: Audio of RGSS3, measured on the original
- **WHEN** `Theme` exists with all five audio extensions, and `Pair` in turn with each two of `mid`, `mp3`, `wav` and `wma`, and each name is looked up as audio of a game of RGSS3
- **THEN** `Theme.ogg` is returned, and for each pair the file whose extension comes first in `mid`, `mp3`, `wav`, `wma`

#### Scenario: Audio of the other runtimes
- **WHEN** `Theme` exists with all five audio extensions and is looked up as audio
- **THEN** `Theme.mid` is returned

#### Scenario: Audio of RGSS3 in a package and in the archive
- **WHEN** only the run-time package holds `Audio/BGM/Theme.ogg`, and in another game only the encrypted archive holds `Audio\SE\Hit.ogg`, and both names are read without extension as audio of a game of RGSS3
- **THEN** the bytes of each are returned

#### Scenario: Other kinds with the audio of RGSS3
- **WHEN** `Fog.png` exists and `Fog` is looked up as audio of a game of RGSS3
- **THEN** the lookup fails

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
When the game directory holds the archive of the game's runtime (`Game.rgssad`, `Game.rgss2a`
or `Game.rgss3a`, name compared without regard to case), the game's files SHALL include its
entries, decrypted: RGSSAD version 1 for RGSS1 and RGSS2, version 3 for RGSS3. Entry names SHALL be
matched like loose file names. An entry SHALL take precedence over a loose file of the same
name, and loose files over the RTPs. The archive of another runtime SHALL be ignored.

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

#### Scenario: Archive of VX Ace
- **WHEN** the runtime is RGSS3 and a generated `Game.RGSS3A` of version 3 holds `Data\Scripts.rvdata2` and `Graphics\System\Window.png`
- **THEN** reading the data file `Data/Scripts.rvdata2` returns that entry's decrypted bytes, and reading the image `graphics/system/window` returns the other's

#### Scenario: Precedence in a game of VX Ace
- **WHEN** the runtime is RGSS3 and `Game.rgss3a` and the game directory both hold `Data/Scripts.rvdata2` with different contents
- **THEN** the archive's contents are returned

#### Scenario: Other archives beside VX Ace
- **WHEN** the runtime is RGSS3 and the directory holds only `Game.rgss2a`, or the runtime is RGSS2 and the directory holds only `Game.rgss3a`
- **THEN** files resolve as if there were no archive

### Requirement: Damaged archive
An archive whose header is not that of the RGSSAD version its runtime reads (version 1 for
`Game.rgssad` and `Game.rgss2a`, version 3 for `Game.rgss3a`), whose entry table runs past
the end of the file, or one of whose entries declares data that the file does not hold, SHALL
make opening the game's files fail with an error that names the archive.

#### Scenario: Wrong header
- **WHEN** `Game.rgssad` starts with other bytes than the RGSSAD version 1 header
- **THEN** opening the game's files fails and the error names the file

#### Scenario: Truncated
- **WHEN** an entry declares more bytes than the file has left
- **THEN** opening the game's files fails and the error names the file

#### Scenario: Wrong header of VX Ace
- **WHEN** the runtime is RGSS3 and `Game.rgss3a` starts with the RGSSAD version 1 header, or with other bytes than the version 3 header, or is shorter than twelve bytes
- **THEN** opening the game's files fails and the error names the file

#### Scenario: Table of version 3 without an end
- **WHEN** a valid `Game.rgss3a` is cut inside a record of its table, or right before the word that ends the table
- **THEN** opening the game's files fails and the error names the file

#### Scenario: Entry of version 3 outside the file
- **WHEN** a record of `Game.rgss3a` gives an offset and a size whose sum is larger than the length of the file
- **THEN** opening the game's files fails and the error names the file

### Requirement: Arbitrary archive bytes
Opening an archive of either version and reading its entries SHALL end in a result or an
error for arbitrary input bytes, without a panic, and without allocating for a declared length
that exceeds the bytes present.

#### Scenario: Random tail
- **WHEN** a valid header is followed by arbitrary bytes
- **THEN** opening returns a directory or an error, and reading each listed entry returns bytes or an error

#### Scenario: Damaged valid archive
- **WHEN** a valid archive has bytes flipped or is cut at an arbitrary length
- **THEN** opening and reading return results or errors and do not panic

#### Scenario: Huge declared length
- **WHEN** an entry declares a length of four gigabytes in a file of one kilobyte
- **THEN** an error is returned

#### Scenario: Random tail of version 3
- **WHEN** the header of version 3 is followed by arbitrary bytes and the file is opened as the archive of a game of RGSS3
- **THEN** opening returns a directory or an error, and reading each listed entry returns bytes or an error

#### Scenario: Damaged valid archive of version 3
- **WHEN** a valid archive of version 3 has bytes flipped or is cut at an arbitrary length
- **THEN** opening and reading return results or errors and do not panic

#### Scenario: Huge declared lengths in version 3
- **WHEN** a record of an archive of version 3 of one kilobyte declares a name of four gigabytes, or data of four gigabytes
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

### Requirement: Table of RGSSAD version 3
An archive of RGSSAD version 3 SHALL start with the bytes `RGSSAD`, 0, 3 and a 32-bit
little-endian seed; its table key SHALL be seed * 9 + 3 in 32 bits, the same for every
record. Each record SHALL be four little-endian words XORed with that key (offset of the
data in the file, size, file key, length of the name), then the UTF-8 name, byte i XORed
with byte i mod 4 of the key. The table SHALL end at the record whose first word decrypts
to 0, and nothing after that word SHALL be read as table.

#### Scenario: Worked example
- **WHEN** `Game.rgss3a` consists of the 38 bytes `52 47 53 53 41 44 00 03`, `01 00 00 00`, `2D 00 00 00`, `09 00 00 00`, `25 00 00 00`, `0D 00 00 00`, `4D`, `0C 00 00 00`, `48 62 63 64 47`
- **THEN** it holds one entry, named `A`, whose contents are the five bytes `abcde`

#### Scenario: Keys measured on the original
- **WHEN** an archive is generated with the seed 0x2A38 and three entries whose file keys are 41, 18467 and 6334, or with the seed 0x350A and the same file keys
- **THEN** its table is encrypted with the key 0x17BFB or 0x1DD5D respectively, and each entry is read back with the contents it was given

#### Scenario: End record of the editor
- **WHEN** the word that ends the table is followed by twelve arbitrary bytes before the data, as the editor writes it, or by the data at once
- **THEN** the same entries are found with the same contents in both archives

#### Scenario: Name that is not ASCII
- **WHEN** an archive holds an entry named `Graphics\Titles\标题.png`
- **THEN** the image `graphics/titles/标题` is read from it

### Requirement: Entry data of RGSSAD version 3
The data of an entry of an RGSSAD version 3 archive SHALL be read at the offset its record
gives and XORed in little-endian 32-bit words with a key that starts at the entry's file key
and becomes key * 7 + 3 in 32 bits after every word; a last word of fewer than four bytes
SHALL use the low bytes of the key.

#### Scenario: Sizes
- **WHEN** an archive holds entries of 0, 1, 5 and 70001 bytes
- **THEN** every entry is read back byte for byte

#### Scenario: Data in another order than the table
- **WHEN** the data of the second record lies before the data of the first in the file
- **THEN** both entries are read back with their own contents

### Requirement: Version of the library of RGSS3
The version of a runtime DLL SHALL be readable from the bytes of the file: the four numbers
of the file version in its version record, found by the record's signature 0xFEEF04BD
followed by the structure version 0x00010000, all little-endian. Bytes without such a record,
or that end inside it, SHALL give no version and no error. The file name SHALL NOT be used.

#### Scenario: Version 3.0.0.1
- **WHEN** bytes are read that hold, after other bytes, the words 0xFEEF04BD, 0x00010000, 0x00030000 and 0x00000001
- **THEN** the version is 3.0.0.1

#### Scenario: Version 3.0.1.1
- **WHEN** the last of those words is 0x00010001
- **THEN** the version is 3.0.1.1

#### Scenario: No record
- **WHEN** an empty file, a file of 1000 zero bytes and a file that ends 3 bytes after the signature are read
- **THEN** each gives no version

#### Scenario: Signature without the structure version
- **WHEN** the signature is followed by another word than 0x00010000 and a full record comes later in the bytes
- **THEN** the version is that of the later record
