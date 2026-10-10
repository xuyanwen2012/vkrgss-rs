# Spec Delta

## ADDED Requirements

### Requirement: Direct dependencies follow their newest stable line
The workspace's direct dependencies SHALL allow the newest stable release line of each crate,
except where the change that last reviewed them recorded a reason to stay behind.

#### Scenario: The three upgraded crates
- **WHEN** `Cargo.toml` is read after this change
- **THEN** `sha2` allows 0.11 and `x11rb` allows 0.14, and `symphonia` allows 0.6 or the change's design records why it stays at 0.5

#### Scenario: Decoding unchanged
- **WHEN** the decoding tests of `vkxp-audio` run
- **THEN** they pass, with an expectation changed only where the design records an accepted difference
