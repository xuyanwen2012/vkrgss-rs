# Tasks

## 1. Targets and command

- [x] 1.1 The three harnesses, their seeds, and `cargo xtask fuzz`. Verify: scenarios "A short run" (for each target) and "Unknown target"; `cargo test -p xtask --locked` passes.

## 2. First run

- [x] 2.1 Run each target for 20 minutes, fix or list what is found as `design.md` says, add the inputs of fixed findings as tests, fill "Checked". Verify: a second run of 5 minutes per target ends without a finding that is not listed; `cargo test -p vkxp-ruby-sys --locked` passes, the vendored-source checks included.

## 3. Review

- [x] 3.1 `docs/vendored-security.md` and the backports. Verify: the scenarios of "Later security fixes are reviewed"; `cargo test -p vkxp-ruby-sys -p vkxp-binding --locked` passes; `cargo test -p vkxp-ruby-sys --test sanitized -- --ignored` passes.

## 4. Documentation

- [ ] 4.1 `docs/development.md` describes the command and links the review. Verify: the tests of `xtask` that check the documents and their links pass.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate fuzz-the-interpreter-and-review-fixes --strict` passes.
