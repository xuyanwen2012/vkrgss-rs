# Tasks

## 1. Backend (`vkxp-vk`)

- [x] 1.1 Add the frame timing with processor durations and the timestamp queries. Verify: the scenarios of "Frame timings on request" in `specs/vulkan-compositor/spec.md`.
- [x] 1.2 Enable debug utils where listed; name objects and label command buffers. Verify: the scenario of "Named objects"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk` passes.

## 2. Host and option (`vkxp-host`, `vkxp`)

- [x] 2.1 Measure the wait for the guest, collect the summaries, add `--frame-stats`. Verify: the scenarios of `specs/host-session/spec.md` and `specs/game-launch/spec.md`.

## 3. Documents

- [ ] 3.1 Describe the option, the rows and the profiling recipe in `README.md` and the usage text. Verify: the usage text test.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate measure-frame-times --strict` passes.
