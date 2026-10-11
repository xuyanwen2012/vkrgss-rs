## ADDED Requirements

### Requirement: The reset stays the current error
While the scripts are evaluated again after a reset, `$!` SHALL be the `Reset` exception
that ended the previous run and `$@` its backtrace, until a script replaces them. Before the
first run both SHALL be nil.

#### Scenario: First run
- **WHEN** the first script records `$!` and `$@` at its start
- **THEN** both are nil

#### Scenario: After a reset
- **WHEN** the host sends a reset request during `Graphics.update` and the first script records `$!.class` and `$@` when it is evaluated again
- **THEN** the class is `Reset` and `$@` is an array whose first entry names `update`

#### Scenario: Guarded alias
- **WHEN** a script wraps `Sprite#dispose` with `alias old_dispose dispose unless $@`, the game is reset twice and a sprite is then disposed
- **THEN** the sprite is disposed without an error

#### Scenario: Rescue inside the second run
- **WHEN** a script of the run after a reset raises and rescues an error of its own and a later script records `$!.class`
- **THEN** the class is `Reset`
