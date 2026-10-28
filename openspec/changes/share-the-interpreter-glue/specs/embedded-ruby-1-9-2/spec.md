## ADDED Requirements

### Requirement: Jumps that are not exceptions cross host functions of Ruby 1.9.2
Over Ruby 1.9.2 a `throw` and a fatal error that pass through a protected call of the host or
through a host function SHALL arrive where they would without the host function in between,
and putting back the previous error SHALL NOT raise, also when that error is a pending
`throw`.

#### Scenario: Throw through a host function
- **WHEN** a script calls a host function inside `catch(:done)` and the block it yields to does `throw :done, 1`
- **THEN** `catch` returns 1 and no `TypeError` is raised

#### Scenario: Protected call after a throw
- **WHEN** a host function's protected body runs while a `throw` is pending and that body raises
- **THEN** the host receives the exception as a value and no Rust frame is unwound
