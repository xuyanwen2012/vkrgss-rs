## ADDED Requirements

### Requirement: Upstream suites do not depend on the caller's search path
The harness that runs the upstream test suites SHALL start them with a search path of its
own, made of its scratch directory and the system's directories, and not with the caller's.

#### Scenario: World-writable directory in the caller's path
- **WHEN** the upstream suites are run from a process whose `PATH` holds a directory that everybody may write
- **THEN** they pass as they do otherwise
