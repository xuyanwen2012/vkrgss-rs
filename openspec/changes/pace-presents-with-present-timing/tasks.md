# Tasks

## 1. Schedule and statistics (`vkxp-vk`)

- [ ] 1.1 Implement the present schedule and the interval statistics as pure types. Verify: the first three scenarios of the target-times requirement and the scenario of the statistics requirement in `specs/vulkan-presentation/spec.md`.

## 2. Extension (`vkxp-vk`)

- [ ] 2.1 Add the hand-written bindings with their layout test, feature detection and device and swapchain setup. Verify: the layout test, and the no-support scenario on llvmpipe (all existing presentation tests pass).
- [ ] 2.2 Attach target times to presents, read the feedback, and use the present wait when available, with the run-time fallback. Verify: `cargo build` without warnings, the no-support scenario, and a test of the fallback path with a stand-in that fails the extension call.

## 3. Host and option (`vkxp-host`, `vkxp`)

- [ ] 3.1 Pass the frame period and pacing restarts to the presenter and add `--present-stats`. Verify: session tests that the presenter is told the period and the restarts; an argument test; a README paragraph that matches the option.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate pace-presents-with-present-timing --strict` passes.
