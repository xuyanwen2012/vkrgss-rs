# Proposal

## Why

Nothing in vkxp measures where a frame's time goes. `--present-stats` reports the intervals
between frames shown, which says whether pacing holds, not what the time is spent on. Every
statement about performance is therefore a guess, and work on speed (fast paths for bitmap
transfers, fewer submissions) cannot be judged before or after. The Vulkan objects are also
anonymous, so a capture in RenderDoc or a validation message names handles only.

## What Changes

- The Vulkan backend measures each frame on request: processor time of its stages (encoding
  the ops, binning, uploads, waiting for the submission, presenting) and, where the device has
  timestamps, device time of the compose dispatch and of the presentation pass.
- The host measures how long it waits for the guest between frames, which is the time the
  game's scripts and frame extraction take.
- A new option `--frame-stats` reports a summary of these per stage, periodically in a window
  and at the end of a headless run.
- When the debug-utils extension is available, the backend names its objects and labels the
  upload, compose and presentation parts of its command buffers.

Out of scope: any change that makes a frame faster; a split of the guest's time into scripts,
bitmap operations and extraction (use a profiler with the `profiling` build profile; the
README says how); a benchmark suite; a protocol change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-compositor`: frame timings on request; named objects.
- `host-session`: frame time statistics.
- `game-launch`: the option.

## Impact

- `crates/vkxp-vk`, `crates/vkxp-host`, `crates/vkxp` (option and usage text), `README.md`.
