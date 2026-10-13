## REMOVED Requirements

### Requirement: Experimental runtime
**Reason**: The milestones V1 to V3 of RPG Maker VX are done, so VX is supported and no run says otherwise.
**Migration**: None. A run of RGSS2 writes one diagnostic line less.

## ADDED Requirements

### Requirement: Supported runtimes say nothing about their state
A run of RGSS1 or of RGSS2 SHALL NOT say that support for its runtime is experimental, and
the usage message SHALL name `rgss1` and `rgss2` without such a word.

#### Scenario: VX
- **WHEN** a generated game of RGSS2 whose only script returns at once is run headless
- **THEN** the diagnostics hold no line with the word "experimental", and the exit status is 0

#### Scenario: Usage
- **WHEN** the usage message is read
- **THEN** the line of `--runtime` says of `rgss2` only that it is RPG Maker VX, and of `rgss3` that it is not supported yet
