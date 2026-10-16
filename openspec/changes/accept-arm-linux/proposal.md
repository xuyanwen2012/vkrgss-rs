# Accept ARM Linux (P1)

## Why

P1 of `docs/decisions.md` is accepted when, on aarch64, the frames and saves of the
comparison games equal those of x86_64 byte for byte for the same games, frames and input.
On 2026-10-16 this was run on the owner's Jetson Orin Nano (`duck-stable`, Ubuntu 24.04.5,
gcc 13.3, llvmpipe) and on the development machine (x86_64), from the same commit
(`3259c0d4`), with `cargo xtask oracle --vkrgss-only` and the frames and input schedules of
the comparisons with the original: the seven XP games, Visions & Voices (VX), Crysalis and In
Search of Immortality (VX Ace). All 84 frames are identical pixel for pixel, and the two
saves In Search of Immortality writes (`Save01.rvdata2`, `AutoSave.rvdata2`) are identical
byte for byte; those saves were shown equal to the original's on x86_64 the same day.
Both interpreters are built for aarch64 with verified headers (change
`build-the-interpreters-for-aarch64`), and the collector stress of Ruby 1.9.2 passes on
aarch64 after the change `guard-values-held-by-inner-pointers`.

## What Changes

- The CI job `gate-aarch64` must pass: `continue-on-error` is removed.
- `docs/compatibility.md` and `README.md` name x86_64 and aarch64 Linux as supported.
- `docs/decisions.md`: the row P1 is "Done" with the evidence above; "Goal and scope" names
  ARM Linux as supported.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: the aarch64 job is required; the documents claim aarch64.

## Impact

`.github/workflows/ci.yml`, `xtask/tests/workflows.rs` (if it checks `continue-on-error`),
`docs/compatibility.md`, `docs/decisions.md`, `README.md`, `CHANGELOG.md`.
