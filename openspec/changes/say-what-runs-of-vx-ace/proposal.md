## Why

The documents, the README and the notice of a run of VX Ace say that the classes of a game of
VX Ace are those of VX still and that a game made with the standard scripts ends with an
error before its title screen. Since the changes that bound the classes of RGSS3 (values,
window, tilemap, scene classes and `Graphics`, audio, font and text, input) that is no longer
true: on 2026-10-14 the main test game of the corpus, run headless with its own DLL's
built-in scripts, showed its title screen and its menu, started a new game and ran 1500
frames without an error. No frame has been compared with the original yet, so VX Ace stays
experimental.

## What Changes

- `docs/compatibility.md`, `docs/running.md`, the README's table of runtimes and the notice
  of a run say that the classes are those of RGSS3 and that no frame was compared with the
  original; none says that a game ends before its title screen.
- The scenario "What experimental means here" says so.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-runtimes`: the requirement "State of VX Ace once its games start".

## Impact

`docs/compatibility.md`, `docs/running.md`, `README.md`, `crates/vkxp/src/main.rs` (the text
of one warning).
