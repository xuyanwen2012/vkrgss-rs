# Tasks

## 1. Marshal target

- [x] 1.1 The engine's recursive functions in the stack list of `marshal` in both crates; a test of the classification. Verify: scenario "Deep pattern through Marshal"; `cargo test -p vkrgss-ruby-sys -p vkrgss-ruby192-sys --locked` passes.

## 2. Free list of Ruby 1.8.7

- [ ] 2.1 Reproduce, find the cause, fix it as a recorded patch, replay test under `fuzz/fixed/`, check Ruby 1.9.2. Verify: scenario "The saved input"; `cargo xtask fuzz parse --seconds 1800` and `cargo xtask fuzz marshal --seconds 1800` end without an unlisted finding; the sanitized run of `vkrgss-ruby-sys` passes.

## 3. Documents

- [ ] 3.1 `docs/vendored-security.md`, `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate fix-the-nightly-fuzz-findings --strict` passes.
