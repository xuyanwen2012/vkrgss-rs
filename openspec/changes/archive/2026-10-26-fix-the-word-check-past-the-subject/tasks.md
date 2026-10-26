# Tasks

## 1. The read past the subject

- [x] 1.1 Reproduce with the saved input, find the cause, fix it as a recorded patch of Oniguruma 5.9.6, check the matcher's other word checks and character reads of the same shape, add the input under `fuzz/fixed/regex/`. Verify: scenario "The saved input of nightly run 40296906749"; `cargo test -p vkrgss-ruby-sys --locked` passes.
- [x] 1.2 Check Ruby 1.9.2's engine for the same defect and fix it the same way if it has it; record the result in design.md. Verify: scenario "The same input through Ruby 1.9.2"; `cargo test -p vkrgss-ruby192-sys --locked` passes.
- [x] 1.3 Check the timeout input; record the result in design.md. Verify: the result and its evidence are under "Follow-ups" of design.md.
- [x] 1.4 Run `cargo xtask fuzz regex --seconds 1800` and `cargo xtask fuzz regex192 --seconds 1800`. Verify: both end without a finding outside the listed limits; the counts are recorded in design.md.

## 2. Documents

- [x] 2.1 `docs/vendored-security.md`, `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 3. Integration check

- [x] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate fix-the-word-check-past-the-subject --strict` passes.
