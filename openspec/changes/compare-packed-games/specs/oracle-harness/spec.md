# Spec Delta

## ADDED Requirements

### Requirement: Games with an encrypted archive
When the game directory holds `Game.rgssad`, the harness SHALL add its probe to the script
archive stored inside it and write the changed archive into each copy of the game, leaving
every other entry byte for byte as it was and adding no loose script archive. The given game
directory SHALL stay untouched.

#### Scenario: Probe inside the archive
- **WHEN** a generated game whose `Data` folder exists only inside `Game.rgssad` is prepared for a run
- **THEN** the copy's archive holds the probed script archive, every other entry is unchanged, and the copy has no loose `Data` folder

#### Scenario: Packed game runs in vkxp
- **WHEN** the harness runs a packed generated game with `--vkxp-only` and asks for frames 10 and 20
- **THEN** the two pictures equal those of the same game run unpacked

### Requirement: Packing a game copy
`cargo xtask pack-rgssad <directory>` SHALL write a `Game.rgssad` of version 1 holding every
file below the directory's `Data` and `Graphics` folders under its path with backslashes, and
SHALL then remove those two folders. It SHALL refuse a directory that already has an archive
or has no `Game.ini`.

#### Scenario: Round trip
- **WHEN** a generated game is packed and its archive is read back
- **THEN** every file of the two folders is found under its backslash path with its bytes, and the folders are gone

#### Scenario: Already packed
- **WHEN** the directory already holds `Game.rgssad`
- **THEN** the command exits with a non-zero status and changes nothing
