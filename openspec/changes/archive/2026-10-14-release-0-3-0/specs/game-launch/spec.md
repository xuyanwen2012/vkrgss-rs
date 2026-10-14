## REMOVED Requirements

### Requirement: Experimental runtime says so
**Reason**: The milestones A1 to A3 of RPG Maker VX Ace are done, so VX Ace is supported and no run says otherwise.
**Migration**: None. A run of RGSS3 writes one diagnostic line less.

## MODIFIED Requirements

### Requirement: Supported runtimes say nothing about their state
A run of RGSS1, RGSS2 or RGSS3 SHALL NOT say that support for its runtime is experimental,
and the usage message SHALL name `rgss1`, `rgss2` and `rgss3` without such a word.

#### Scenario: VX
- **WHEN** a generated game of RGSS2 whose only script returns at once is run headless
- **THEN** the diagnostics hold no line with the word "experimental", and the exit status is 0

#### Scenario: VX Ace
- **WHEN** a generated game of RGSS3 whose only script returns at once is run headless
- **THEN** the diagnostics hold no line with the word "experimental", and the exit status is 0

#### Scenario: Usage
- **WHEN** the usage message is read
- **THEN** the line of `--runtime` says of `rgss2` only that it is RPG Maker VX, and of `rgss3` only that it is RPG Maker VX Ace
