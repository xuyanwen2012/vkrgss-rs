# Tasks

## 1. Guest executable

- [ ] 1.1 The package `vkrgss-guest-rgss12` and the checks of `xtask` that `vkrgss-guest-rgss3` has. Verify: scenarios of "Guest executable of RGSS1 and RGSS2"; `cargo test -p xtask --locked` passes.

## 2. The program starts it

- [ ] 2.1 `guest_executable_of` and what the change of it needs in `vkrgss`; tests. Verify: the scenarios of "Executable of the guest process"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss --locked` passes, the tests of save states and of the sandbox included.

## 3. Package and documents

- [ ] 3.1 `cargo xtask dist` with three executables; `docs/` where they name the executables; `CHANGELOG.md`. Verify: the scenarios of "Release package holds the executables" (run `cargo xtask dist` once and the three-file check); the tests of `xtask` that check the documents and their links pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate start-xp-and-vx-guests-from-their-own-executable --strict` passes.
