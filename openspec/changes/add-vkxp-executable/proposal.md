# Proposal

## Why

Every part needed to run a game exists as a library: the host (window, presentation, pacing,
input, headless runs), the guest (interpreter, binding, `run_game`) and the font engine. The
`vkxp` program that puts them on their threads and connects them does not, so no game can be
started.

## What Changes

- The `vkxp` executable: parses its command line, reads the game's configuration, starts the
  interpreter, runs the game's scripts on the interpreter thread and the host on the main
  thread, joined by the protocol link, and exits with a status that says how the game ended.
- Windowed mode by default; headless mode with a frame limit, scripted input and frame dumps
  for automated runs.
- Real fonts: the adapter that gives the binding its text backend from the `vkxp-text` engine,
  fed with the system's fonts, the game's `Fonts` directory and directories named on the
  command line.
- Options for the things that have no configuration file yet: where the RTPs are, where the
  built-in scripts file is, which GPU to use, font directories, debug mode, the text encoding of
  `Game.ini`, process functions.
- A tool, `cargo xtask extract-rpg`, that copies the built-in `RPG` script source out of an
  original RGSS DLL into a local file the executable loads. The source itself is never
  committed.
- An end-to-end test with a small game assembled by the test itself.

Out of scope: a configuration file, gamepads, audio output (audio commands are accepted and
dropped by the host), packaging.

## Capabilities

### New Capabilities

- `game-launch`: starting a game from the command line, windowed or headless, with real fonts,
  and the tool that provides the built-in scripts.

### Modified Capabilities

None.

## Impact

- Crate `vkxp` (the binary) and `xtask` (one new command). `README.md` gains a section on
  running a game.
- Running a real game needs a file extracted from the user's own RGSS DLL; the README says how.
- On the development machine every run restricts the Vulkan loader to the AMD GPU or to
  llvmpipe, as `AGENTS.md` requires.
