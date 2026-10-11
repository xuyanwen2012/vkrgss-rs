# Tasks

## 1. The patch to `marshal.c`

- [x] 1.1 Guards as in `design.md`, with the diff, the entry in `PATCHES.md` and the test of `design.md`. Verify: the scenarios of `specs/embedded-ruby/spec.md` (the first in the container if one can be started); `cargo test -p vkxp-ruby-sys -p vkxp-binding --locked` passes with the local gcc and with `CC=clang` (a separate `CARGO_TARGET_DIR`).

## 2. The audit

- [ ] 2.1 The reading of `design.md`, with "Found" filled in and what it finds patched. Verify: the same tests pass; the upstream suites that `vkxp-ruby-sys` runs pass.

## 3. Documentation

- [ ] 3.1 "Testing with another compiler" in `docs/development.md`. Verify: `cargo test -p xtask --locked` (the link check) passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate guard-dumped-strings-in-marshal --strict` passes.
