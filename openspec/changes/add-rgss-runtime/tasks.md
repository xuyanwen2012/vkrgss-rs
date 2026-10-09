# Tasks

## 1. Guest plumbing (`vkxp-binding`)

- [x] 1.1 Add the scene, the input state, the guest end of the link and `Guest::pump` to the guest state, and the scripted host to the test harness. Verify: a harness test sends three frames through `pump` and the scripted host records three frames and has sent three ticks.
- [x] 1.2 Write the Ruby prelude (`$KCODE`, load path, `$DEBUG`, `$BTEST`, `Reset`, `Hangup`, `save_data`, `Win32API`, disabled process functions, restricted `require` and `load`) and its evaluation at start-up. Verify: Ruby tests for the interpreter-preparation, reset-class, process-function, loading-code and Win32API scenarios in `specs/rgss-runtime/spec.md`.

## 2. Modules (`vkxp-binding`)

- [x] 2.1 Implement `Graphics` (`update`, `freeze`, `transition`, `frame_reset`, `frame_rate`, `frame_count`, the stored `visible` flag). Verify: tests for the scenarios of the three Graphics requirements in `specs/rgss-modules/spec.md`; `Graphics` added to the arity test.
- [x] 2.2 Implement `Input` with its constants. Verify: tests for the input scenarios and the button-constants scenario; `Input` added to the arity test.
- [x] 2.3 Implement `Audio` with the duplicate check for music and background sound. Verify: tests for the four audio scenarios; `Audio` added to the arity test.
- [x] 2.4 Implement `load_data`, `print` and `p`. Verify: tests for the data-file and message-output scenarios.

## 3. Running a game (`vkxp-binding`)

- [x] 3.1 Implement `run_game` up to the evaluation of the scripts: configuration file, title, prelude, built-in scripts file, archive, `$RGSS_SCRIPTS`, section names. Verify: tests for the built-in-scripts, script-archive and title scenarios, with archives assembled in the test.
- [x] 3.2 Implement the end of the game and the error report. Verify: tests for the end-of-game and uncaught-error scenarios, asserting the report text and the modal and shutdown messages.
- [x] 3.3 Implement reset and quit. Verify: tests for the reset scenario (a script that counts its own evaluations sees 2 after one reset and the scene is empty) and the quit scenario of `Graphics.update`.
- [x] 3.4 Document `run_game`, the prelude and the pump in the crate documentation. Verify: `cargo doc -p vkxp-binding --no-deps` builds without warnings.

## 4. Integration check

- [x] 4.1 Run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-rgss-runtime --strict` passes.
