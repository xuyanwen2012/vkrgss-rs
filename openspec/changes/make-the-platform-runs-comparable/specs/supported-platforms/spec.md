## ADDED Requirements

### Requirement: The comparison games give the same output on every platform
With the same game, frames, input, RTP, fonts and built-in scripts, the frames and saves of the
comparison games SHALL be identical on x86_64 Linux, macOS and the two Android phones;
`cargo xtask xarch` SHALL run and compare them.

#### Scenario: All platforms
- **WHEN** `cargo xtask xarch` runs the ten comparison games on this machine, on the Mac and on both phones, and compares each with this machine
- **THEN** every frame and every save is identical

#### Scenario: Arial on Android
- **WHEN** a script on a phone evaluates `Font.exist?("Arial")`
- **THEN** it is true
