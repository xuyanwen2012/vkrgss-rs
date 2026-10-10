# Proposal

## Why

`Input.repeat?`, `Input.dir4` and the lower bound of colour components were written from
common descriptions and marked `PLACEHOLDER`. They have now been measured on the original
runtime (RGSS103J under Wine, keys injected at exact updates, 18920 logged updates, and a
`Marshal` probe), and two of the three differ from what vkxp does: only one button repeats
at a time, the four-way direction on a diagonal follows a single persistent flag and not the
order of the presses, and a colour clamps its components from below as well.

## What Changes

- `InputState` keeps one repeating button for the whole module, and one axis flag for `dir4`.
- A colour clamps red, green and blue to -255..255 and alpha to 0..255 whenever a component
  is set.
- The oracle's probe script, which answers `Input` from the schedule on both sides, follows
  the same two input rules.
- The `PLACEHOLDER` notes that the measurement confirmed are replaced by plain statements:
  the `_dump` layout of `Rect`, the low 16 bits kept by a table element, the window's
  `active` and `pause` assignments, the autotile animation step, and the single-button repeat
  timing.

Out of scope: the lenient `_load` of the original for strings of a wrong length; gamepads
and several keys on one button, which were not measured.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-input`: repeat and four-way direction as measured.
- `core-values`: the bounds of a colour.
- `rgss-values`: the scenario that shows the clamping.

## Impact

- `crates/vkxp-core` (`input.rs` and its tests, `color.rs` and its tests, comments in
  `rect.rs`, `table.rs`, `element.rs`), `xtask/src/oracle/probe.rb`, tests of
  `crates/vkxp-binding` that state the old colour bound or the old input rules.
