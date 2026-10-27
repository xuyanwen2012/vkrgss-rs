## MODIFIED Requirements

### Requirement: Fuzz targets of Ruby 1.9.2
`cargo xtask fuzz` SHALL also build and run three targets of Ruby 1.9.2, `marshal192`,
`parse192` and `regex192`, that feed arbitrary bytes to `Marshal.load`, to the parser and
to the regular expression engine. A Ruby exception SHALL NOT count as a failure. An input
that made a target fail and was fixed SHALL be kept as a test of the normal test step.

#### Scenario: A short run
- **WHEN** `cargo xtask fuzz marshal192 --seconds 30` is run on a machine with clang and its sanitizer runtimes
- **THEN** the target runs for about 30 seconds and ends with status 0 when nothing failed

#### Scenario: Unknown target
- **WHEN** `cargo xtask fuzz nothing` is run
- **THEN** it ends with an error that names all nine targets
