## Why

Milestone V2 of VX is walking on a map, and the tile system of VX shares nothing with that
of XP but the cell size: nine tile sheets instead of a tileset and seven autotiles, other
tile numbers, three kinds of autotile shapes, two fixed planes instead of priorities by
row, shadows and table edges drawn by the class, another blend, another animation and
flash. It was measured on the original runtime of VX on 2026-10-12
(`vx-v2-tilemap-REPORT.md` with its model under `ref/tools/oracle/measurements/`, outside
the repository): 1071 frames, 242 856 960 pixels, none wrong, among them 140 held-out random
frames and two real maps of the test game, on both DLL versions.

## What Changes

- In RGSS2 the class `Tilemap` has `bitmaps` (a `Tilemap::BitmapArray` of nine slots),
  `passages` and no `tileset`, `autotiles`, `priorities`; `TilemapAutotiles` is not
  defined.
- A new op kind draws one plane of a tilemap of RGSS2. The CPU reference and the Vulkan
  compositor both evaluate it from the tables and the nine bitmaps, as the decisions ask.
- The protocol version goes up by one.
- XP is drawn exactly as before.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: the op of a tilemap plane of RGSS2.
- `frame-composition`: how that op is evaluated.
- `core-scene`: the tilemap of RGSS2 in the scene.
- `rgss-scene-classes`: the class by runtime.

## Impact

- `crates/vkxp-proto`, `crates/vkxp-core` (`tilemap.rs`, extraction),
  `crates/vkxp-compose-ref`, `crates/vkxp-vk` (shader, records, binning),
  `crates/vkxp-binding` (`tilemap.rs`, the method list and its test), `README.md` where it
  names the protocol version, `docs/architecture.md`, `docs/compatibility.md`.
