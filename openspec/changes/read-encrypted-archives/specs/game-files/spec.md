# Spec Delta

## ADDED Requirements

### Requirement: Encrypted archive
When the game directory holds a file `Game.rgssad` (name compared without regard to case), the
game's files SHALL include the entries of that RGSSAD version 1 archive, decrypted. Entry names
SHALL be matched like loose file names: without regard to case, with `/` and `\` equivalent,
and with the extension search of the kind asked for. An entry SHALL take precedence over a
loose file of the same name, and loose files over the RTPs.

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

### Requirement: Damaged archive
An archive whose header is not that of RGSSAD version 1, or whose entry table runs past the end
of the file, SHALL make opening the game's files fail with an error that names the archive.

#### Scenario: Wrong header
- **WHEN** `Game.rgssad` starts with other bytes than the RGSSAD version 1 header
- **THEN** opening the game's files fails and the error names the file

#### Scenario: Truncated
- **WHEN** an entry declares more bytes than the file has left
- **THEN** opening the game's files fails and the error names the file
