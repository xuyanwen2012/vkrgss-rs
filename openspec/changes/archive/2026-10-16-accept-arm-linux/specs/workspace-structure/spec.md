## MODIFIED Requirements

### Requirement: aarch64 job
The CI workflow SHALL have a job that runs the gate on an aarch64 runner, and its failure
SHALL fail the workflow. The documents SHALL name x86_64 and aarch64 Linux as supported.

#### Scenario: Job is present and may fail
- **WHEN** `.github/workflows/ci.yml` is read
- **THEN** it has a job on `ubuntu-24.04-arm` with the steps of the gate job and without `continue-on-error`, so its failure is not allowed

#### Scenario: Documents
- **WHEN** `docs/compatibility.md` is read
- **THEN** it names x86_64 and aarch64 Linux as supported
