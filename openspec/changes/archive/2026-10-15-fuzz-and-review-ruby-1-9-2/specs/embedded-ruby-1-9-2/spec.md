## ADDED Requirements

### Requirement: Sanitized run of the upstream suite
`vkrgss-ruby192-sys` SHALL have a test, ignored by default, that builds Ruby 1.9.2 with clang,
AddressSanitizer and UndefinedBehaviorSanitizer and runs the upstream suite with it. Every
report SHALL fail the run unless an allow-list in the crate names it with its reason.

#### Scenario: A clean run
- **WHEN** `cargo test -p vkrgss-ruby192-sys --test sanitized -- --ignored` is run on a machine with clang and its sanitizer runtimes
- **THEN** it passes, and every report it saw is on the allow-list

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
- **THEN** it ends with an error that names all six targets

### Requirement: Later security fixes of Ruby 1.9.2 are reviewed
`docs/vendored-security.md` SHALL list the security fixes published for Ruby after
1.9.2-p0 that concern code the tree of Ruby 1.9.2 builds, and for each whether a game's
data reaches it and whether the fix was backported as a recorded patch, does not apply, or
was left, with the reason.

#### Scenario: A row for each
- **WHEN** the section on Ruby 1.9.2 of `docs/vendored-security.md` is read
- **THEN** every row has an identifier, the code concerned and one of the three outcomes with its reason or patch number

#### Scenario: Backports are recorded
- **WHEN** a row says backported
- **THEN** `PATCHES.md` of the tree of Ruby 1.9.2 has the patch and the vendored-source checks pass
