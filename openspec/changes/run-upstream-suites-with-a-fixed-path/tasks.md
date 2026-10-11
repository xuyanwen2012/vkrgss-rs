# Tasks

## 1. Search path

- [x] 1.1 The fixed search path and its test. Verify: the scenario of `specs/embedded-ruby/spec.md`; `cargo test -p vkxp-ruby-sys --locked` passes.

## 2. Gate

- [x] 2.1 `--no-fail-fast` in the two test steps and the texts that quote them. Verify: `cargo test -p xtask --locked` passes.

## 3. Integration check

- [x] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate run-upstream-suites-with-a-fixed-path --strict` passes.
