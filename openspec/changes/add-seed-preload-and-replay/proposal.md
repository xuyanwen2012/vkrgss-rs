# Proposal

## Why

Three first-version features of the design record are missing: an injectable random seed (runs
of a game that calls `rand` differ from run to run, which defeats frame comparisons), preload
scripts (Ruby patches evaluated before the game's scripts, the way to fix a game without
editing it), and input record and replay (to reproduce a play session exactly). The executable
also does not yet open encrypted archives although `vkxp-vfs` can read them.

## What Changes

- `--seed N`: the interpreter's random number generator is seeded with N before any script.
  Headless runs default to seed 0; windowed runs to a seed from the clock.
- `--preload FILE` (repeatable): Ruby files evaluated after the built-in scripts and before the
  game's first script, again after every reset.
- `--record FILE` writes the input snapshot of every frame and the seed; `--replay FILE` plays
  such a file back instead of the keyboard, in windowed and headless runs.
- The executable opens the game's files with the constructor that reads `Game.rgssad`.

Out of scope: save states, rewind, recording of anything but input and seed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: seed and preload scripts.
- `host-session`: input recording and replay.
- `game-launch`: the new options and encrypted archives.

## Impact

- Crates `vkxp-binding` (run configuration: seed, preload), `vkxp-host` (recorder and replayer
  as input sources), `vkxp` (options, `GameFiles::open`), README.
