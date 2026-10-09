# Tasks

## 1. Scene (`vkxp-core`)

- [x] 1.1 Dispose the elements of a viewport with it, and allow elements in a disposed viewport. Verify: unit tests that the four kinds of element are disposed with their viewport, that an element created afterwards is alive and draws nothing, and that other elements are untouched.

## 2. Classes (`vkxp-binding`)

- [x] 2.1 Let `Sprite#bitmap` and `Tilemap#autotiles` work on disposed elements and let constructors accept a disposed viewport. Verify: Ruby tests for the four measured scenarios of `specs/rgss-scene-classes/spec.md`; the other scenarios still pass.

## 3. Integration check

- [x] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-disposed-element-behaviour --strict` passes.
