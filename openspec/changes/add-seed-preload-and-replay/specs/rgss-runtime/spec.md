# Spec Delta

## ADDED Requirements

### Requirement: Random seed
When the caller gives a seed, the random number generator SHALL be seeded with it before the
built-in scripts run and again on every reset, so that two runs with the same seed and input
produce the same sequence of `rand` results.

#### Scenario: Same seed
- **WHEN** a game that writes ten values of `rand(1000)` to a file is run twice with seed 7
- **THEN** both files are equal

#### Scenario: Different seeds
- **WHEN** it is run with seed 7 and with seed 8
- **THEN** the files differ

### Requirement: Preload scripts
The files the caller names as preload scripts SHALL be evaluated in the given order after the
built-in scripts and before the game's first script, and again after a reset. An error in a
preload script SHALL end the run like an uncaught script error, with the file's name in the
report. A file that does not exist SHALL fail the run before any script.

#### Scenario: Patch before the game
- **WHEN** a preload script defines a constant and the game's first script reads it
- **THEN** the game sees the constant

#### Scenario: Error in a preload script
- **WHEN** a preload script raises on its line 2
- **THEN** the report names that file and line 2, and no game script runs
