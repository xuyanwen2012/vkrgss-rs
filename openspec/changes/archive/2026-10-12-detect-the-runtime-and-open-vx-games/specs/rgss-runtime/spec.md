## MODIFIED Requirements

### Requirement: Interpreter preparation
Before any game script runs, `$KCODE` SHALL be `"UTF8"`, the load path SHALL be empty and the
window title SHALL have been sent to the host from the game configuration file. In RGSS1
`$DEBUG` SHALL be `true` in debug mode and `$BTEST` in battle-test mode. In RGSS2 `$TEST`
SHALL be `true` in debug mode, `$TEST` and `$BTEST` in battle-test mode, and `$DEBUG` SHALL
stay `false`. Otherwise these globals SHALL be `false`.

#### Scenario: Globals
- **WHEN** the first game script evaluates `[$KCODE, $:, $DEBUG, $BTEST]`
- **THEN** the result is `["UTF8", [], false, false]`

#### Scenario: Debug mode
- **WHEN** the caller asks for debug mode
- **THEN** `$DEBUG` is `true` in the first game script

#### Scenario: Title
- **WHEN** a game whose configuration file has the title `Demo` is started
- **THEN** the host receives a title message with `Demo` before the first frame

#### Scenario: Globals of VX, measured on the original
- **WHEN** the first script of a game of RGSS2 evaluates `[$TEST, $BTEST, $DEBUG]` in a plain run, in debug mode and in battle-test mode
- **THEN** the results are `[false, false, false]`, `[true, false, false]` and `[true, true, false]`
