## ADDED Requirements

### Requirement: Release package command
`cargo xtask dist` SHALL build the release executable and write under `target/dist/` an
archive named after the version and the target that holds the executable, the README, the
changelog, the licence, the third-party notice and the licence texts of the bundled source,
and beside it a SHA-256 checksum file that `sha256sum -c` accepts.

#### Scenario: Package
- **WHEN** `cargo xtask dist` has run
- **THEN** `target/dist/vkxp-0.1.0-x86_64-unknown-linux-gnu.tar.gz` unpacks to one directory holding an executable `vkxp`, `README.md`, `CHANGELOG.md`, `LICENSE`, `THIRD-PARTY.md`, `licenses/ruby-1.8.7/COPYING` and `licenses/oniguruma-5.9.6/COPYING`, and `sha256sum -c` on the checksum file passes

#### Scenario: Missing file
- **WHEN** a file of the list does not exist
- **THEN** the command fails and names it

### Requirement: Workflows
The repository SHALL have a workflow that runs the merge gate on a software Vulkan device
for every push to `main` and every pull request, and a workflow that for a version tag
builds the package with `cargo xtask dist` and publishes it as a GitHub release, and that
only builds it when started by hand.

#### Scenario: Gate in the workflow
- **WHEN** the continuous integration workflow file is read
- **THEN** it runs `cargo xtask gate` with `VK_LOADER_DRIVERS_SELECT` set to lavapipe

#### Scenario: Release only for a tag
- **WHEN** the release workflow file is read
- **THEN** the step that creates the release is conditional on a tag reference
