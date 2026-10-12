## ADDED Requirements

### Requirement: Symbol numbers follow the runtime
The program SHALL start the interpreter with the start-up symbol table of the run's runtime,
so that the scripts of a game see the symbol numbers of the original runtime it was made for.

#### Scenario: First new names of a game of VX, measured on the original
- **WHEN** the first script of a game of RGSS2 evaluates `[:zz_first.to_i, :zz_second.to_i]`
- **THEN** the result is `[16089, 16097]`

#### Scenario: Known names of a game of VX, measured on the original
- **WHEN** a script of a game of RGSS2 evaluates `[:initialize.to_i, :to_s.to_i, :update.to_i]`
- **THEN** the result is `[2961, 3161, 6481]`

#### Scenario: XP is unchanged
- **WHEN** the existing test of the symbol numbers of a game of RGSS1 runs
- **THEN** it passes without a change to its expected numbers
