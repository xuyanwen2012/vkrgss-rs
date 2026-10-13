# Tasks

## 1. Game files

- [x] 1.1 The reader of RGSSAD version 3 in `vkxp-vfs`, the archive name `Game.rgss3a` for RGSS3 and the version read by runtime; a builder of version 3 archives in the test code, written from the layout in `design.md`. Verify: the scenarios of "Table of RGSSAD version 3", "Entry data of RGSSAD version 3", "Encrypted archive" and "Damaged archive" in `specs/game-files/spec.md`; `cargo test -p vkxp-vfs --locked` passes.
- [x] 1.2 Property tests of version 3 beside those of version 1; tests of the run-time package of RGSS3 and of detection with `Library=System\RGSS300.dll` and `System\RGSS301.dll`; the documentation of the crate names the third archive. Verify: the scenarios of "Arbitrary archive bytes" and "Run-time package directories"; `cargo test -p vkxp-vfs --locked` and `cargo doc -p vkxp-vfs --no-deps --locked` pass.

## 2. Extraction tool

- [x] 2.1 `cargo xtask extract-rpg` finds the blocks that start with `def rgss_main` and `def rgss_stop` beside those of `module RPG;`; the message for a file without a block names the memory image and `docs/running.md`. Tests on generated images only. Verify: the scenarios of "Extraction tool" in `specs/game-launch/spec.md`; the tests of `extract.rs` that exist pass unchanged; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p xtask --locked` passes.

## 3. Program

- [ ] 3.1 The built-in scripts file of RGSS3: the default `rpg_builtin_rgss3.rb`, the message for a missing file, the usage text. The refusal of RGSS3 leaves `runtime_of` for one function that the program and a guest process call after `guest_inputs`. Verify: the scenarios of "Built-in scripts file"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.
- [ ] 3.2 A run of RGSS3 opens the game's files with `Game.rgss3a`, reads the file that `Scripts=` names and then ends with the message; end-to-end tests on generated games of RGSS3, with a builder of version 3 archives of their own. Verify: the scenarios of "VX Ace is refused" and "Encrypted archive of the game"; the tests of XP and VX in `crates/vkxp/tests/end_to_end.rs` pass unchanged; the same test command passes.

## 4. Documentation

- [ ] 4.1 `docs/running.md`: what follows the runtime for RGSS3 (the key `RTP`, `Game.rgss3a`, `rpg_builtin_rgss3.rb`), `--rtp RPGVXAce=DIR`, the section "The built-in scripts of RPG Maker VX Ace", the rows of `--rtp` and `--builtin-scripts`, and what a run of a game of VX Ace does today. Verify: the usage text of the program, the message for a missing file of RGSS3 and the document name the same file and section; the document still says that VX Ace is planned and not supported.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate detect-rgss3-and-read-its-files --strict` passes.
