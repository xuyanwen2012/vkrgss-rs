# Proposal

## Why

Nothing in vkxp has been compared with the original runtime, so every formula marked
PLACEHOLDER is a guess. The original `Game.exe` with its RGSS DLL has now been shown to run
under Wine in a virtual display on the development machine. A harness that runs the same game,
with the same scripted input, in the original and in vkxp and compares chosen frames turns
those guesses into measurable differences.

## What Changes

- `cargo xtask oracle`: prepares a copy of a game with a probe script added in front of its
  scripts, runs it in the original runtime under Wine and Xvfb and in vkxp headless, captures
  the same frames from both, and writes a comparison report with difference images.
- The probe script, in Ruby that both runtimes accept: fixes the random seed, replaces the
  `Input` queries by a scripted input schedule, counts frames, and at the frames asked for
  holds the game until the harness has captured the screen.
- A minimal reader and writer for the script archive (`Scripts.rxdata`) in `xtask`.
- Nothing of the original runtime, the RTP or any game enters the repository; the harness takes
  their locations as arguments and writes only below an output directory.

Out of scope: fixing any difference found, audio comparison, pass or fail thresholds in the
gate (the harness reports numbers), running the original anywhere but under Wine on Linux.

## Capabilities

### New Capabilities

- `oracle-harness`: comparing vkxp with the original runtime on a real game.

### Modified Capabilities

None.

## Impact

- `xtask` only (new command, new dependencies `flate2`, `x11rb`, `png`), plus a section in
  `README.md`. Needs `wine`, `Xvfb` and the user's own DLL at run time; tests that need them
  are skipped when they are absent.
