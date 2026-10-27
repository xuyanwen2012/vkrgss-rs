## MODIFIED Requirements

### Requirement: Fuzz targets
`cargo xtask fuzz <target>` SHALL build and run, with libFuzzer and the sanitizers, one of
three targets that feed arbitrary bytes to `Marshal.load`, to the parser, and to the
compilation and matching of a regular expression. A Ruby exception SHALL NOT count as a
failure. The command SHALL NOT be a step of the gate. An input that made a target fail and
was fixed SHALL be kept as a test of the normal test step.

#### Scenario: A short run
- **WHEN** `cargo xtask fuzz marshal --seconds 30` is run on a machine with clang and its sanitizer runtimes
- **THEN** the target runs its seeds and new inputs for about 30 seconds and ends with status 0 when nothing failed

#### Scenario: Unknown target
- **WHEN** `cargo xtask fuzz nothing` is run
- **THEN** it ends with an error that names every target, these three among them
