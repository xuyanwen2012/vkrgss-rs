## ADDED Requirements

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
- **THEN** it ends with an error that names the three targets

### Requirement: Later security fixes are reviewed
The repository SHALL have a document that lists the security fixes published for Ruby 1.8
after 1.8.7-p374 and for Oniguruma after 5.9.6, and for each whether the code concerned is
built, whether a game's data reaches it, and whether the fix was backported as a recorded
patch, does not apply, or was left, with the reason.

#### Scenario: A row for each
- **WHEN** `docs/vendored-security.md` is read
- **THEN** every row has an identifier, the code concerned and one of the three outcomes with its reason or patch number

#### Scenario: Backports are recorded
- **WHEN** a row says backported
- **THEN** `PATCHES.md` of that tree has the patch and the vendored-source checks pass
