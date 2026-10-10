# Spec Delta

## ADDED Requirements

### Requirement: Diagnostics
Diagnostics of the running program SHALL be events with a level, written to the error stream.
By default warnings and errors SHALL be shown, each as a line that starts with `vkxp: `. Text
the game produces and the program's usage and fatal error message SHALL be shown at every
level.

#### Scenario: Default level
- **WHEN** a game runs without `--verbose` and without `VKXP_LOG`
- **THEN** the error stream has no informational line

#### Scenario: Script error at every level
- **WHEN** a game whose script raises runs with `VKXP_LOG=error`
- **THEN** the error stream still carries the script error report

### Requirement: Diagnostic level
The option `--verbose` SHALL also show informational and debug events. The environment
variable `VKXP_LOG` SHALL set the level or a per-module filter and take precedence over
`--verbose`; a value that cannot be parsed SHALL end the program with a message and a non-zero
status.

#### Scenario: Verbose
- **WHEN** the same game runs headless with `--verbose`
- **THEN** the error stream names the built-in scripts file that was loaded

#### Scenario: Level from the environment
- **WHEN** the game runs with `VKXP_LOG=error` and `--verbose`
- **THEN** the error stream has no informational line

#### Scenario: Malformed filter
- **WHEN** the program runs with `VKXP_LOG` set to `=`
- **THEN** it ends with a message naming the variable and a non-zero status
