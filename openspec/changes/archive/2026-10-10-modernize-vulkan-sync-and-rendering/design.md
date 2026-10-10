# Design

## Context

- `src/device.rs` requires Vulkan 1.3 and enables four descriptor indexing features of 1.2;
  no 1.3 feature is enabled. `synchronization2` and `dynamicRendering` are mandatory in 1.3,
  so every device the backend accepts has them. `ash` 0.38 has both interfaces.
- `src/resources.rs` has the barrier helpers: `layout_use` maps a layout to a stage and an
  access mask (`SHADER_READ_ONLY_OPTIMAL` to the compute stage), and the transitions and the
  read-back barrier are built on it with `cmd_pipeline_barrier`.
- `src/compositor.rs`: uploads change a bitmap's layout to transfer destination and back, one
  pair of barriers per upload; the compute dispatch writes the frame target in `GENERAL`.
- `src/compositor/present.rs`: `Presenter::pass` creates a render pass per surface format and
  keeps them in `passes`; `ensure_swapchain` creates a framebuffer per swapchain image; the
  off-screen scaling used by frame dumps (`scale_frame`) creates a framebuffer per call. The
  fragment shader reads the composed frame, the held frame and the mask.
- Every submission is followed by a wait for its fence, so there is one command buffer in
  flight at a time. This change keeps that.
- The tests run with the validation layer and fail on any message (`src/validation.rs`).

## Decisions

- Enable `synchronization2` and `dynamic_rendering` through `PhysicalDeviceVulkan13Features`,
  and check them in device selection like the other features, with the same kind of message
  for a device that lacks one.
- Replace `layout_use` by barrier helpers that take the source and destination stage, access
  and layout explicitly (`ImageMemoryBarrier2`, `cmd_pipeline_barrier2`). Each call site names
  what really happens before and after: transfer write, compute sampled read, compute storage
  write, fragment sampled read, colour attachment write, present.
- Add the barrier between the compute write of the frame target and the fragment read, and
  make the held frame's and the mask's barriers name the fragment stage where that is the
  reader. An image read by both stages names both.
- The presentation pass and the off-screen scaling pass use `cmd_begin_rendering` with the
  image view, load and store operations of the former render pass; the pipeline is created
  with `PipelineRenderingCreateInfo` for the colour format. The pipeline cache per format
  stays if pipelines still differ by format; the render pass cache and the framebuffers go.
  With dynamic rendering the swapchain image's layout changes are barriers of this crate,
  recorded around the pass.
- Drop `MUTABLE_FORMAT` from the frame target.
- Check with synchronisation validation: run the crate's tests once with the validation
  layer's synchronisation checks enabled (through the layer settings the validation set-up of
  the tests can pass) on llvmpipe, and fix what it reports about this crate's commands. If
  the installed layer cannot be configured that way from the tests, note under Follow-ups how
  far it got, and go on.
- Pixels do not change: the differential tests against the reference compositor are the
  check.

## Risks / Trade-offs

- [A barrier that the render pass used to provide implicitly is forgotten] → The validation
  layer reports layout mismatches, and the tests fail on any message.
- [Different behaviour on the AMD driver] → The owner's side runs the crate's tests on RADV
  after the merge; nothing for the flow to do.

## Follow-ups

- Batching the barriers of several uploads to one bitmap.

## Checked

Synchronisation validation, on llvmpipe (Mesa lavapipe) with `VK_LAYER_KHRONOS_validation`
1.4.341:

- The layer takes its settings from the environment, so nothing was added to the validation
  set-up of the tests. The run was
  `VK_LOADER_DRIVERS_SELECT='*lvp*' VK_LAYER_VALIDATE_SYNC=1 VK_LAYER_SYNCVAL_SHADER_ACCESSES_HEURISTIC=1 cargo test -p vkxp-vk`.
  The second setting is needed: without it this version of the layer does not look at what a
  shader reads or writes through a descriptor, which is most of what this crate does.
  (`VK_VALIDATION_VALIDATE_SYNC`, `VK_KHRONOS_VALIDATION_VALIDATE_SYNC` and `VK_LAYER_ENABLES`
  did not show any effect on this version.)
- That the settings take effect was checked with a deliberate fault that was not committed:
  with the barrier in front of the compute dispatch naming the transfer stage instead of the
  compute stage, 10 of the 12 tests of `tests::compositing` fail with
  `vkCmdDispatch(): WRITE_AFTER_WRITE hazard detected`; without the two settings the same
  fault passes unnoticed.
- Result after the barriers of task 2.1: all 218 tests pass, no message.
- The same run after task 3.1, with the passes rendered by dynamic rendering and the layouts
  of the swapchain image and the off-screen target changed by this crate's barriers: all 218
  tests pass, no message.
- Limit of the check: the code before this change also passes it (run on the commit of task
  1.1). Every submission is waited for through its fence before the next one is recorded, and
  the layer then takes the earlier accesses as finished, so the stages that the old barriers
  got wrong between two submissions (fragment reads taken for compute reads, no hand-over of
  the composed frame) are not something it can report. Those are right by the stages each
  call site now names, not by this run. The accesses of the bitmap array, which is bound
  partially and indexed at run time, are also outside what the layer's heuristic follows.
