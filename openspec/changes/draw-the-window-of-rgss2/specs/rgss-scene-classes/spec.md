## ADDED Requirements

### Requirement: Openness
In RGSS2 `Window#openness` SHALL default to 255 and `openness=` SHALL take an integer, a Float
being truncated, clamped to 0 to 255; on a disposed window both SHALL raise `RGSSError`. In
RGSS1 neither SHALL be defined.

#### Scenario: Limits, measured on the original
- **WHEN** a game of RGSS2 sets `openness` to -1, 256, 100000 and -1.7, reading it after each
- **THEN** the values are 0, 255, 255 and 0

#### Scenario: Errors, measured on the original
- **WHEN** a game of RGSS2 evaluates `w.openness = nil` and, on a disposed window, `w.openness`
- **THEN** a `TypeError` and an `RGSSError` are raised

#### Scenario: Not pending any more
- **WHEN** the method test of RGSS2 runs
- **THEN** `Window#openness` and `openness=` are checked and are not in its pending list

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Window.new.respond_to?(:openness)`
- **THEN** the result is `false`
