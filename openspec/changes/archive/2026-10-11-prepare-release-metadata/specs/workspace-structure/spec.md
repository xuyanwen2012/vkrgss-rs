## ADDED Requirements

### Requirement: Package metadata
Every crate of the workspace SHALL have a description, the licence, the repository address
`https://github.com/xuyanwen2012/vkxp` and the workspace's version in its package metadata,
and SHALL NOT be publishable to a registry.

#### Scenario: Metadata of every crate
- **WHEN** `cargo metadata --no-deps` is read for the workspace
- **THEN** every package has a non-empty `description`, the `license` `MIT`, that `repository` and an empty `publish` list
