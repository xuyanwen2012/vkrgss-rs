## ADDED Requirements

### Requirement: Fortified build
The interpreter of Ruby 1.9.2 SHALL pass the suites of the release that the workspace tests
run, plain and under collector stress, when the vendored source is compiled with
`-O2 -D_FORTIFY_SOURCE=3`, and the build script SHALL NOT switch that check off for the
library. Each correction SHALL be a patch recorded in `PATCHES.md` that says whether it fixes
a write past a buffer or rewrites an idiom the check misreads.

#### Scenario: The three files
- **WHEN** `cargo test -p vkxp-ruby192-sys --locked --test upstream` is run with `CFLAGS='-O2 -D_FORTIFY_SOURCE=3'` in a target directory of its own
- **THEN** it passes, and `test_dir.rb`, `test_require.rb` and `test_system.rb` are not among the files left out or allowed to fail for this reason

#### Scenario: Under collector stress
- **WHEN** `cargo test -p vkxp-ruby192-sys --locked --test upstream_stress` is run with the same flags
- **THEN** it passes

#### Scenario: The plain build
- **WHEN** `cargo test -p vkxp-ruby192-sys --locked` is run without those flags
- **THEN** it passes as before, and the test of the manifest accepts the corrected files with their patches

#### Scenario: The check stays on
- **WHEN** the flags that the build script gives the compiler are read
- **THEN** none undefines `_FORTIFY_SOURCE` or sets it to 0 for the library as a whole
