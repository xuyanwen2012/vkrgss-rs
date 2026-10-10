# Tasks

## 1. Device (`vkxp-vk`)

- [ ] 1.1 Enable and check the two Vulkan 1.3 features. Verify: the scenarios of "Device requirements" in `specs/vulkan-compositor/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk` passes.

## 2. Barriers (`vkxp-vk`)

- [ ] 2.1 Replace the layout-derived barriers by explicit synchronization2 barriers at every call site and add the missing ones. Verify: the scenarios of "Barriers name the stages that use an image"; no call of `cmd_pipeline_barrier` and no `layout_use` remain.
- [ ] 2.2 Run the tests once with synchronisation validation as `design.md` says and fix or note the result. Verify: the run's outcome is written under a "Checked" heading at the end of `design.md`.

## 3. Dynamic rendering (`vkxp-vk`)

- [ ] 3.1 Render the presentation pass and the off-screen scaling pass with dynamic rendering; remove render passes, their cache and framebuffers; drop the unused image flag. Verify: the scenarios of `specs/vulkan-presentation/spec.md`; no `create_render_pass` and no `create_framebuffer` remain.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate modernize-vulkan-sync-and-rendering --strict` passes.
