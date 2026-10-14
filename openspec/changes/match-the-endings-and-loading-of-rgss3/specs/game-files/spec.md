## ADDED Requirements

### Requirement: Data files of RGSS3
The game's files SHALL find the data file a game of RGSS3 names in the archive first, by
the name as given or by an entry's name without its extension, without regard to letter
case, unless a part of the name is empty, `.` or `..`, or the name ends with a dot or a
space; else as a loose file from the current directory, inside the game directory only,
after dropping trailing dots and spaces. Other lookups SHALL not change.

#### Scenario: Archive first, measured on the original
- **WHEN** the archive and the game directory both hold `Data/both.rvdata2`, and a loose file is written under that name during the run
- **THEN** `Data/both.rvdata2`, `Data\both.rvdata2` and `DATA/BOTH.rvdata2` give the bytes of the entry

#### Scenario: Without the extension, measured on the original
- **WHEN** the archive holds `Data\arconly.rvdata2`, `Other\x.bin` and `Data\noext`
- **THEN** `Data/arconly`, `Data\arconly`, `data/ARCONLY.RVDATA2`, `Other/x` and `Data/noext` find the entries, and `Data/arconly.rvdata`, `Data/arconly.rvdata2.rvdata2`, `Data/arconly.zzz`, `Data/arconl`, `Data/arconly.` and `Data/noext.rvdata2` find nothing

#### Scenario: Names the archive is not asked for, measured on the original
- **WHEN** the archive holds `Data\arconly.rvdata2` and `Data\both.rvdata2` and the game directory holds `Data/both.rvdata2` only
- **THEN** `./Data/arconly.rvdata2`, `Data//arconly.rvdata2`, `Data/./arconly.rvdata2`, `Data/Sub/../arconly.rvdata2` and the absolute path of the game directory joined with `Data/arconly.rvdata2` find nothing, and the same forms of `both` find the loose file

#### Scenario: No extension for a loose file, measured on the original
- **WHEN** the game directory holds `d/p.bin` and no archive holds it
- **THEN** `d/p` finds nothing, and `d/p.bin`, `d\p.bin`, `./d/p.bin`, `D/P.BIN`, `d//p.bin`, `d/../d/p.bin`, `d/p.bin.` and `d/p.bin ` find it, and ` d/p.bin` does not

#### Scenario: The current directory, measured on the original
- **WHEN** the current directory of the process is `d` of the game directory
- **THEN** `p.bin` finds `d/p.bin`, `d/p.bin` finds nothing, and `../Game.ini` finds `Game.ini` of the game directory

#### Scenario: The current directory does not move the archive, measured on the original
- **WHEN** the current directory is `Data` and the archive holds `Data\arconly.rvdata2`
- **THEN** `Data/arconly.rvdata2` finds the entry and `arconly.rvdata2` finds nothing

#### Scenario: A directory, measured on the original
- **WHEN** `Data` is a directory of the game and `Data` is looked up
- **THEN** the lookup fails as a directory, not as a missing file

#### Scenario: Outside the game directory
- **WHEN** `../other.rvdata2` is looked up from the game directory, or an absolute path outside it
- **THEN** nothing is found

#### Scenario: The other runtimes and kinds
- **WHEN** the existing tests of the game's files run
- **THEN** they pass unedited
