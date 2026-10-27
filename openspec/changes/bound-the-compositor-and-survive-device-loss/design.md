# Design

## Context

- `crates/vkrgss-vk/src/compositor/render.rs:120-165`: `prepare` checks the frame size, the
  table bytes and the record bytes against device limits, then bins with the cap
  `max_storage_buffer_range / 4` (`binning.rs` allocates `vec![0u32; needed]`;
  `buffers.rs:55-94` grows a host-visible buffer to match).
- `compositor/submit.rs:259` waits on the timeline with `u64::MAX`.
- `error.rs`: `VkError::Vulkan { call, result }` carries every Vulkan failure; `Limit` lists
  `BitmapSlots`, `BitmapDimension`, `FrameDimension`, `FrameDataBytes`, `TableBytes`.
- `crates/vkrgss-host/src/session.rs:1033-1046` reports a rejected frame and goes on; in a
  headless run `fail_on_rejected_frame` makes the session fail.
- `present/lifecycle.rs:36-47` returns `idle.map_err(...)?` before `gpu.release_surface()`.
- `compositor/buffers.rs:195-233` `ensure_target`; `render.rs:27-28` promises the previous frame
  on an error.
- `buffers.rs:66-74` frame buffers double up to `limit`, `u64::MAX` for the staging buffer.
- `present/show.rs:45,105-111`: `unshown_frame` is taken at the start and restored only on a
  timed-out acquire.
- `device/instance.rs:290-293`, `device.rs:188`: `max_update_after_bind_sampled_images` is the
  minimum of three limits, compared with `BITMAP_SLOTS` alone.

## Decisions

- **Work budget.** A new `Limit::FrameWork` bounds the tile-list entries of one frame (the sum
  over ops of the tiles each covers, which `bin` already counts in its first pass). The check
  comes before the index list or any buffer is allocated. The budget is a constant of the
  backend: the actor measures the largest count of the regression games (the ten games of
  `cargo xtask xarch`, or the oracle routes, through a temporary log) and sets the budget at
  least 64 times above it and at most what keeps the index list under 256 MiB; both numbers go
  into this file under "As built". A frame over it is refused like any limit error (the
  existing "Device limits are reported" requirement), so the session keeps the previous image
  and goes on.
- **Bounded waits.** Every wait for a submission of the compositor (composition, upload,
  read-back, presentation draw) uses a timeout, a constant of some seconds that no valid frame
  under the budget reaches on llvmpipe. A wait that runs out is a lost device: nothing that the
  submission uses may be destroyed or reused, and the compositor cannot know when it ends.
- **Lost device.** `VkError::DeviceLost { call }` is returned for `VK_ERROR_DEVICE_LOST` from
  any call and for a wait that ran out; `VkError::is_device_lost()` tells it. Once lost, the
  compositor answers every later call that would touch the device with the same error and does
  not touch it. Dropping a lost compositor does not wait for the device to be idle without a
  bound (it may skip the wait and leak, the process is about to end). Re-creating the device is
  out of scope.
- **Session.** In `Session`, a compositor error with `is_device_lost()` is reported once with
  severity error and ends the session with a failure status, in windowed and in headless runs,
  instead of the "rejected frame" path. Kept to the place where the compositor's result is
  matched, so the edit to `vkrgss-host` is small.
- **Releasing the surface.** `release_surface` destroys the swapchain and calls
  `gpu.release_surface()` whatever `vkDeviceWaitIdle` returned, then returns that error.
- **Previous frame.** `ensure_target` allocates the new image first; only when it exists are
  the old image destroyed, the descriptor rewritten, `has_frame` cleared and the presenter told
  to forget its binding. A failure leaves target and `has_frame` as they were. A test uses the
  existing resource probe (`probe.rs`) to fail the allocation of the frame target.
- **Staging buffer.** The staging buffer (the one whose `limit` is `u64::MAX`) is replaced by
  one of the size needed when it is more than four times larger than every upload of the last
  sixty frames and larger than 64 MiB; the numbers are constants beside the code.
- **Unshown frame.** Every path of `present` that returns without showing puts `frame` back
  into `unshown_frame`.
- **Descriptor limits.** The check counts every descriptor of the composition set against
  `maxPerStageUpdateAfterBindResources` (bitmap slots plus the other bindings of the stage) and
  the bitmap slots against the two sampled-image limits; a device that falls short is listed as
  unsuitable with the numbers. The devices in use today (llvmpipe, RADV, MoltenVK on Apple M4,
  the Mali of the Pixel 7a, the Adreno of the Galaxy S24+) must stay suitable: the actor checks
  the limits of llvmpipe and RADV with `vulkaninfo` (under the loader restriction) and of the
  others from public gpuinfo reports for those GPUs; if one of them would fall short, the
  check is not tightened, the finding goes under "Follow-ups" and task 3.4 stays open.
- **Tests.** In `crates/vkrgss-vk/src/tests` and the module tests, on llvmpipe:
  `frame_over_the_work_budget_is_a_limit_error` (checks also that no buffer grew, through the
  probe's counters), `failed_target_allocation_keeps_the_previous_frame`,
  `staging_buffer_shrinks_after_one_large_upload`, `out_of_date_twice_keeps_the_frame_unshown`
  (the present probe forces out-of-date), `lost_device_refuses_later_calls` and
  `wait_that_runs_out_is_a_lost_device` (a probe hook that makes the wait or a call return the
  result; no real device loss is caused), `descriptor_check_counts_the_whole_set` (a unit test
  of the check with made-up limits). In `vkrgss-host` session tests:
  `lost_device_ends_the_session_with_a_failure`.

## Out of scope

Re-creating the device after a loss; asynchronous submission; bounding the presentation
engine's own waits beyond the existing bounded acquire.

## As built

- **Work budget.** The largest tile-list entry count of the ten games of `cargo xtask xarch`,
  measured on 2026-10-26 through a temporary log in `prepare_frame` over a run of
  `xarch baseline` on llvmpipe, is 39252 (a frame of 640 by 480 with 41 ops). The budget
  `FRAME_WORK_BUDGET` in `binning.rs` is 2^22 = 4194304 entries, 107 times that, and keeps
  the index list at 16 MiB. A frame over it fails with `Limit::FrameWork`; `bin` is given the
  smaller of the budget and the entries the storage buffer range holds.
- **Bounded waits.** Every wait for a submission goes through `submit_with`, which waits on
  the timeline for `SUBMISSION_TIMEOUT`, 20 seconds. The heaviest frames at the budget,
  measured on llvmpipe on this machine with a temporary test, took 0.43 s (full-screen
  clears), 1.2 to 1.5 s (full-screen quads with tone, colour and additive blending, at 640 by
  480 and at 4096 by 4096) and up to 2.6 s (tilemap layers of the reference scenes), so the
  timeout is about eight times the slowest. A wait that runs out is `DeviceLost` with the call
  `vkWaitSemaphores`. The compositor records the loss at once (`lost`), and every failure
  path that would destroy what a submission used (`execute`, `read_image`,
  `discard_image`, `discard_buffer`, `submit_or_idle`, the undrawn swapchain image) leaves it
  alone instead and does not idle the device. A test injects the result of the wait through
  the probe after the real wait has returned, so no device is lost.
- **Lost device.** `VkError::from_result` turns `VK_ERROR_DEVICE_LOST` of any call into
  `DeviceLost`; `VkError::vulkan` and every place that built `VkError::Vulkan` from a result
  go through it. The public methods that use the device (`render` and through it `handle`,
  `forget_resources`, `read_frame`, `freeze` and `freeze_into_buffer` and through them
  `freeze_by`, `scale_frame`, `set_overlay`, `release_surface`, `attach_window`, `present`)
  run their work through `unless_lost`, which refuses at once when the device is lost and
  records a loss the work found. The methods that only set state (`set_mix`,
  `resize_surface` and the like) stay as they were.
- **Releasing the surface.** `release_surface` keeps the result of idling, destroys the
  swapchain, releases the surface and then returns the error. The presentation probe can
  replace the result of that idle, after it has idled, for the test.
- **Session.** The session cannot see `VkError`: it is generic over `Display`, whose error is
  only `Display`. So `Display` got a provided method `is_device_lost(&Self::Error) -> bool`,
  `false` by default, which `HeadlessDisplay` and `VulkanDisplay` answer with
  `VkError::is_device_lost` and the `RecordingDisplay` of the tests with a loss it is told to
  fake from a frame on (`lose_device_at`). These are a few lines each in `display.rs`,
  `headless.rs`, `windowed.rs` and `testing.rs` of `vkrgss-host`, outside the files the
  scope named, which the requirement plainly needs. In `Session::frame` an error that is a
  lost device is reported once as "the display's device was lost at frame N" and makes the
  session failing whatever `fail_on_rejected_frame` says, so it ends as `SessionEnd::Failed`
  with the tick of that frame; no later frame is rendered.
- **Previous frame.** `ensure_target` allocates the new image and points the descriptor at
  it first; only then are `has_frame` cleared, the old image destroyed and the presenter told
  to forget its binding. The rendering probe can fail the next allocation of a frame target
  (`fail_next_target`), which is what the test uses; the existing resource probe counts only
  frame buffers.
- **Staging buffer.** `frame_buffer` replaces the staging buffer by one of exactly the size
  the packet needs when it is larger than 64 MiB and more than four times larger than every
  upload of the last 60 packets (`STAGING_KEPT_BYTES`, `STAGING_SHRINK_FACTOR`,
  `STAGING_FRAMES` in `buffers.rs`). Each submission counts the bytes of its uploads, 0 for
  none, in a queue of 60. The check is made where the staging buffer is asked for, so only
  a packet with an upload gives the buffer back.
- **Unshown frame.** The attempts of `present` moved into `show`; `present` puts the frame
  back into `unshown_frame` whenever `show` returns anything but `Ok(true)`. The presentation
  probe can make acquires report an out-of-date swapchain (`report_out_of_date`).
- **Descriptor limits.** `DeviceCaps` holds the sampled-image limit (the smaller of the per-set
  and per-stage update-after-bind limits, against the bitmap slots) and
  `maxPerStageUpdateAfterBindResources` apart; the latter is held against
  `COMPOSE_SET_DESCRIPTORS`, 65542 (the 65536 bitmap slots, the frame target and five storage
  buffers), which a debug assertion ties to the bindings of the set. The devices in use stay
  suitable, with these values of `maxPerStageUpdateAfterBindResources`: llvmpipe (Mesa
  26.2.5, LLVM 22.1.8) 1015808 and RADV on the Raphael integrated GPU (Mesa 26.2.5) 8388606,
  both from `vulkaninfo` under the loader restriction on 2026-10-26; from vulkan.gpuinfo.org,
  Apple M4 with MoltenVK (report 52267, macOS 27.0.1) 1000000, Mali-G710 (report 51018,
  Pixel Tablet, the GPU of the Pixel 7a, driver 54.3.0; report 51090, an older driver,
  1572864) 8388608, and Adreno 750 (report 51830, Galaxy Z Fold6, the GPU of the Galaxy
  S24+; report 52484 the same) 50331648. Their sampled-image limits are at least 524288.
- **Drop.** A lost compositor leaks everything, the device and the instance with it: the
  device is now a `ManuallyDrop<Gpu>` that `drop` drops last only when the device is not lost.

## Follow-ups

- A headless run that ends on a lost device returns `HeadlessError::FrameRejected`, whose
  text, printed by the program after the session's own report, says "frame N was rejected by
  the compositor". The session reports the loss itself once; whether the runner should have
  an error of its own for a lost device is left open, as it would touch `headless.rs` and the
  program beyond the lines this change needs.
- A lost device found by a freeze (`Display::hold`) is reported as "cannot freeze the
  display" and the session ends at the next frame, which reports the loss; only the result
  of a frame is matched, as the design says.
