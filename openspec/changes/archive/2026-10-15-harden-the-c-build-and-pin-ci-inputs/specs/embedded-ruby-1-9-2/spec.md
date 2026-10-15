## ADDED Requirements

### Requirement: Hardening flags are asked for by the build script
The build script of the interpreter SHALL itself give the compiler `-D_FORTIFY_SOURCE=3`,
`-fstack-protector-strong` and `-fstack-clash-protection`, and on x86_64
`-fcf-protection=full`, whatever the compiler's defaults are. Builds for the sanitizers or
fuzzing MAY leave out `_FORTIFY_SOURCE`.

#### Scenario: Flags of the build
- **WHEN** the flags that `base_build()` of the build script gives the compiler are read
- **THEN** they contain those flags, and the x86_64 one only for that architecture

#### Scenario: The suites pass with them
- **WHEN** the tests of the interpreter crate run with the default compiler of this machine
- **THEN** they pass
