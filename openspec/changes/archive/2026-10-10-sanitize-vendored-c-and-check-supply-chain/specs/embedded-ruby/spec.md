# Spec Delta

## ADDED Requirements

### Requirement: Upstream suites under sanitizers
The repository SHALL provide an opt-in run that builds the interpreter and the regular
expression engine with AddressSanitizer and UndefinedBehaviorSanitizer and runs the upstream
test suites under them. The run SHALL fail on a sanitizer report that is not in a committed
allow-list whose entries each state a reason. The run SHALL NOT be part of the merge gate.

#### Scenario: Sanitized run
- **WHEN** the opt-in run is started on the development machine
- **THEN** the upstream suites pass and the sanitizers report nothing outside the allow-list

#### Scenario: Not in the gate
- **WHEN** the merge gate runs
- **THEN** the sanitized build is not made
