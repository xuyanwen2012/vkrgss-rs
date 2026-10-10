# Design

## Context

- `src/compositor/present.rs`, `present`: `acquire_next_image` is called with `u64::MAX`.
  `present` returns `Ok(false)` when nothing was shown (a window without pixels) and the host
  ignores the flag; pacing is the session's timer when present timing is absent.
- `ensure_swapchain` calls `device_wait_idle` before it replaces a swapchain; a failed draw
  replaces the swapchain together with its semaphores (`mark_swapchain_stale`).
- `src/compositor.rs`: `temporary_buffer` allocates a buffer per submission; `execute`
  creates the staging buffer and the four storage buffers of a frame, writes the descriptors
  and destroys the buffers after the fence wait. A test hook makes one such allocation fail
  (`fail_temporary_buffer_after`), and tests pin what a frame does when it fails.
- Each submission is waited for before the next is recorded. This change keeps that, and it
  is what makes reuse safe without per-frame copies of the buffers.
- The tests run on llvmpipe with a headless surface, which hands out images at once.
- The device code after `modernize-vulkan-sync-and-rendering` enables features through
  `PhysicalDeviceVulkan13Features`; optional extensions are recorded in the device's
  capabilities next to the present-timing ones.

## Decisions

### Bounded acquire

- The acquire waits at most 100 ms. `TIMEOUT` and `NOT_READY` are not errors: the frame's
  compute work is already done, the call returns `Ok(false)`.
- After a timeout the presenter is "starved": following acquires use a timeout of zero until
  one succeeds, so a hidden window costs one wait and not one per frame. The first success
  ends the state.
- The acquire semaphore is unsignalled after a timeout and is used again as it is.
- Present-timing state that counts presents treats a skipped present like the existing
  "window without pixels" case.
- Test: a `#[cfg(test)]` hook that makes the next N acquires report a timeout, like the
  existing allocation hook. It checks the return value, that rendering and read-back go on,
  that the next present succeeds, and that validation stays clean.

### Buffer reuse

- One long-lived buffer per role (staging, ops, tile heads, tile indices, tilemap records),
  host-visible and persistently mapped, created on first use. A frame that needs more than
  the capacity replaces that buffer by one of at least twice the size; nothing shrinks.
  Contents start at offset 0.
- A storage buffer's descriptor is written when the buffer is created or replaced, with the
  whole buffer as its range. The shader reads only the counts it is given, as today.
- A replaced buffer is destroyed at once when no submission is in flight, which is always the
  case here.
- The allocation-failure hook applies to creating or growing one of these buffers; the tests
  that use it keep their meaning (a frame whose buffer cannot be had fails the way it does
  today, and the next frame works).
- The compositor's teardown releases the five buffers.

### Swapchain maintenance

- Optional. The device enables `VK_KHR_swapchain_maintenance1` when present, else the `EXT`
  one, with its feature; instance side `surface_maintenance1` as the extension requires.
  If `ash` 0.38 lacks the `KHR` name, use `EXT` only and say so under "Checked".
- With it: each present carries a present fence (`SwapchainPresentFenceInfo`); replacing a
  swapchain waits for the outstanding present fences of the old one instead of idling the
  device; the path that abandons an acquired image after a failed draw releases it with
  `ReleaseSwapchainImages` where that lets the swapchain be kept. Present modes and scaling
  from the extension are not used.
- Without it: today's path, unchanged.
- Both paths are tested: a `#[cfg(test)]` switch makes the presenter ignore the extension.
  Write under a "Checked" heading at the end of this file whether llvmpipe offered the
  extension in the run, and run the presentation tests once on the AMD GPU
  (`VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV`) and note the result there.

## Risks / Trade-offs

- [A compositor that is merely slow gets frames dropped] → 100 ms is four frame periods at
  40 fps; a dropped present loses no game state.
- [Reused buffers keep stale bytes past the counts] → The shader is bounded by the counts;
  the differential tests against the reference compositor cover it.
- [A present fence that is never signalled on a lost surface] → Bound the wait like the
  acquire, and fall back to idling the device when it expires.

## Follow-ups

- Telling the host that the window is not being shown, so that it could lower its work.
