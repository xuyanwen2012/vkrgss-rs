## ADDED Requirements

### Requirement: Frame size of the run
The program SHALL give the host the frame size of the run's runtime as the size the window
starts with, and a frame dumped by a headless run SHALL have the size of the game's frame.

#### Scenario: Dump of a game of VX
- **WHEN** a generated game of RGSS2 that calls `Graphics.update` runs headless with a dump of its first frame
- **THEN** the written picture is 544 by 416

#### Scenario: Dump of a game of XP
- **WHEN** a generated game of RGSS1 runs the same way
- **THEN** the written picture is 640 by 480
