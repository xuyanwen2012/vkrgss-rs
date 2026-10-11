# Tasks

## 1. Character classes

- [ ] 1.1 The patch to `regparse.c`, its record, and the test of `design.md`. Verify: the scenarios of `specs/embedded-ruby/spec.md`; `cargo test -p vkxp-ruby-sys --locked` passes with the local gcc and with `CC=clang` (a separate `CARGO_TARGET_DIR`).

## 2. Audit tests

- [ ] 2.1 The lock and the bounded retry in the tests of `xtask/src/audit.rs`. Verify: `cargo test -p xtask --locked` passes 50 times in a row.

## 3. Warnings

- [ ] 3.1 The two compilations of `design.md`, with "Found" filled in and real findings patched. Verify: `cargo test -p vkxp-ruby-sys -p vkxp-binding --locked` passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate fix-findings-of-the-first-ci-runs --strict` passes.
