## ADDED Requirements

### Requirement: Stack exhaustion by nesting is set aside in a fuzz run
A run of a fuzz target SHALL count a stack overflow that the sanitizer reports in one of the
recursive functions listed for that target like a limit: the input is put aside, the run goes
on, and the summary counts it. A stack overflow elsewhere SHALL be a finding.

#### Scenario: Deep nesting in a pattern
- **WHEN** a process of the target `regex` ends with an AddressSanitizer stack overflow whose first frame is `get_min_match_length`
- **THEN** the run counts a limit of the stack, keeps the input, and continues

#### Scenario: Another stack overflow
- **WHEN** a process ends with a stack overflow whose first frames are in no listed function
- **THEN** the run fails as for a finding
