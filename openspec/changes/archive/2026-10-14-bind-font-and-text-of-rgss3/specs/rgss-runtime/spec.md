## ADDED Requirements

### Requirement: Version of RGSS3 by the library of the game
In a game of RGSS3 `RGSS_VERSION` SHALL be `"3.0.0"` when the DLL the game's `Library`
names has a version whose first three numbers are 3.0.0, and `"3.0.1"` for any other
version, a DLL without a version record and no DLL, in ASCII-8BIT and defined once. This
SHALL hold in the place of the version that the requirement "Preparation of a game of
RGSS3" states.

#### Scenario: Versions, measured on the original
- **WHEN** a game of RGSS3 is run with the library versions 3.0.0.1 and 3.0.1.1 and evaluates `[RGSS_VERSION, RGSS_VERSION.encoding.name]`
- **THEN** the results are `["3.0.0", "ASCII-8BIT"]` and `["3.0.1", "ASCII-8BIT"]`

#### Scenario: The table of the decision
- **WHEN** a game of RGSS3 is run with the library version 3.0.0.7, with 2.0.2.1 and without a version
- **THEN** `RGSS_VERSION` is `"3.0.0"`, `"3.0.1"` and `"3.0.1"`, and no warning of a constant defined twice is written

#### Scenario: Through both executables
- **WHEN** a generated game of RGSS3 whose `Library=System\RGSS300.dll` is a file holding a version record is run headless by `vkxp`, once with 3.0.0.1 and once with 3.0.1.1, and reports `RGSS_VERSION`
- **THEN** it reports `"3.0.0"` and `"3.0.1"`

#### Scenario: Before this change
- **WHEN** the tests of the preparation of RGSS3 of `run-rgss3-games-in-a-guest-of-their-own` run with their configuration, which names no library version
- **THEN** they pass unedited, `RGSS_VERSION` being `"3.0.1"`
