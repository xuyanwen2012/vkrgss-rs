# Link libcrypt statically into the release package

## Why

The package that the release workflow builds on Ubuntu 24.04 does not start on Fedora:
`error while loading shared libraries: libcrypt.so.1`. The interpreter needs `crypt` for
`String#crypt`, Ubuntu's library has the name `libcrypt.so.1` and Fedora's `libcrypt.so.2`.
Every other library the executable needs (the C library, libm, libgcc, ALSA, libudev) has
the same name on current distributions.

## What Changes

- The build script of `vkxp-ruby-sys` links libcrypt statically when the environment
  variable `VKXP_STATIC_CRYPT` is `1`, and dynamically otherwise, as today.
- `cargo xtask dist` sets that variable, and fails with a message that names the static
  library and its package when the library is not installed.
- `cargo xtask dist` checks the package it has built: the executable must not need a
  libcrypt at run time.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: the release package does not depend on the name of libcrypt.

## Impact

`crates/vkxp-ruby-sys/build.rs`, `xtask/src/dist.rs`, `.github/workflows/release.yml` (the
note about run-time needs), `docs/releasing.md`, `docs/building.md`.
