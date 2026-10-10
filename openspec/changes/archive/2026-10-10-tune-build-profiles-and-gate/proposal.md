# Proposal

## Why

The workspace has no build profile of its own, so the gate runs the CPU reference compositor,
the JPEG decoder, the property tests and every dependency without optimisation, and a release
build is whatever the compiler defaults to. The gate does not build the documentation, does
not hold the lock file, and does not say how long each step took. Two sentences of
`docs/decisions.md` describe things that do not exist in the code.

## What Changes

- Build profiles in the workspace manifest: optimised dependencies and lightly optimised
  workspace crates for development and tests, a defined release profile, and a profiling
  profile.
- The gate gains a documentation step with warnings denied, passes `--locked` to the cargo
  steps that resolve dependencies, and reports the duration of each step.
- Three more workspace lints: `unreachable_pub`, `clippy::dbg_macro`, `clippy::todo`.
- `docs/decisions.md` is corrected in two places (see `design.md`), and `README.md` says how
  to build the optimised executable and lists the gate's steps as they are.

Out of scope: continuous integration, dependency auditing, `missing_docs`, the pedantic
clippy groups, changes to what any crate does at run time.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: the steps of the merge gate, the build profiles, the lint set.

## Impact

- Root `Cargo.toml`, `xtask/src/gate.rs`, `README.md`, `AGENTS.md` and `CLAUDE.md` (the line
  that lists the gate's steps; the two files stay identical), `docs/decisions.md`.
- Any crate where a new lint or the documentation step reports something: visibility
  keywords and documentation comments only.
