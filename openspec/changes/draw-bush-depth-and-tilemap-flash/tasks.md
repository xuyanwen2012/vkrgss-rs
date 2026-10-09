# Tasks

## 1. Bush depth (`vkxp-core`)

- [x] 1.1 Extract a sprite with bush depth as two quads. Verify: pixel tests for the three scenarios of the bush-depth requirement in `specs/core-scene/spec.md`, and one with mirror and rotation that compares against the same sprite drawn from two hand-split bitmaps.

## 2. Tilemap flash (`vkxp-core`)

- [ ] 2.1 Draw the flash overlay with its pulse. Verify: pixel tests for the three scenarios of the flash-data requirement; the test for the removed "stored only" requirement is replaced.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-bush-depth-and-tilemap-flash --strict` passes.
