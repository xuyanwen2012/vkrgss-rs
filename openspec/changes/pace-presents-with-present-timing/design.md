# Design

## Context

- `docs/decisions.md`: "Optional, with fallback: `VK_EXT_present_timing` /
  `VK_KHR_present_wait2` for 40 fps pacing".
- `vkxp-vk` creates the device with `VK_KHR_swapchain` (`device.rs`) and a FIFO swapchain
  (`compositor/present.rs`); `ash` 0.38 (Vulkan headers 1.3.281) has no bindings for
  `VK_EXT_present_timing`, `VK_KHR_present_id2` or `VK_KHR_present_wait2`. The current headers
  are installed on the development machine at `/usr/include/vulkan/vulkan_core.h` and define
  them (search for `VK_EXT_present_timing`); they are the reference for structure layouts,
  `sType` values, enum values and function names.
- On the development machine the AMD integrated GPU (RADV) advertises `VK_EXT_present_timing`
  revision 3, `VK_KHR_present_wait2` and `VK_KHR_swapchain_maintenance1`; llvmpipe, which the
  tests run on, does not have present timing.
- The session (`vkxp-host`) sends ticks by its own clock at the frame period and calls the
  presenter once per frame; fast-forward and a frame reset change its pacing.
- The flow's sandbox cannot open a window; the coordinator checks the result on the AMD GPU.

## Decisions

- Hand-written bindings in one module of `vkxp-vk`: the structures and constants the feature
  needs, with a layout test against sizes and offsets computed in the test from the header's
  definitions transcribed there, and function pointers loaded with `vkGetDeviceProcAddr`.
- Feature detection at device creation: the extension names are enabled only when the device
  lists them and the feature structures report support; the swapchain is created with the
  flags the extension requires; the surface's present-timing capabilities are queried and the
  absolute-time path is used only when reported.
- The schedule is a pure type (`PresentSchedule`): integer nanoseconds, the period as a
  rational (one second over the frame rate) so that 40 fps accumulates without error, restart
  rules as the spec says. It is tested without Vulkan. The time domain and the conversion from
  the swapchain's clock follow the extension (time domain properties and calibration).
- `present_wait2`, when available, is used to wait for the previous frame's visibility with a
  timeout of two periods before submitting the next, so that at most one frame is queued; a
  timeout is not an error.
- The session tells the presenter the frame period with each frame and when pacing restarts
  (frame reset, fast-forward on and off, transition out of a modal wait).
- `--present-stats` prints the summary of the last 600 intervals every 600 frames and at exit
  to the error stream.
- Everything degrades: any error from the extension's calls at run time is reported once and
  the presenter continues without target times.

## Risks / Trade-offs

- [Cannot be exercised by the automated tests] → The schedule, the statistics, the bindings'
  layouts and the no-support path are tested; the live path is checked by hand with
  `--present-stats` on the AMD GPU and documented in the README.
- [Compositors that ignore target times] → The feedback then shows it; nothing breaks.
