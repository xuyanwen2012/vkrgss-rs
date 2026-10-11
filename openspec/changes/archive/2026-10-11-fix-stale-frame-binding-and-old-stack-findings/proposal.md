# Fix a stale binding of the presented frame and what an older graphics stack shows

## Why

Running the tests of `vkxp-vk` with the graphics stack of Ubuntu 24.04 (lavapipe of Mesa
25.2.8, validation layer 1.3.275) showed three things. An investigation with the sources of
Mesa and of the validation layer, kept under `target/cc-check/` of the owner's checkout,
gave these verdicts:

1. **A fault of ours, on every driver.** `Presenter::bind` skips writing the descriptor set
   when the three view handles equal the ones it wrote last. `VkCompositor::ensure_target`
   destroys and recreates the frame target when the logical size changes and does not
   invalidate that record. A driver may give the new view the handle of the one just
   destroyed (lavapipe does); the set then still refers to the destroyed view, and the old
   image is shown: two tests read black where the newest frame is white. The validation
   layer wraps handles in unique numbers, so with the layer loaded the handles never repeat
   and the fault is hidden; players run without the layer. Reproduced on the development
   machine: `VK_LOADER_LAYERS_DISABLE='*validation*' VK_LOADER_DRIVERS_SELECT='*lvp*' cargo
   test -p vkxp-vk --locked freeze_during_a_mix` fails
   `freeze_during_a_mix_keeps_the_pixels_shown_at_a_fractional_factor` and
   `..._at_other_factors_and_with_bars`. A game that changes its screen size during a
   transition could show a stale picture.
2. **A fault of the driver.** Mesa before 25.3.4 advertises swapchain maintenance for
   headless surfaces and has no `release_images` for them: `vkReleaseSwapchainImagesEXT`
   calls a null pointer (fixed by Mesa commit c3763b12, in 25.3.4 and 26.0). Our call is
   valid, and the X11, Wayland and display surfaces of those versions implement it, so only
   the tests, which use a headless surface, crash.
3. **A false report of the old layer.** Validation layers before the tracking of present
   fences (fixed over 2024 and 2025; issues 8578 and 9587 of Vulkan-ValidationLayers) report
   `VUID-vkDestroySemaphore-semaphore-05149` when the wait semaphores of a present are
   destroyed after its present fence has signalled, which the specification allows in so
   many words.

## What Changes

- The record of what the presentation set holds is forgotten when the frame target is
  recreated.
- The Vulkan tests are also run without the validation layer, as a step of the merge gate.
- On a headless surface of Mesa older than 25.3.4 an undrawn image is not released; the
  swapchain is replaced instead, as without swapchain maintenance.
- The test harness does not count that one false report of validation layers that do not
  track present fences, and says when it has dropped one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: the frame shown after the frame target was recreated; releasing an
  undrawn image.
- `workspace-structure`: the merge gate gains a step.

## Impact

`crates/vkxp-vk` (compositor, presenter, device, test harness), `xtask/src/gate.rs`,
`docs/development.md`, `AGENTS.md` and `.github/workflows/ci.yml` where they list the steps.
