## MODIFIED Requirements

### Requirement: Upstream test suites pass
Ruby's own core test suite, its `Zlib` test suite and its sample test SHALL pass against the
interpreter exactly as built for vkrgss, and SHALL run as part of the workspace tests.

#### Scenario: Core suite
- **WHEN** the workspace tests run
- **THEN** the files of `test/ruby` report 625 tests with no failure and no error

#### Scenario: Zlib suite and sample
- **WHEN** the workspace tests run
- **THEN** `test/zlib` reports no failure and no error and `sample/test.rb` reports no `not ok`

### Requirement: Collector stress switch
With the environment variable `VKRGSS_GC_STRESS` set to a number `n` of 1 or more, an
interpreter started through `vkrgss-ruby-sys` SHALL run its collector at every n-th
allocation of an object or of memory, except while collections are disabled. Unset, empty
or 0 SHALL change nothing. The change to the vendored collector SHALL be a recorded patch.

#### Scenario: Every allocation
- **WHEN** an interpreter is started with `VKRGSS_GC_STRESS=1` and a script allocates 100 objects
- **THEN** `GC.count`, or a counter the test reads in its place, has grown by at least 100

#### Scenario: Off
- **WHEN** an interpreter is started without the variable and the same script runs
- **THEN** the collector runs as often as it did before this change
