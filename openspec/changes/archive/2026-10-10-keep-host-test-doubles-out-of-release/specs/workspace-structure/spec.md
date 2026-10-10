# Spec Delta

## ADDED Requirements

### Requirement: Test doubles stay out of the release library
The test doubles and the demo guest of `vkxp-host` SHALL be compiled only for tests and
examples, behind a feature that the executable does not enable.

#### Scenario: Executable build
- **WHEN** the feature graph of the `vkxp` package is listed
- **THEN** the `testing` feature of `vkxp-host` is not in it

#### Scenario: Tests and examples
- **WHEN** `cargo test -p vkxp-host --locked` and `cargo build -p vkxp-host --examples --locked` run
- **THEN** both succeed without a feature flag on the command line, or with the one the gate passes
