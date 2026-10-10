# Tasks

## 1. Indices and include (`vkxp-vk`)

- [x] 1.1 Name the word indices, write the encoder through them, and move the struct declarations into an include. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes with no frame changed.

## 2. Layout test (`vkxp-vk`)

- [x] 2.1 Add the test shader and the test. Verify: the scenarios of `specs/vulkan-compositor/spec.md`; for "A field moved on one side", swap two fields locally, see the test fail with their names, and undo the swap.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate check-shader-record-layouts --strict` passes.
