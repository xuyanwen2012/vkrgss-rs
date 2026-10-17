# Tasks

## 1. Filter

- [ ] 1.1 `seccomp.rs`, the call after Landlock, the options that leave it out, tests. Verify: scenario "Denied system calls" and the other scenarios of "Confined guest process"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss-guest -p vkrgss --locked` passes.

## 2. Android

- [ ] 2.1 The guest crate's filter test on a phone through `cargo xtask android-test` if it covers the crate, or a note in `design.md`. Verify: the test passes on both phones, or the note says why it was not run.

## 3. Documents

- [ ] 3.1 `docs/decisions.md` (confinement per platform), `docs/running.md` if it describes the sandbox, `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate filter-the-system-calls-of-the-guest --strict` passes.
