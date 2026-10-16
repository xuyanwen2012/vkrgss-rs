## MODIFIED Requirements

### Requirement: Hermetic build
Building the workspace SHALL compile the vendored interpreter sources and its `Zlib` extension
from a fixed list of files with fixed compiler flags, using only a C compiler. The build SHALL
NOT run the interpreter's `configure` or `make`, need an installed Ruby, or use a zlib installed
on the system. For a target without a provided configuration header the build SHALL fail with a
message that names the target.

#### Scenario: No Ruby on the machine
- **WHEN** the workspace is built on a machine with a C compiler and no Ruby installation
- **THEN** the build succeeds and produces the interpreter library

#### Scenario: Unsupported target
- **WHEN** the crate is built for a target that has no configuration header
- **THEN** the build stops with an error naming that target and the supported one

#### Scenario: Header of aarch64
- **WHEN** the configuration headers of `aarch64-unknown-linux-gnu` are compared with those of `x86_64-unknown-linux-gnu`
- **THEN** they differ only in the platform strings and their leading comment
