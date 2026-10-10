# Proposal

## Why

The merge gate fails when it is started by a program that ignores SIGINT, as a tool without a
terminal does. One upstream Ruby test (`test_should_propagate_signaled`) starts a child
interpreter that sends itself SIGINT; an ignored disposition is inherited through every child
process and the interpreter keeps a disposition that is not the default, so the child is not
interrupted. Every agent flow has to be told to wrap the gate in `env --default-signal=INT`.

## What Changes

- The test harness that runs Ruby's own suites starts the test executable with the default
  disposition of the signals the suites send to themselves, whatever the harness inherited.

Out of scope: how the interpreter treats signals inside vkxp (the "Process signals are left
alone" requirement stays as it is); the gate's step list.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the upstream suites pass whatever signal dispositions the caller has.

## Impact

- `crates/vkxp-ruby-sys/tests` (the suite runner in `tests/common/mod.rs`, used by
  `upstream.rs` and `sanitized.rs`). `AGENTS.md` and `CLAUDE.md` are not touched; they name the
  plain gate command already.
