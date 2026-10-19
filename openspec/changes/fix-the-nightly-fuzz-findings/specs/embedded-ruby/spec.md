## ADDED Requirements

### Requirement: The parser keeps the collector's free list intact
Parsing any input, valid or not, SHALL NOT put an object on the collector's free list while it
is still referenced, and a stack exhausted by nesting in the regular expression engine SHALL
be set aside in every fuzz target that reaches the engine.

#### Scenario: The saved input
- **WHEN** the input of nightly run 39110302699 is replayed through the fuzz harness of `parse` under the sanitizers, alone and after the corpus
- **THEN** no report is made

#### Scenario: Deep pattern through Marshal
- **WHEN** a process of the target `marshal` ends with a stack overflow whose first frame is `get_min_match_length`
- **THEN** the run counts a limit of the stack and goes on
