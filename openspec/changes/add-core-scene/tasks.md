# Tasks

## 1. Scene arena and elements (`vkxp-core`)

- [x] 1.1 Add the `Scene` arena with generation-carrying handles for sprites, viewports, windows, planes and tilemaps, creation with or without a viewport, disposal and the disposed error. Verify: tests for the use-after-disposal scenario for each kind and for a stale handle after its slot is reused.
- [x] 1.2 Add the properties of sprite, viewport and window with their RGSS1 defaults and clamping (opacity values 0 to 255). Verify: tests for the sprite-defaults scenario, equivalent default checks for viewport and window, and the opacity-clamp scenario.
- [x] 1.3 Add planes and tilemaps as property holders. Verify: tests that their properties read back what was written and the plane scenario of `specs/core-scene/spec.md` (rendered with the reference compositor).

## 2. Order and geometry (`vkxp-core`)

- [x] 2.1 Implement the draw order with z and stamps, for top-level elements, viewports and the elements inside a viewport. Verify: tests for the four draw-order scenarios, asserting rendered pixels.
- [x] 2.2 Add `libm` (workspace dependency, allow-list entry in `xtask/src/deps.rs` with a comment) and implement the transform derivation of `design.md`, exact for multiples of 90 degrees. Verify: `cargo xtask deps` reports no violation; unit tests that angles 0, 90, 180 and 270 give coefficients of exactly 0, 1 or -1 and that the derived map sends pixel centres to the expected source positions for a zoomed and a mirrored case.

## 3. Sprites (`vkxp-core`)

- [x] 3.1 Extract a sprite as a quad: position, origin, source rectangle including the part outside the bitmap, missing or disposed bitmap, invisibility. Verify: tests for the four sprite-placement scenarios.
- [x] 3.2 Apply zoom, rotation and mirror. Verify: tests for the four scenarios of that requirement and for zoom 0.
- [x] 3.3 Apply opacity, blend type, tone and colour. Verify: tests for the sprite-appearance scenarios.
- [x] 3.4 Implement flash and sprite update. Verify: tests for the three sprite-flash scenarios.

## 4. Viewports (`vkxp-core`)

- [x] 4.1 Extract the elements of a viewport with offset, origin, clip and visibility, at the viewport's place in the order. Verify: tests for the four viewport scenarios.
- [x] 4.2 Emit the viewport's tone, colour and flash as a rectangle effect, and implement viewport update. Verify: tests for the two viewport-effect scenarios and that a neutral viewport emits no rectangle effect.

## 5. Windows (`vkxp-core`)

- [x] 5.1 Build and cache the background and frame bitmaps from the skin, invalidated by skin changes, size and the stretch flag, and emit them with the two opacities. Verify: tests for the four frame-and-background scenarios, plus tests that changing the size, the stretch flag and a pixel of the skin each change the rendered window.
- [x] 5.2 Emit the contents with origin, clip and contents opacity at z plus 2. Verify: tests for the three window-contents scenarios.
- [x] 5.3 Build the cursor bitmap, place it behind the contents, and implement the blink with window update. Verify: tests for the four cursor scenarios.
- [x] 5.4 Emit the pause sign and the scroll arrows. Verify: tests for the two scenarios of that requirement and that cached bitmaps are disposed with the window (the arena's bitmap count returns to its earlier value).

## 6. Extraction and bookkeeping (`vkxp-core`)

- [x] 6.1 Implement frame count, frame rate and logical size, and assemble the packet with the deltas for exactly the referenced bitmaps. Verify: tests for the frame-bookkeeping scenarios and the unused-bitmap scenario.
- [x] 6.2 Add the property test over random scenes and operations. Verify: every extracted packet is rendered by `vkxp_compose_ref::Compositor` without error, across at least a few hundred generated cases.

## 7. Input (`vkxp-core`)

- [x] 7.1 Implement `InputState` with `receive`, `update`, pressed, triggered and repeated. Verify: tests for every scenario of the first three requirements in `specs/core-input/spec.md`.
- [x] 7.2 Implement the four-way and eight-way direction queries. Verify: tests for the three direction scenarios and for all four diagonals.

## 8. Integration check

- [ ] 8.1 Extend the crate documentation in `crates/vkxp-core/src/lib.rs` with the scene, extraction and input modules, and run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-core-scene --strict` passes.
