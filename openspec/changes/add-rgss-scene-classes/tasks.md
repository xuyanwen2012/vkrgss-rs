# Tasks

## 1. Shared machinery (`vkxp-binding`)

- [x] 1.1 Give the `Color`, `Tone` and `Rect` wrappers working owners (scene handle and property tag) with write-through on every mutation, and make the `Table` wrapper hold a `SharedTable`. Verify: the existing Ruby tests of the value classes still pass, and a unit test shows that a value object keeps working after its owner element is disposed.
- [x] 1.2 Add the test-only helpers `pixel(x, y)` (extract, render with the reference compositor, read) and `scene_count(kind)`. Verify: a harness test draws a 1 by 1 bitmap through a sprite created from Rust and reads its colour from Ruby.

## 2. Sprite, Viewport and Plane (`vkxp-binding`)

- [x] 2.1 Implement `Viewport` (both constructors, `rect`, `visible`, `z`, `ox`, `oy`, `color`, `tone`, `flash`, `update`, `dispose`, no `disposed?`). Verify: Ruby tests for the viewport cases of the construction, plain-property, object-property and disposal requirements in `specs/rgss-scene-classes/spec.md`; class added to the arity test.
- [x] 2.2 Implement `Sprite` with all its properties, `bitmap` by reference, `src_rect` reset on assignment, `flash`, `update`, `dispose`, `disposed?`. Verify: Ruby tests for every scenario that names a sprite; class added to the arity test.
- [x] 2.3 Implement `Plane`. Verify: Ruby tests that a plane with a bitmap tiles the screen (two pixels one bitmap width apart are equal) and for its object properties and disposal; class added to the arity test.

## 3. Window and Tilemap (`vkxp-binding`)

- [x] 3.1 Implement `Window` with `windowskin` and `contents` by reference, `cursor_rect` as an owned object, and its flags, positions, sizes and opacities. Verify: Ruby tests for the cursor-blink scenario, for contents appearing 16 pixels inside the window, and for disposal; class added to the arity test.
- [x] 3.2 Implement `Tilemap` and `TilemapAutotiles` with shared tables. Verify: Ruby tests for the autotile-list and table-change scenarios and for a tileset tile appearing at its cell; both classes added to the arity test, which now covers the whole list.

## 4. Lifetime (`vkxp-binding`)

- [ ] 4.1 Refuse `clone` and `dup`, and free unreachable elements through the deferred queue. Verify: Ruby tests for the no-copies scenario and the two unreachable-element scenarios, and for a bitmap kept alive only by a sprite.
- [ ] 4.2 Run all Ruby tests of this change under forced garbage collection. Verify: same results as without it.
- [ ] 4.3 Document the element classes, owned value objects and lifetime rules in the crate documentation. Verify: `cargo doc -p vkxp-binding --no-deps` builds without warnings.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-rgss-scene-classes --strict` passes.
