# Spec Delta

## ADDED Requirements

### Requirement: Dependency audit command
`cargo xtask audit` SHALL check the dependency graph against security advisories and a
committed licence allow-list. It SHALL NOT be a step of the merge gate. When its tool is not
installed it SHALL say how to install it and exit with a non-zero status.

#### Scenario: Tool missing
- **WHEN** `cargo xtask audit` runs without `cargo-deny` installed
- **THEN** it prints how to install it and exits with a non-zero status

#### Scenario: Gate unchanged
- **WHEN** the merge gate runs
- **THEN** it does not run the audit
