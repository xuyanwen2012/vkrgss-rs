# Tasks

## 1. Synchronization validation

- [ ] 1.1 `VK_EXT_layer_settings` with `validate_sync` in the test instance; a test that records a hazard on purpose and sees it reported; fix and record every hazard the tests now report. Verify: scenarios of "Clean under validation"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss-vk --locked` passes.

## 2. Submissions

- [ ] 2.1 `vkQueueSubmit2` and a timeline semaphore for composition, uploads, read-back and presentation submits. Verify: scenario "No fence reset for submissions"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss-vk --locked` passes, and with `VK_LOADER_DRIVERS_SELECT='*radeon*' VKRGSS_TEST_DEVICE=RADV` too.

## 3. Uploads

- [ ] 3.1 Batched barriers for uploads, `UNDEFINED` for whole replacements. Verify: scenario "Same frames"; the same two test runs pass.

## 4. Documentation

- [ ] 4.1 `docs/development.md` if it describes validation; `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents and their links pass.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate check-and-tighten-vulkan-synchronization --strict` passes.
