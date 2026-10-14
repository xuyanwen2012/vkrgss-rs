# Tasks

## 1. Archive of RGSS3

- [x] 1.1 `xtask/src/rgss3a.rs`: the reader and writer of RGSSAD version 3 of decision 2, reusing the word cipher and `names_match` of `xtask/src/rgssad.rs` without changing what they do; tests on archives the tests generate. Verify: the four scenarios of "Archives of RGSSAD version 3"; `cargo test -p xtask --locked` passes and no existing test of `rgssad.rs` changed.

## 2. Runtime and probe of RGSS3

- [x] 2.1 `Runtime::Rgss3` (decision 1): `.rvdata2` and `--runtime rgss3`, frame size, registry key, archive name, the usage text; `vkxp::build` with the guest of RGSS3 (decision 8); `xtask/src/oracle/probe_rgss3.rb` and its generation in `probe.rs` (decisions 3 to 5): the model with Symbols and the Integers of RGSS2, the frame numbers, the loops and `frame_reset` on the original's side, late frames, the size in the request. `probe.rb` and `probe_rgss2.rb` stay byte for byte as they are. Verify: the four scenarios of "Runtime of the game", "Capture in vkxp of RGSS3", the scenario of "Probe of RGSS3", "Buttons of RGSS3" and "Repeat of RGSS3, measured on the original", "Frame numbers of RGSS3", "Screen resized in RGSS3" and "Late frame of RGSS3", and that the probe of RGSS3 with `--no-threads` runs the block of a thread at once in vkxp; `cargo test -p xtask --locked` passes.
- [x] 2.2 The probe put into `Game.rgss3a` by `game.rs` with the module of task 1.1 (implementation notes of `design.md`). Verify: the scenarios "Archive of VX Ace" and "Packed game of RGSS3 runs in vkxp" of "Games with an encrypted archive", and the three others of that requirement unchanged; `cargo test -p xtask --locked` passes.

## 3. The original's side

- [x] 3.1 The DLL of a game of RGSS3 at the path of `Library=`, `--dll` optional for RGSS3, the check before Wine (decision 6); the runtime decided before the prerequisites. Verify: the four scenarios of "The DLL of a game of RGSS3"; the scenario "No DLL" of "Missing prerequisites" still passes for a game of RGSS1; `cargo test -p xtask --locked` passes.
- [x] 3.2 The size of a captured frame from the request, and the choice of the window by that size as a function tested without a display (decision 5). Verify: the scenario "Size from the request"; the requests of the probes of RGSS1 and RGSS2 give the frame size of their runtime; `cargo test -p xtask --locked` passes.
- [ ] 3.3 The GStreamer plugin directories and variables of decision 7 on every Wine command. Verify: the four scenarios of "The original side has no GPU device and no network"; `cargo test -p xtask --locked` passes; no test or command of this task starts `wine`, `wineboot`, `wineserver` or `Xvfb`.

## 4. Documentation

- [ ] 4.1 `docs/development.md`, section "Comparing with the original runtime", as the implementation notes of `design.md` list it. Verify: the usage text of the command and the document name the same options and the same three values of `--runtime`; the section names `Library=`, `Game.rgss3a`, `RGSS3\RTP`, the size from the probe, VL Gothic as a `--font` and the GStreamer plugin directories; it claims no comparison of a real VX Ace game.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate run-the-oracle-on-vx-ace-games --strict` passes; `git diff main -- crates docs/decisions.md xtask/src/oracle/probe.rb xtask/src/oracle/probe_rgss2.rb` is empty.
