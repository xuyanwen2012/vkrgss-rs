## ADDED Requirements

### Requirement: Jumps that are not exceptions cross host functions
A `throw` and a fatal error that pass through a protected call of the host or through a host
function SHALL arrive where they would without the host function in between: a `throw` at its
`catch`, a fatal error not rescuable by `rescue Exception`. Putting back the previous error
SHALL NOT raise.

#### Scenario: Throw through a host function
- **WHEN** a script calls a host function inside `catch(:done)` and the block that the host function yields to does `throw :done, 1`
- **THEN** `catch` returns 1 and no `TypeError` is raised

#### Scenario: Fatal error
- **WHEN** a host function resumes a fatal error inside `begin ... rescue Exception`
- **THEN** the `rescue` branch does not run and the interpreter ends with the fatal error
