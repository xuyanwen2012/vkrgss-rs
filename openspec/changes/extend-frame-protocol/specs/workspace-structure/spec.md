## ADDED Requirements

### Requirement: Host crates do not depend on guest crates
No host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`) and not the reference compositor
`vkxp-compose-ref` SHALL depend, directly or transitively, on a guest crate (`vkxp-core`,
`vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-binding`). Only the `vkxp` binary
may depend on both sides. As for the other dependency rules, development-only dependencies do
not count.

#### Scenario: Forbidden dependency is rejected
- **WHEN** a host crate or the reference compositor gains a dependency path to a guest crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binary may join both sides
- **WHEN** the `vkxp` binary depends on a guest crate and on a host crate
- **THEN** the dependency check reports no violation for it
