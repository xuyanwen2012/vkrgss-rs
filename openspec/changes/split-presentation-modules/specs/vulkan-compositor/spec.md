## ADDED Requirements

### Requirement: Source layout of the backend
No file under `crates/vkxp-vk/src` SHALL be longer than 1000 lines unless it holds only
tests. The unit tests of the compositor, the presenter and the device SHALL live in files of
their own. In the non-test files of the compositor and the presenter, `#[cfg(test)]` SHALL
stand only on module declarations and whole `impl` blocks, and in the files of the test
probes.

#### Scenario: File lengths
- **WHEN** the lines of every file under `crates/vkxp-vk/src` are counted
- **THEN** each file longer than 1000 lines holds only tests

#### Scenario: Test attributes
- **WHEN** `#[cfg(test)]` is searched in `compositor.rs` and the non-test files under `compositor/`
- **THEN** every match outside a probe file is on a `mod` declaration or an `impl` block

#### Scenario: Nothing else changes
- **WHEN** the workspace is built and tested after the move
- **THEN** every crate that uses `vkxp-vk` compiles unchanged and the tests that passed before pass, none removed
