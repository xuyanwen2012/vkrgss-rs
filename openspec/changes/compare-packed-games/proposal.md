# Proposal

## Why

Released games usually ship their data inside `Game.rgssad`. The oracle harness refuses such a
game: it adds its probe to a loose `Data\Scripts.rxdata`, and the original runtime reads the
script archive only from `Game.rgssad` when that file exists (measured: a loose script archive
next to it makes the original fail with "failed to load scripts"). So the encrypted archive
reader of vkxp has been checked against the original only by wall-clock screenshots, not frame
by frame.

## What Changes

- The harness reads a game's `Game.rgssad`, replaces the script archive inside it with the
  probed one, and writes the archive back into each side's copy of the game, so that both
  runtimes read everything from the archive.
- `cargo xtask pack-rgssad <game directory>` packs the `Data` and `Graphics` folders of a copy
  of a game into `Game.rgssad` and removes them, to make test material from an unpacked game.

Out of scope: later archive versions, unpacking as a user-facing tool.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oracle-harness`: games with an encrypted archive.

## Impact

- `xtask` only, and a paragraph in `README.md`.
