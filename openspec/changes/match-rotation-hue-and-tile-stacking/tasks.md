# Tasks

## 1. Rotation (`vkxp-core`)

- [x] 1.1 Render rotated sprites by the measured rule into cached bitmaps and extract them as unscaled quads; commit small crops of the measured frames as test data. Verify: the scenarios of the two sprite requirements in `specs/core-scene/spec.md`.

## 2. Hue (`vkxp-pixel`, `vkxp-core`)

- [x] 2.1 Implement the measured hue arithmetic with its provenance and commit a selection of the samples. Verify: the scenarios of `specs/core-bitmap/spec.md`.

## 3. Tiles (`vkxp-core`)

- [ ] 3.1 Keep tilemap layers apart per priority. Verify: the two scenarios of the tile requirement; the existing tilemap tests still pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-rotation-hue-and-tile-stacking --strict` passes.
