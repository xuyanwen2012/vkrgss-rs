# Design

## Context

- `validation.rs` builds the debug messenger; the layer is enabled for tests when installed.
- `compositor/submit.rs` (`submit_and_wait`) and `compositor/present/show.rs` submit with
  `queue_submit` and a fence, wait, then reset. `compositor/present/swapchain.rs` has a
  fence per present for `VK_EXT_swapchain_maintenance1`; that one is not part of this
  change.
- `resources.rs::transition` records one `vkCmdPipelineBarrier2` per image; it is called in
  14 places. `ash` 0.38 has `vk::LayerSettingsCreateInfoEXT`.

## Decisions

- **Synchronization validation.** When the instance is created for tests with the
  validation layer, chain `VkLayerSettingsCreateInfoEXT` with the setting
  `validate_sync = true` for `VK_LAYER_KHRONOS_validation` (and, if the layer version has
  it, `syncval_shader_accesses_heuristic` left at its default). Enable
  `VK_EXT_layer_settings` only if the layer lists it; otherwise run without and say so once
  in the log. Hazards are reported as validation errors (`VUID-...` with `SYNC-HAZARD-*`),
  which already fail a test. GPU-assisted validation is not enabled here (slow; a later
  step of the nightly checks).
- **Fix what it finds.** Each hazard is fixed in the code with the narrowest correct
  barrier; record each in `design.md` under a heading "Hazards found" with the test that
  showed it.
- **Timeline semaphore.** One timeline semaphore per submitting object (or per device, if
  simpler and correct), a counter incremented per submission, `vkQueueSubmit2` with a
  `VkSemaphoreSubmitInfo` signal at `ALL_COMMANDS`, and `vkWaitSemaphores` for the wait. A
  submission that waits for an acquired swapchain image uses a binary semaphore wait with
  the stage that first touches the image (the colour attachment output or the transfer
  stage, whichever the commands use first). Timeline semaphores are core in 1.2, so no
  new requirement on the device; check the feature `timelineSemaphore` is enabled and add
  it to what device selection requires only if it is not core-mandatory in 1.3 (it is
  mandatory in 1.2, so it is).
- **Batched barriers.** A function that records the transitions of several images in one
  `DependencyInfo`; uploads use it before and after their copies. An upload that replaces
  the whole image transitions it from `UNDEFINED`.
- **Proof that pixels do not change.** The existing differential tests against
  `vkrgss-compose-ref` and the frame tests pass unchanged; no expected value is edited.

## Out of scope

`VK_KHR_swapchain_maintenance1` (needs bindings `ash` 0.38 lacks), timing properties and
variable refresh, GPU-assisted validation, best-practice validation.

## Follow-ups

None yet.
