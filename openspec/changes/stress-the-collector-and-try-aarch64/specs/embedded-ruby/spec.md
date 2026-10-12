## ADDED Requirements

### Requirement: Collector stress switch
With the environment variable `VKXP_GC_STRESS` set to a number `n` of 1 or more, an
interpreter started through `vkxp-ruby-sys` SHALL run its collector at every n-th
allocation of an object or of memory, except while collections are disabled. Unset, empty
or 0 SHALL change nothing. The change to the vendored collector SHALL be a recorded patch.

#### Scenario: Every allocation
- **WHEN** an interpreter is started with `VKXP_GC_STRESS=1` and a script allocates 100 objects
- **THEN** `GC.count`, or a counter the test reads in its place, has grown by at least 100

#### Scenario: Off
- **WHEN** an interpreter is started without the variable and the same script runs
- **THEN** the collector runs as often as it did before this change

### Requirement: Tests under collector stress
`cargo xtask stress` SHALL run the upstream suites in the sanitized build, the tests of the
binding and the tests of the executable that start a guest, each with the collector stress
switch on, and SHALL report every step that fails. A test left out SHALL be named in a
committed list with the reason. The command SHALL NOT be a step of the gate.

#### Scenario: Stressed run
- **WHEN** `cargo xtask stress` is run on a machine with `clang` and its sanitizer runtimes
- **THEN** every step passes and the sanitizers report nothing outside the allow-list

#### Scenario: A lost value shows
- **WHEN** a test evaluates code that keeps only the buffer pointer of a fresh string across an allocation, under the switch in the sanitized build
- **THEN** the run fails with a sanitizer report
