# Tasks

## 1. Presenter (`vkxp-vk`)

- [x] 1.1 Add the filter selector to the present pass. Verify: tests on llvmpipe for the four scenarios of `specs/vulkan-presentation/spec.md`; the existing presentation tests still pass.

## 2. Window (`vkxp-host`)

- [x] 2.1 Add the initial size computation and the fullscreen toggle. Verify: unit tests for the size-computation scenario and for the key handling of the toggle scenario on the event-handling function, without opening a window.

## 3. Options (`vkxp`)

- [x] 3.1 Add `--filter` and `--fullscreen` with their configuration keys and a README paragraph. Verify: tests for the two scenarios of `specs/game-launch/spec.md`.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-scaling-filters-and-fullscreen --strict` passes.
