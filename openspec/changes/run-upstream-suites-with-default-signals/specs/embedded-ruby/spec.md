# Spec Delta

## ADDED Requirements

### Requirement: Upstream suites do not depend on the caller's signal dispositions
The harness that runs the upstream test suites SHALL start them with the default disposition
of the signals those suites send to themselves.

#### Scenario: SIGINT ignored by the caller
- **WHEN** the upstream suites are run from a process that ignores SIGINT
- **THEN** they pass as they do from a terminal

#### Scenario: Gate without a terminal
- **WHEN** `cargo xtask gate` is started with SIGINT ignored
- **THEN** its test step passes
