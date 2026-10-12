# Tasks

## 1. Game files

- [x] 1.1 The runtime value and its detection; the key `RTP`; the names of the run-time packages by runtime; the archive name by runtime. Verify: the scenarios of `specs/game-files/spec.md`; `cargo test -p vkxp-vfs --locked` passes.

## 2. Guest

- [x] 2.1 `RunConfig` carries the runtime; the globals of test mode follow it. Verify: the scenarios of `specs/rgss-runtime/spec.md`; `cargo test -p vkxp-binding --locked` passes.

## 3. Program

- [ ] 3.1 The option `--runtime`, detection before anything else, the error for a conflict, the end of a VX Ace run. Verify: scenarios of "Runtime of the run" and "VX Ace is refused"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.
- [ ] 3.2 The built-in scripts file by runtime, the run-time package of a VX game, `Game.rgss2a`, the experimental notice. Verify: the scenarios of "Built-in scripts file", "Encrypted archive of the game" and "Experimental runtime"; the same test command passes.

## 4. Documentation

- [ ] 4.1 `docs/running.md`: the option, how the runtime is found, `--rtp RPGVX=...`, the built-in scripts file of RGSS2 and how it is made today, and that a VX game does not get far yet. Verify: the help text of the program and the document name the same option and file names.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate detect-the-runtime-and-open-vx-games --strict` passes.
