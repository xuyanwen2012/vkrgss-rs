## MODIFIED Requirements

### Requirement: Release package command
`cargo xtask dist` SHALL build the release executable and write under `target/dist/` an
archive named after the version and the target that holds the executable, the README, the
changelog, the licence, the third-party notice and the licence texts of the bundled source,
and beside it a SHA-256 checksum file that `sha256sum -c` accepts. The target SHALL be that of
the machine it runs on, `x86_64-unknown-linux-gnu` or `aarch64-unknown-linux-gnu`; on any other
machine the command SHALL fail and say so.

#### Scenario: Package
- **WHEN** `cargo xtask dist` has run on x86_64 Linux
- **THEN** `target/dist/vkrgss-0.1.0-x86_64-unknown-linux-gnu.tar.gz` unpacks to one directory holding an executable `vkrgss`, `README.md`, `CHANGELOG.md`, `LICENSE`, `THIRD-PARTY.md`, `licenses/ruby-1.8.7/COPYING` and `licenses/oniguruma-5.9.6/COPYING`, and `sha256sum -c` on the checksum file passes

#### Scenario: Package on aarch64
- **WHEN** `cargo xtask dist` has run on aarch64 Linux
- **THEN** the archive is named `vkrgss-<version>-aarch64-unknown-linux-gnu.tar.gz` and holds the same files

#### Scenario: Missing file
- **WHEN** a file of the list does not exist
- **THEN** the command fails and names it

## ADDED Requirements

### Requirement: Both Linux packages in a release
The release workflow SHALL build the Linux package on an x86_64 and on an aarch64 runner, and a
release SHALL carry both archives with their checksum files.

#### Scenario: Both packages in a release
- **WHEN** the release workflow file is read
- **THEN** it builds the package on `ubuntu-24.04` and `ubuntu-24.04-arm` and passes both archives and checksum files to the step that creates the release
