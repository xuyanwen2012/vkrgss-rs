# Tasks

## 1. Archive property tests (`vkxp-vfs`)

- [x] 1.1 Add the property tests and fix what they find. Verify: the scenarios of `specs/game-files/spec.md`; `cargo test -p vkxp-vfs --locked` passes.

## 2. Audit (`xtask`, `deny.toml`)

- [x] 2.1 Add `deny.toml` and `cargo xtask audit`. Verify: the scenarios of `specs/workspace-structure/spec.md` as unit tests in `xtask`; the note under "Checked" in `design.md` on whether the check was run and what it reported.

## 3. Sanitizer run (`vkxp-ruby-sys`)

- [x] 3.1 Add the opt-in sanitized build and run, the exclusion of the collector's scan and the allow-list. Verify: the scenarios of `specs/embedded-ruby/spec.md`; the vendored-source checks of the crate still pass.
- [x] 3.2 Fix the small findings, list the others, and write the outcome under "Checked" in `design.md`. Verify: the notes are there; `cargo test -p vkxp-ruby-sys --locked` passes.

## 4. Documents

- [x] 4.1 Describe the two commands in `README.md`. Verify: the text names both commands as they are run.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate sanitize-vendored-c-and-check-supply-chain --strict` passes.
