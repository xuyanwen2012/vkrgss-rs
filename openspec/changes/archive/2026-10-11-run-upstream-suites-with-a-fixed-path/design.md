# Design

## Decisions

### The search path of the suites

In `crates/vkxp-ruby-sys/tests/common/mod.rs` the `PATH` given to a suite is the scratch
directory that holds the links to the executable, then `/usr/bin` and `/bin`; the caller's
`PATH` is not read. The suites start `sh`, `echo` and the interpreter itself, which those
directories and the scratch directory provide. The sanitized run uses the same function and
gets the same path.

A test: with a `PATH` of the caller that holds a directory with mode 0777 (made in a
temporary directory), the suite `test_beginendblock.rb` passes. It is run through the same
function as the real run, so it fails without the change.

### Every failure in one run

The steps `test` and `test-unvalidated` of the gate pass `--no-fail-fast` to `cargo test`.
The step still fails when any binary fails; the output then holds all of them.
`docs/development.md`, `AGENTS.md` and the comments that quote the commands are updated.

## Follow-ups

None yet.
