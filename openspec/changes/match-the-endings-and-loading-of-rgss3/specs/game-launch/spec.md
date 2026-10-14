## ADDED Requirements

### Requirement: Title of a game of RGSS3
The title of a game of RGSS3 whose `Game.ini` has no `Title=` value, or an empty one, SHALL
be `Untitled`, for the window the program opens and for the title the guest sends, as
measured on the original runtime. `Game.ini` itself SHALL not be changed, and games of
RGSS1 and RGSS2 SHALL keep their titles.

#### Scenario: Without a title, measured on the original
- **WHEN** the inputs of a guest are read for a game directory whose `Game.ini` names `System\RGSS301.dll` and `Data\Scripts.rvdata2` and has no `Title=`
- **THEN** the title of the configuration is `Untitled`, and the file is unchanged

#### Scenario: Empty value
- **WHEN** the same `Game.ini` has `Title=` without a value
- **THEN** the title of the configuration is `Untitled`

#### Scenario: XP and VX
- **WHEN** the inputs are read for games of RGSS1 and RGSS2 without `Title=`
- **THEN** their titles are empty, as before
