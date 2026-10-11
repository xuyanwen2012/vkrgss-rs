## MODIFIED Requirements

### Requirement: Single merge gate
The repository SHALL provide one command that checks formatting, lints with warnings denied,
runs all workspace tests, runs the Vulkan tests again without the validation layer, builds the workspace's documentation with warnings denied and
validates OpenSpec artifacts, and exits non-zero if any step fails. The steps that resolve
dependencies SHALL use the committed lock file unchanged. The command SHALL report how long
each step took.

#### Scenario: Gate passes on a healthy tree
- **WHEN** the gate command runs on a tree with no violations
- **THEN** it exits with status zero

#### Scenario: Gate reports the failing step
- **WHEN** any one of the six checks fails
- **THEN** the gate exits non-zero and identifies which check failed

#### Scenario: Broken documentation link
- **WHEN** a documentation comment links to an item that does not exist
- **THEN** the gate fails at the documentation step

#### Scenario: Lock file out of date
- **WHEN** a manifest asks for a dependency that the lock file does not hold
- **THEN** the gate fails without rewriting the lock file

#### Scenario: Durations
- **WHEN** the gate finishes, passing or failing
- **THEN** its output names each step that ran with the time it took

#### Scenario: Fault hidden by the validation layer
- **WHEN** a test of the Vulkan backend passes only because the validation layer is loaded
- **THEN** the gate fails at the step that runs without the layer

#### Scenario: Two failing test binaries
- **WHEN** tests of two crates fail
- **THEN** the output of the gate names both
