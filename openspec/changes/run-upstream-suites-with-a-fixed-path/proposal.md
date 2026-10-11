# Run the upstream suites with a fixed search path, and report every failing test

## Why

On the GitHub runner `test_beginendblock.rb` of the upstream Ruby suites fails: the runner's
`PATH` holds a directory that everybody may write (`/opt/pipx_bin`), Ruby 1.8.7 warns about
such a directory whenever it starts a program ("Insecure world writable dir ... in PATH"),
and the test compares the error stream of a child with an exact text. The harness passes the
caller's `PATH` on, so the result of the suites depends on the machine.

The gate also stops at the first test binary that fails, so each run on another machine has
shown one finding at a time: five runs for five findings.

## What Changes

- The harness starts the upstream suites with a search path of its own: its scratch
  directory, `/usr/bin` and `/bin`.
- The `test` steps of the gate run every test binary and report all failures.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the upstream suites do not depend on the caller's search path.
- `workspace-structure`: the gate reports every failing test binary.

## Impact

`crates/vkxp-ruby-sys/tests/common/mod.rs`, `xtask/src/gate.rs`, the texts that quote the
command of the step.
