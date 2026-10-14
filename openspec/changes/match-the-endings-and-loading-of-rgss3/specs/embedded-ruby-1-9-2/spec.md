## ADDED Requirements

### Requirement: An interpreter without a script name
The interpreter crate SHALL start an interpreter whose options have an empty script name
without giving it a script name: `$0` SHALL then be `false` and the frame of a C function
at the bottom of a backtrace SHALL be named `ruby`. The options for a game of RGSS3 SHALL
have an empty script name; the default options and those for RGSS1 and RGSS2 SHALL keep
`vkxp`.

#### Scenario: Options for RGSS3
- **WHEN** an interpreter started with `Options::for_rgss(3)` evaluates `[$0, $PROGRAM_NAME]` and the last entry of `caller(0)` inside a string given to `eval` by the crate's evaluation helper
- **THEN** the results are `[false, false]` and an entry that starts with ``ruby:in `eval'``

#### Scenario: Default options
- **WHEN** an interpreter started with the default options evaluates `$0`
- **THEN** the result is `"vkxp"`, and the existing tests of the crate pass unedited
