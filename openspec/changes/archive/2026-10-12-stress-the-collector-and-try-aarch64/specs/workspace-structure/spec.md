## ADDED Requirements

### Requirement: aarch64 job
The CI workflow SHALL have a job that runs the gate on an aarch64 runner and whose failure
does not fail the workflow. The documents SHALL name x86_64 as the supported architecture
and SHALL NOT claim support for aarch64.

#### Scenario: Job is present and may fail
- **WHEN** `.github/workflows/ci.yml` is read
- **THEN** it has a job on `ubuntu-24.04-arm` with the steps of the gate job and `continue-on-error: true`

#### Scenario: Documents
- **WHEN** `docs/compatibility.md` is read
- **THEN** it names x86_64 as supported and says that the aarch64 job shows what works there
