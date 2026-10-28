## ADDED Requirements

### Requirement: Source layout of the binding
No file under `crates/vkrgss-binding/src` SHALL be longer than 1000 lines unless it holds
only tests. Comment lines in a prelude that only keep line numbers SHALL name what reads
those numbers; padding that nothing reads SHALL NOT exist.

#### Scenario: File lengths
- **WHEN** the lines of every file under `crates/vkrgss-binding/src` are counted
- **THEN** each file longer than 1000 lines holds only tests

#### Scenario: Prelude positions
- **WHEN** a prelude file has comment lines that keep the line numbers of what follows
- **THEN** a comment next to them names the test, report or comparison that reads those numbers

#### Scenario: Nothing else changes
- **WHEN** the workspace is built and tested and `cargo xtask xarch check` runs after the move
- **THEN** every crate compiles unchanged, the tests that passed before pass, none removed, and every game is identical to the baseline
