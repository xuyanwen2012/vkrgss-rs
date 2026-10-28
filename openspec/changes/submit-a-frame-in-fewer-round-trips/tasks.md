# Tasks

## 1. Measure before (`vkrgss-vk`, no code change)

- [ ] 1.1 Record the frame times of the chosen game and route with `--frame-stats` on llvmpipe (`VK_LOADER_DRIVERS_SELECT='*lvp*'`) and on RADV (`VK_LOADER_DRIVERS_SELECT='*radeon*' VKRGSS_TEST_DEVICE=RADV`) in `design.md` under "As built". Verify: the numbers and the command lines are in `design.md`.

## 2. `render` does not wait (`vkrgss-vk`)

- [ ] 2.1 Submit the compose without waiting, keep its timeline value as pending, and make every method that records, reads back, replaces or destroys, `has_frame` readers and `Drop` wait for it first, as `design.md` says. Verify: a new test per public method of `VkCompositor` that calls it right after `render` and checks the result against the reference; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss-vk` passes with validation and no new validation message.
- [ ] 2.2 Restate the contract in the doc comment of `VkCompositor` and in `render`'s errors section. Verify: the "Restated contract" scenario of `specs/vulkan-compositor/spec.md`.

## 3. `present` and a freeze wait once (`vkrgss-vk`)

- [ ] 3.1 Record the presentation pass into a second command buffer, submit it after the compose and wait once for it. Verify: the "One wait per shown frame" scenario (a test counts the host waits of `render` followed by `present` on the headless surface); the presentation tests pass.
- [ ] 3.2 Record a freeze's clear and copy into one submission. Verify: the freeze tests of `present/tests.rs` pass; a test counts one wait per freeze.

## 4. Measure after and integration check

- [ ] 4.1 Record the frame times after, with the same commands, under "As built". Verify: the numbers are in `design.md`.
- [ ] 4.2 Run the AMD GPU tests. Verify: `VK_LOADER_DRIVERS_SELECT='*radeon*' VKRGSS_TEST_DEVICE=RADV cargo test -p vkrgss-vk` passes.
- [ ] 4.3 Run the full gate and the cross-architecture check. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` reports every game identical, and `openspec validate submit-a-frame-in-fewer-round-trips --strict` passes.
