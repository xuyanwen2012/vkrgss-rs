## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Encrypted archive
When the game directory holds the archive of the game's runtime, `Game.rgssad` for RGSS1,
`Game.rgss2a` for RGSS2 and `Game.rgss3a` for RGSS3 (name compared without regard to case),
the game's files SHALL include the entries of that archive, decrypted: an RGSSAD version 1
archive for RGSS1 and RGSS2, an RGSSAD version 3 archive for RGSS3. Entry names SHALL be
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
