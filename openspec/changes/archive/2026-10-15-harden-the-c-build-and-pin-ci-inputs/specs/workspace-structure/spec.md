## MODIFIED Requirements

### Requirement: Workflows
The repository SHALL have a workflow that runs the merge gate on a software Vulkan device
for every push to `main` and every pull request, and a workflow that for a version tag
builds the package with `cargo xtask dist` and publishes it as a GitHub release, and that
only builds it when started by hand. The workflows SHALL refer to actions by commit hash,
install tools at fixed versions, name the C compiler, and cancel a run in progress only for
a pull request. A scheduled workflow SHALL run the dependency audit, the collector stress,
the sanitized suites and a short run of each fuzz target, without blocking merges.

#### Scenario: Gate in the workflow
- **WHEN** the continuous integration workflow file is read
- **THEN** it runs `cargo xtask gate` with `VK_LOADER_DRIVERS_SELECT` set to lavapipe

#### Scenario: Release only for a tag
- **WHEN** the release workflow file is read
- **THEN** the step that creates the release is conditional on a tag reference

#### Scenario: Pinned inputs
- **WHEN** the workflow files are read
- **THEN** every `uses:` names a 40-character commit hash, `@fission-ai/openspec` is installed with a version, and the jobs that build set `CC`

#### Scenario: Pushes to main are not cancelled
- **WHEN** the continuous integration workflow file is read
- **THEN** `cancel-in-progress` is true only for pull requests

#### Scenario: Scheduled checks
- **WHEN** `.github/workflows/nightly.yml` is read
- **THEN** it runs on a schedule and by hand, and its jobs run `cargo xtask audit`, `cargo xtask stress`, the sanitized suites and `cargo xtask fuzz` for each target
