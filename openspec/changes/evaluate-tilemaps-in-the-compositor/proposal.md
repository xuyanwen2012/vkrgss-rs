# Proposal

## Why

The design record settles that a tilemap layer is an op evaluated by the compositor per pixel,
not something the guest expands. To reach a playable map quickly an earlier change composed
tilemaps on the CPU into cached bitmaps instead, and recorded that as a temporary deviation.
The pictures are now identical to the original runtime's, measured on real games, so the
deviation can be removed without guessing: the op only has to reproduce the same pixels. The
cache costs a recomposition and a texture upload every time a map scrolls across a tile
boundary or an autotile animates; the op costs a table lookup per pixel.

## What Changes

- **BREAKING** (protocol version 6): frame packets carry tables (the map's tile numbers, the
  tile priorities) as resources beside bitmaps, and `TilemapLayer` becomes a real op that names
  them together with the tileset, the autotile bitmaps, the scroll origin, the map layer, the
  priority and the autotile animation step.
- The CPU reference and the Vulkan compositor evaluate the op per pixel: map cell, tile number,
  priority test, autotile pattern, texel, blend.
- `vkxp-core` emits these ops in place of the cached tile bitmaps and uploads a table only when
  its content changed. The flash overlay keeps its cached bitmap.
- `docs/decisions.md` loses the paragraph about the temporary deviation.

Out of scope: planes as an op (they are exact as stretch quads), the flash overlay.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: table resources and the tilemap op, version 6.
- `frame-composition`: evaluation of the tilemap op.
- `core-scene`: tilemaps are extracted as tilemap ops.

## Impact

- Crates `vkxp-proto`, `vkxp-core` (tilemap extraction, table revisions), `vkxp-compose-ref`,
  `vkxp-vk` (table storage, binning, compute shader), `docs/decisions.md`.
