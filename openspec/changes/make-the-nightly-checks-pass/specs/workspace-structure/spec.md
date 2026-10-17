## ADDED Requirements

### Requirement: The nightly fuzz job keeps its findings
The fuzz job of `nightly.yml` SHALL upload the inputs a target put aside when the job fails,
and the dependency audit SHALL run with a version of `cargo-deny` that reads the current
advisory database.

#### Scenario: Upload on failure
- **WHEN** `.github/workflows/nightly.yml` is read
- **THEN** the fuzz job has a step that runs on failure and uploads `target/tmp/fuzz/<target>/artifacts/` with a pinned action

#### Scenario: Version of cargo-deny
- **WHEN** the audit job is read
- **THEN** it installs `cargo-deny` at 0.20.2
