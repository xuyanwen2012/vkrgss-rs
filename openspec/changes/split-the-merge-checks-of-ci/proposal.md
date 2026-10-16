# Split the merge checks of CI into parallel jobs

## Why

The owner decided on 2026-10-15 (Q10, "Process" in `docs/decisions.md`) that the merge gate
of CI keeps the fast checks, about ten minutes in parallel jobs, the fast part of the
collector stress included, and that the nightly workflow opens an issue when it fails;
`cargo xtask gate` stays one command that runs everything locally.

Today the CI job `gate` runs `cargo xtask gate` in one job and takes 31 minutes (run
38822373885 of 2026-10-15: `test` 1402 s, `test-unvalidated` 355 s, `clippy` 52 s). The
slowest test binaries were `tests/upstream_stress.rs` of Ruby 1.9.2 (412 s) and a unit test
binary (336 s). Both interpreters are compiled from scratch in every run, because cargo
decides by modification time and a checkout gives every file a new one.

## What Changes

- `cargo xtask gate --job <name>` runs one named part of the gate; the parts together are
  exactly the steps `cargo xtask gate` runs, which a test checks. `cargo xtask gate` alone is
  unchanged.
- The CI workflow runs the parts as parallel jobs, each with the same prerequisites as now,
  and a C compiler cache (sccache) so the interpreters are not compiled again when their
  sources did not change.
- The nightly workflow opens an issue (or comments on the open one) when a job fails.
- The aarch64 job is unchanged apart from what the split needs; it becomes required with P1.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: parts of the gate, parallel jobs in CI, issue on a nightly failure.

## Impact

`xtask` (`gate.rs`, `main.rs`, tests), `.github/workflows/ci.yml`,
`.github/workflows/nightly.yml`, `xtask/tests/workflows.rs`, `docs/development.md`,
`CHANGELOG.md`. No change to the program.
