## Why

The first run of the merge gate on the CI runner with Ruby 1.9.2 in the tree failed
(2026-10-14, commit `7ba5ee5`): the files `test_dir.rb`, `test_require.rb` and
`test_system.rb` of the release's suite end with `*** buffer overflow detected ***:
terminated`, plain and under collector stress. The compiler of Ubuntu 24.04 defines
`_FORTIFY_SOURCE` by default; the compiler of the development machine does not, and there
the suites pass. Built here with `CFLAGS='-O2 -D_FORTIFY_SOURCE=3'` the same three files end
the same way, so the fault is in the vendored source as it is built, not in the runner.

A check of the C library that ends the process is either a write past a buffer in the
interpreter that will run the scripts of games, or an old idiom that the check misreads.
Either way the interpreter must not die of it in a distribution's default build.

## What Changes

- Every place of the vendored Ruby 1.9.2 at which the fortified C library ends the process
  in the suites is found and corrected by a recorded patch: a real overflow by its fix, an
  idiom the check misreads by writing it so that the object's size is true.
- The suites of the release pass, plain and stressed, when the interpreter is built with
  `_FORTIFY_SOURCE=3`, and the merge gate on the CI runner passes again.
- The same build is tried for Ruby 1.8.7, whose suites pass on the runner today; what is
  found there is recorded, and fixed if it is of the same kind.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby-1-9-2`: the interpreter passes its suites in a fortified build.

## Impact

`third_party/ruby-1.9.2/` (the corrected files, `PATCHES.md`, the manifest),
`crates/vkxp-ruby192-sys` (tests or build script only if the correction needs it),
`docs/development.md`. No change to what a script sees.
