# Package Linux aarch64

## Why

Linux aarch64 is supported (milestone P1: frames and saves identical to x86_64, the whole gate
runs on `ubuntu-24.04-arm` in CI), but `cargo xtask dist` names its archive after the one
target `x86_64-unknown-linux-gnu` and the release workflow builds only on x86_64, so a user of
an ARM Linux machine has to build from source.

## What Changes

- `cargo xtask dist` packages for the target it runs on: `x86_64-unknown-linux-gnu` or
  `aarch64-unknown-linux-gnu`, named so in the archive; any other host is refused with a
  message.
- The release workflow builds the package on `ubuntu-24.04` and on `ubuntu-24.04-arm`, and the
  release carries both archives and their checksum files, besides the APK. The notes name both
  targets and their requirements.
- `docs/building.md`, `README.md` (the platform table and the quick start), `CHANGELOG.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: the release package is built for the host's target, x86_64 or aarch64
  Linux, and the release carries both.

## Impact

`xtask/src/dist.rs`, `.github/workflows/release.yml`, the documents above.
