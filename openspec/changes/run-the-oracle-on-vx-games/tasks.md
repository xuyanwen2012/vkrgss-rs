# Tasks

## 1. Runtime of the game

- [x] 1.1 The runtime from `Scripts=` or `--runtime`; frame size, archive name, registry key and package names by runtime; a report that copes with pictures of different sizes. Verify: scenarios of "Runtime of the game", "Games with an encrypted archive" and "Frame capture" that need no original; `cargo test -p xtask --locked` passes.

## 2. Probe of RGSS2

- [x] 2.1 `probe_rgss2.rb`: seed, input model with the repeat rule of RGSS2, own frame number, the loops and `frame_reset` on the original's side, late frames. Verify: the scenarios of "Same conditions on both sides" and "Frames inside a transition" that are stated on the probe's model; `cargo test -p xtask --locked` passes.
- [x] 2.2 `--original-only` and `--no-threads`. Verify: scenarios of "Original alone" and "Games that start threads" that need no original.

## 3. On the original

- [ ] 3.1 Run `cargo xtask oracle ../../games/vx-visions-and-voices --original-only --no-threads` twice with the DLL the game ships, `--rtp RPGVX=../../ref/vx/rtp`, the schedule of `../../ref/tools/oracle/measurements/vx-v0-vv-input.txt` and the frames 100, 260, 1000, 3500, 6600, 11300, 11580, 11700, 11900, output under `target/`. Verify: both runs end with status 0 and give the same nine pictures byte for byte; frame 260 lies inside a fade and is captured; record in `design.md` under "Measured" the SHA-256 of the pictures and whether they equal those of `vx-v0-vv-sha256.txt` for the frames both have (a difference is recorded with its likely reason, not hidden; it does not fail the task).
- [ ] 3.2 Run the harness with the original on the XP game `../../games/群战示范工程` (a copy is made by the harness; DLL from `../../ref/sys`, `--rtp Standard=../../ref/app`, frames 10, 60 and 200, no input), once with the `xtask` of `main` before this change and once with this change. Verify: the pictures of the original are equal byte for byte in both.

## 4. Documentation

- [ ] 4.1 `docs/development.md`: the oracle on a VX game, the two options, what a late frame is, the limits (threads, transitions, a fixed window size). Verify: the usage text of the command and the document name the same options.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate run-the-oracle-on-vx-games --strict` passes.
