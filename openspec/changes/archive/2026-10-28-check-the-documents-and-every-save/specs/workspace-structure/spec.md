## ADDED Requirements

### Requirement: The architecture document matches the workspace
A test SHALL fail when the crate table of `docs/architecture.md` does not name exactly the
workspace members, when its paragraph on `unsafe` disagrees with the `unsafe_code` attributes of
the crates and their modules, or when a stated target list of an interpreter crate differs from
its `targets.rs`.

#### Scenario: Current repository
- **WHEN** the test runs on the repository
- **THEN** it passes

#### Scenario: Missing crate
- **WHEN** the check is given a crate table without `vkrgss-audio`
- **THEN** it reports `vkrgss-audio` as missing

#### Scenario: Module allowed unsafe but not listed
- **WHEN** the check is given an `unsafe` paragraph that omits a module of `vkrgss-guest` that allows `unsafe_code`
- **THEN** it reports that module
