## ADDED Requirements

### Requirement: Parts of the gate
`cargo xtask gate --job <name>` SHALL run one named part of the gate. The parts together
SHALL run exactly the checks of `cargo xtask gate`, which SHALL be unchanged.

#### Scenario: Union of the parts
- **WHEN** the test of `xtask` compares the parts with the full gate
- **THEN** every check, package and test target of the gate is in exactly one part

#### Scenario: Unknown part
- **WHEN** `cargo xtask gate --job nothing` runs
- **THEN** it fails and lists the names of the parts

### Requirement: Parallel merge checks in CI
The CI workflow SHALL run each part of the gate as a job of its own, with a compiler cache
for C and Rust, and the nightly workflow SHALL open or update an issue when one of its jobs
fails.

#### Scenario: One job per part
- **WHEN** `.github/workflows/ci.yml` is read
- **THEN** each part has a job that runs `cargo xtask gate --job` with its name

#### Scenario: Nightly failure
- **WHEN** `.github/workflows/nightly.yml` is read
- **THEN** a job that runs only when another failed opens or comments on an issue, and only that job may write issues
