# Design

## Context

- `crates/vkxp-core/src/tiles.rs`: constants, `sheet()`, `frames()`, and `pieces()`, an
  iterator of the rectangles a tile is made of. Production code uses `sheet()`, `frames()`,
  the tile size, and `pieces(..).next().is_none()` as an existence test
  (`tilemap.rs`, in the scan of visible cells); the rectangles themselves are consumed only by
  `tilemap/cached.rs`.
- `crates/vkxp-compose-ref/src/tilemap.rs`: the autotile quarter table, the same constants and
  `Layer::texel`, which the shader's `apply_tilemap` mirrors.
- `crates/vkxp-core/src/tilemap/cached.rs` (about 440 lines) is behind `#[cfg(test)]`. The test
  harness (`Stage::frame` in `extract.rs`) clones the scene and the bitmaps, renders the
  cached composition and asserts that it equals the frame rendered from the ops; a property
  test does the same. Production functions in `tilemap.rs` have three `#[cfg(test)]` early
  returns for it, `element.rs` a test-only field, `scene.rs` a hook.
- `vkxp-proto` already holds rules every backend must match: `Sampling::source_texel`,
  `Quad::is_window_tail`, `stretch_step` and `stretch_index` in `frame.rs`.
- `MAX_LOGICAL_DIMENSION` (16384) is defined in `vkxp-core`, `vkxp-compose-ref` and `vkxp-vk`.
  `vkxp_pixel::stretch_index` and `vkxp_proto::stretch_index` are the same rule; the pixel
  crate's documentation still says stretching has no GLSL counterpart.
- `vkxp-vk` has a test that compares the text of the shader's `AUTOTILE_QUARTERS` with the
  reference compositor's table.
- `cargo xtask deps` restricts what `vkxp-core` may depend on.

## Decisions

- A module in `vkxp-proto` holds the tile numbering: the tile and quarter sizes, the autotile
  frame width, the first autotile and first tileset numbers, the quarter table, and `const`
  functions that give, for a tile number, the sheet it reads (tileset or which autotile) and,
  for a pixel inside the tile and an animation frame, the texel. It has the single-tile sheet
  rule and the frame count rule that both crates implement today.
- `vkxp-compose-ref` evaluates a tilemap op through these functions; `vkxp-core` uses them
  where it uses `tiles.rs` today. What remains in `tiles.rs` is what only the guest needs; if
  nothing remains the file goes.
- Order of work, so that no picture changes unnoticed:
  1. While the cached comparison still runs, pin each tilemap test scene that the comparison
     covered: a digest or an expected image of the frame, committed as test data and asserted
     by the test. Scenes are generated procedurally, as everywhere in this crate.
  2. Move the rules to `vkxp-proto` and switch both crates; the comparison and the pinned
     expectations both still pass.
  3. Remove `tilemap/cached.rs`, the three `#[cfg(test)]` branches, the test-only field and
     the hook. The pinned expectations remain. The property test keeps what it asserts
     besides the comparison.
- `MAX_LOGICAL_DIMENSION` moves to `vkxp-proto`; the three crates use it.
- One of the two `stretch_index` functions calls or re-exports the other, in the direction
  the dependency rules allow. If neither crate may depend on the other, both stay and the
  fact goes under Follow-ups. The stale sentence in `vkxp-pixel`'s documentation is corrected
  either way.
- The test in `vkxp-vk` that compares the shader's table reads the table from its new place.

## Risks / Trade-offs

- [The second opinion on extraction is gone] → It compared against code nobody maintains; the
  pinned frames, the reference compositor's own tests, the Vulkan differential tests and the
  comparison with the original runtime remain.
- [A conflict with the change that rewrites `vkxp-vk`] → Only one constant and one test file
  are touched there.

## Follow-ups

- Generating the shader's constants and table from the Rust definition.
