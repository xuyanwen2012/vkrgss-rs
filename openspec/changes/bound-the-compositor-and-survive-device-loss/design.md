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

## Follow-ups

None yet.
