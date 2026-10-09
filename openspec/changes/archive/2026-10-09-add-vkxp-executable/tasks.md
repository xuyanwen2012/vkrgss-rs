# Tasks

## 1. Extraction tool (`xtask`)

- [x] 1.1 Implement `cargo xtask extract-rpg <DLL> <OUTPUT>`. Verify: unit tests for the two extraction-tool scenarios in `specs/game-launch/spec.md`, with input files built in the test.

## 2. Executable (`vkxp`)

- [x] 2.1 Implement argument parsing with the option set of `design.md` and the usage message. Verify: unit tests for valid combinations, the unknown-option scenario and malformed `--rtp`, `--dump` and `--frames` values.
- [x] 2.2 Implement `FontBackend` over `vkxp-text`. Verify: unit tests with the test fonts of `vkxp-text` as the only source: a measured string has the mask's size, a rasterised Latin string has ink, `font_exists` is true for `Liberation Sans` and false for an unknown family.
- [x] 2.3 Implement the process layout: configuration, working directory, signal mask, interpreter, the windowed and headless runners, the wait for a guest that does not answer, the combined exit status. Verify: `cargo build -p vkxp` has no warnings and `cargo xtask deps` reports no violation.
- [x] 2.4 Add the end-to-end test that assembles a small game and runs the built executable headless. Verify: tests for the scenarios of the starting-a-game, exit-status, options, headless-mode, fonts and built-in-scripts-file requirements.
- [x] 2.5 Test a guest that does not end. Verify: a game whose script loops without calling `Graphics.update` after its third frame, run with `--headless --frames 3`, exits with a non-zero status within the wait.

## 3. Documentation

- [x] 3.1 Add "Running a game" to `README.md`: extracting the built-in scripts, the options, windowed and headless examples with the loader restriction, and update the status paragraph. Verify: every command in the section matches the implemented options.

## 4. Integration check

- [x] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-vkxp-executable --strict` passes.
