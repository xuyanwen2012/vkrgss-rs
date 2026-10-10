# Spec Delta

## ADDED Requirements

### Requirement: Large unit test modules of core live beside the module
In `vkxp-core`, a unit test module of more than about 300 lines SHALL be a file
`src/<module>/tests.rs` declared from the module with `#[cfg(test)] mod tests;`, instead of an
inline module.

#### Scenario: Frame extraction
- **WHEN** `crates/vkxp-core/src/extract.rs` is read
- **THEN** it holds the production code and the declaration of its test module, and the tests are in `crates/vkxp-core/src/extract/tests.rs`

#### Scenario: The same tests run
- **WHEN** the names printed by `cargo test -p vkxp-core -- --list` before and after the move are compared
- **THEN** the two lists are equal
