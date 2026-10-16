# Tasks

## 1. Sanitized run

- [x] 1.1 `tests/sanitized.rs` and `tests/sanitized.allow` of `vkrgss-ruby192-sys`; fix or allow-list what it reports, as `design.md` says. Verify: scenario "A clean run"; `cargo test -p vkrgss-ruby192-sys --locked` passes, the vendored-source checks included.

## 2. Targets and command

- [x] 2.1 The three harnesses, their seeds, and the targets `marshal192`, `parse192`, `regex192` in `cargo xtask fuzz`. Verify: scenarios "A short run" (for each target) and "Unknown target"; `cargo test -p xtask --locked` passes.

## 3. First run

- [x] 3.1 Run each target for 20 minutes, fix or list what is found, add the inputs of fixed findings under `fuzz/fixed/` with a test that replays them, fill "Follow-ups". Verify: a second run of 5 minutes per target ends without a finding that is not listed; `cargo test -p vkrgss-ruby192-sys -p vkrgss-binding-rgss3 --locked` passes.

## 4. Review

- [x] 4.1 The section on Ruby 1.9.2 of `docs/vendored-security.md` and the backports, each with a test in `tests/backports.rs` where a few lines can show the defect. Verify: the scenarios of "Later security fixes of Ruby 1.9.2 are reviewed"; `cargo test -p vkrgss-ruby192-sys -p vkrgss-binding-rgss3 --locked` passes; the sanitized run passes.

## 5. Documentation

- [ ] 5.1 `docs/development.md` names the new targets and the sanitized run; `CHANGELOG.md` lists the fixes under Security. Verify: the tests of `xtask` that check the documents and their links pass.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate fuzz-and-review-ruby-1-9-2 --strict` passes.
