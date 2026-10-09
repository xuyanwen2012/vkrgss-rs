# Tasks

## 1. Guest (`vkxp-binding`)

- [ ] 1.1 Add seed and preload scripts to the run configuration and `run_game`. Verify: tests for the four scenarios of `specs/rgss-runtime/spec.md`, including a reset.

## 2. Host (`vkxp-host`)

- [ ] 2.1 Add the recording format, the recorder and the replayer as input sources. Verify: unit tests of writing and reading, the not-a-recording scenario, and a session test that a replayed recording delivers the recorded snapshots frame by frame.

## 3. Executable (`vkxp`)

- [ ] 3.1 Add the four options and the seed defaults. Verify: argument tests, and end-to-end tests for the round-trip scenario of `specs/host-session/spec.md` and the two scenarios of the options requirement.
- [ ] 3.2 Open the game's files with `GameFiles::open`. Verify: the game-in-an-archive scenario and a damaged-archive run that exits non-zero naming the file.
- [ ] 3.3 Describe the options in `README.md`. Verify: every command shown matches the implemented options.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-seed-preload-and-replay --strict` passes.
