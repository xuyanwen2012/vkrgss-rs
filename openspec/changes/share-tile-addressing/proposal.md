# Proposal

## Why

Which texel of which sheet a tile number shows is written out four times: in `vkxp-core`
(`tiles.rs`), in the reference compositor (`tilemap.rs`), in the compute shader, and in the
guest's former implementation that composed tilemaps into cached bitmaps
(`vkxp-core/src/tilemap/cached.rs`). The last one is compiled only for tests, where it serves
as a second opinion on the extracted ops; it costs three `#[cfg(test)]` branches inside
production functions, a test-only field, and copies of code that also exists in production.
Each copy carries the autotile table and the same constants, and only tests keep them equal.

## What Changes

- The tile numbering rules (constants, the autotile quarter table, the texel of a tile number)
  get one Rust definition in `vkxp-proto`, next to the sampling rules backends already share.
  `vkxp-core` and `vkxp-compose-ref` use it. The shader keeps its copy, checked by a test.
- The cached-bitmap tilemap implementation and its hooks in production code are removed. The
  pictures its comparison protected are pinned as committed expectations first.
- `MAX_LOGICAL_DIMENSION` is defined once; the two stretch index functions become one where
  the dependency rules allow.

Out of scope: the GLSL side beyond the test that compares its table; table deltas; any change
to what a tilemap draws.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: what the tilemap test scenes are compared with.
- `frame-composition`: one definition of the tile numbering rules.

## Impact

- `crates/vkxp-proto`, `crates/vkxp-core`, `crates/vkxp-compose-ref`, `crates/vkxp-pixel`.
- In `crates/vkxp-vk` only the constant `MAX_LOGICAL_DIMENSION` in `src/records.rs` and the
  test that compares the shader's autotile table with the Rust one (`src/tests/tilemap.rs`);
  another change is rewriting other parts of that crate at the same time.
- No protocol change: nothing that crosses the boundary changes its encoding.
