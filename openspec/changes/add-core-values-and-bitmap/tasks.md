# Tasks

## 1. Value types (`vkxp-core`)

- [x] 1.1 Implement `Color` and `Tone` (floating-point components, clamping setters, default alpha 255 and grey 0, equality) and their conversion to the protocol's integer types by discarding fractions. Verify: `cargo test -p vkxp-core` covers every scenario of the colour, tone and conversion requirements in `specs/core-values/spec.md`.
- [x] 1.2 Implement `Rect` (four `i32` stored as given, emptying, equality) and its conversion to a protocol rectangle (non-positive sizes become empty). Verify: tests for the rectangle scenarios and for the conversion of negative sizes.
- [x] 1.3 Implement `Table` (one to three dimensions, zero-initialised, bounds-checked reads returning no value, ignored out-of-range writes, 16-bit wrap, resize). Verify: tests for every scenario of the table and table-resize requirements.
- [x] 1.4 Implement the serialized forms of `Color`, `Tone`, `Rect` and `Table` with a typed error for malformed input. Verify: tests for the known colour bytes, the 1820-byte map table and its header, the element order, wrong lengths and an inconsistent header; property tests that every value round-trips.
- [x] 1.5 Implement `Font` and `FontDefaults` (names, size, bold, italic, colour; defaults size 22, not bold, not italic, opaque white; family names given at construction). Verify: tests for the three font scenarios.

## 2. Bitmap arithmetic (`vkxp-pixel`)

- [x] 2.1 Add a module for CPU-only bitmap arithmetic with `stretch_index` and `rotate_hue` as specified in `design.md`, each documented with its formula and a provenance line. Verify: unit tests that an angle of 0 leaves every tested pixel unchanged (exhaustively over two channels, sampled over the third), that 120 and 240 degrees map pure red to pure green and blue, that grey pixels and alpha never change, that the largest and smallest channel values are preserved for random pixels and angles, and that `stretch_index` matches the doubling and halving scenarios.
- [x] 2.2 Add the separate provenance list and `bitmap_provenance()`. Verify: a test asserts both new functions are listed as placeholders and that `provenance()` is unchanged in length and names; `cargo test -p vkxp-pixel` and `cargo test -p vkxp-vk --lib tests::shaders` both pass.

## 3. Bitmap (`vkxp-core`)

- [ ] 3.1 Implement the `Bitmaps` arena with creation (blank and from pixels), the creation limit, disposal, the disposed error and the per-bitmap font. Verify: tests for every scenario of the creation, font and disposal requirements in `specs/core-bitmap/spec.md`, and a test (using the development-only dependency) that the limit equals `vkxp_compose_ref::MAX_BITMAP_PIXELS`.
- [ ] 3.2 Implement `get_pixel`, `set_pixel`, `clear` and `fill_rect`. Verify: tests for every scenario of the pixel-access and clear-and-fill requirements.
- [ ] 3.3 Implement `blt` with clipping against source and destination and the same-bitmap case. Verify: tests for every block-transfer scenario, plus a property test comparing against a naive per-pixel model for random rectangles, positions and opacities.
- [ ] 3.4 Implement `stretch_blt`. Verify: tests for the three stretched-transfer scenarios and for empty rectangles.
- [ ] 3.5 Implement `hue_change`. Verify: tests for the four hue scenarios at bitmap level.

## 4. Text (`vkxp-core`)

- [ ] 4.1 Define the `TextRasterizer` interface and a deterministic stand-in rasteriser for tests (not exported from the crate's public API outside a test-support module). Verify: the stand-in's `rasterize` output always has the size its `measure` reports, asserted by a test over several strings and fonts.
- [ ] 4.2 Implement `draw_text` (alignment, vertical centring, squeezing to no less than 60 percent, clipping, coverage and colour alpha). Verify: tests for the five text-drawing scenarios using the stand-in.
- [ ] 4.3 Implement `text_size`. Verify: tests for the two measurement scenarios.

## 5. Deltas (`vkxp-core`)

- [ ] 5.1 Track per bitmap what the host knows and the rectangle changed since the last update, updated by every drawing operation. Verify: tests that each operation marks exactly the pixels it may have changed (for example `set_pixel` marks one pixel, a clipped `fill_rect` marks only its visible part).
- [ ] 5.2 Implement `deltas(referenced)` in the order given in `design.md`. Verify: tests for the five bitmap-delta scenarios and the slot-reuse scenario.
- [ ] 5.3 Add the property test that applies emitted deltas to `vkxp_compose_ref::BitmapStore`. Verify: for random sequences of creation, drawing, disposal and delta requests the store never rejects a delta and every referenced bitmap equals the arena's pixels byte for byte.

## 6. Integration check

- [ ] 6.1 Document the crate's modules in `crates/vkxp-core/src/lib.rs` and run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-core-values-and-bitmap --strict` passes.
