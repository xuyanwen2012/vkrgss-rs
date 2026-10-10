# Proposal

## Why

The tilemap's `PLACEHOLDER` rules were measured on the original runtime. Four are confirmed
(repetition beyond the map edges, tile ids below 48, autotile sheets of unusual sizes, the
low 16 bits of a table element). Three differ: the flash overlay pulses between 4 and 8
times its colour nibble and is there from the start, where vkxp ramps between nothing and
full white (all 200 measured frames differ); a tile with a priority outside 0 to 5 is not
drawn where the original survives, and vkxp draws it; and the z of a priority row must not
include the viewport's origin.

## What Changes

- The flash overlay follows the measured pulse, skips negative entries, and finds its cell
  in a flash table of another size as the original does.
- Tiles with a priority outside 0 to 5 are not drawn.
- The z of a priority row is computed from the tilemap's origin alone.
- Confirmed notes are reworded.

Out of scope: the edges of table value conversion and `Table._load`; ties between tiles and
sprites (settled by `match-draw-order-self-transfer-and-colour-wrap`); the protocol, the
compositors and the shader, unless design.md's rules cannot be expressed with today's ops,
which is then a follow-up.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: tile priorities and tilemap flash data.

## Impact

- `crates/vkxp-core` (`tilemap.rs`, its tests and test data), comments in
  `crates/vkxp-proto/src/tiles.rs`.
