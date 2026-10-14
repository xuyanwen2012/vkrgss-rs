## ADDED Requirements

### Requirement: Fonts of a game of RGSS3
For a game of RGSS3 the text engine SHALL take the fonts of the directories named with
`--font-dir`, then of the game's `Fonts` directory, then of the `Fonts` directory of each
run-time package of the game in the order of the packages, each when it exists, then the
fonts installed on the system. For RGSS1 and RGSS2 the list SHALL stay that of the
requirement "Fonts".

#### Scenario: The order of the directories
- **WHEN** the font directories are made for a game of RGSS3 with one `--font-dir`, a game `Fonts` directory and two run-time packages of which only the second has a `Fonts` directory
- **THEN** they are the `--font-dir`, the game's `Fonts` and the second package's `Fonts`, in this order

#### Scenario: VX
- **WHEN** the font directories are made for a game of RGSS2 with the same directories
- **THEN** they are the `--font-dir` and the game's `Fonts`, as before

#### Scenario: A font of the run-time package
- **WHEN** a text backend is made for a game of RGSS3 whose run-time package has a `Fonts` directory holding a copy of a font of the test fonts of `vkxp-text`
- **THEN** that font's family name exists exactly in the backend

### Requirement: Glyph rule by the library of a game of RGSS3
The guest of a game of RGSS3 SHALL read the version record of the file its `Library` names,
found in the game directory as a data file is, and SHALL set the mask rules of its text
engine to those of 3.0.0.1 for a version whose first three numbers are 3.0.0, and to those
of 3.0.1.1 for any other version, a file without a record, and no file. RGSS1 and RGSS2
SHALL keep the plain rules. The guest record SHALL not change.

#### Scenario: The table of the decision
- **WHEN** the mask rules are chosen for a game of RGSS3 with the versions 3.0.0.1, 3.0.0.7, 3.0.1.1 and 2.0.2.1 and without a version, and for a game of RGSS2 with 3.0.0.1
- **THEN** they are `Rgss3Early`, `Rgss3Early`, `Rgss3`, `Rgss3`, `Rgss3` and `Plain`

#### Scenario: The file named by the configuration
- **WHEN** the guest reads the version of a game whose `Game.ini` has `Library=System\RGSS300.dll` and whose file `system/rgss300.dll` holds a version record 3.0.0.1
- **THEN** the version is 3.0.0.1

#### Scenario: No version
- **WHEN** the guest reads the version of games with an empty `Library`, with a `Library` that names no file, and with a file of 1000 zero bytes
- **THEN** each gives no version, the guest goes on, and its log names the reason in one line

#### Scenario: The name of the file does not count
- **WHEN** the guest reads `System\RGSS300.dll` holding a version record 3.0.1.1
- **THEN** the version is 3.0.1.1 and the mask rules are `Rgss3`
