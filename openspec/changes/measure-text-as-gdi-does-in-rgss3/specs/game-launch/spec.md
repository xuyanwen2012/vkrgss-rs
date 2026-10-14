## ADDED Requirements

### Requirement: Metric rule of a game of RGSS3
The guest of a game of RGSS3 SHALL set the metric rule of its text engine to that of RGSS3,
whatever version the DLL of the game has. A game of RGSS1 or RGSS2 SHALL keep the plain
metric rule. The mask rules SHALL stay as "Glyph rule by the library of a game of RGSS3"
sets them, and the guest record SHALL not change.

#### Scenario: The table
- **WHEN** the metric rule is chosen for a game of RGSS3 with the versions 3.0.0.1 and 3.0.1.1 and without a version, and for games of RGSS1 and RGSS2
- **THEN** it is the rule of RGSS3 for the first three and the plain rule for the last two

#### Scenario: The backend of a game
- **WHEN** a text backend is made for a game of RGSS3 and for a game of RGSS2 over the test fonts of `vkxp-text`
- **THEN** its engine has the metric rule of RGSS3 in the first and the plain rule in the second
