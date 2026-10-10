# Design

## Context

- `VkCompositor::render` (`crates/vkxp-vk/src/compositor.rs`) encodes and bins a frame, then
  `execute` uploads bitmaps, submits and waits for the fence; `present`
  (`compositor/present.rs`) records and submits the presentation pass.
- `--present-stats`: `crates/vkxp-host/src/windowed.rs` counts frames and every
  `STATS_INTERVALS` frames writes a line built from an `IntervalSummary` (count, mean,
  smallest, largest). After `log-through-tracing` such lines are events.
- `debug_utils` is enabled on the instance only together with the validation layer
  (`device.rs`).
- The session (`vkxp-host/src/session.rs`) receives frame packets from the guest and answers
  them; the headless run drives the same session without a window.

## Decisions

### Backend

- `VkCompositor::set_frame_timing(bool)`, off by default. When off, no clock is read and no
  query is recorded. When on, each `render` and `present` fills a plain `FrameTiming` value
  that `take_frame_timing()` hands out: processor durations for encode, bin, upload (staging
  copy and recording), submit-and-wait, and present (record, submit, wait), measured with
  `std::time::Instant`; and optional device durations for the compose dispatch and the
  presentation pass.
- Device durations use one timestamp query pool, written at the start and end of the compose
  dispatch and of the presentation pass, read after the fence wait that already happens. They
  are present only when the queue family's `timestampValidBits` is not zero; otherwise the
  fields are `None`. `timestampPeriod` converts ticks to nanoseconds.
- Names and labels: the instance enables `VK_EXT_debug_utils` whenever the loader lists it,
  not only with validation. With it, the backend names its long-lived objects (the frame
  target, held frame, mask, the per-frame buffers, pipelines, the swapchain images, and
  bitmap images by slot number) and wraps uploads, compose and present in command buffer
  labels. Without it nothing is named. Naming is one helper; a failure to name is ignored.

### Host

- The session measures, when asked, the time from answering a frame to receiving the next
  frame packet ("guest"), with the host's monotonic clock. It does not feed back into pacing
  or into anything the guest can observe.
- `--frame-stats` turns on the backend's timing and the session's. In a window a summary is
  reported every `STATS_INTERVALS` frames like the present statistics; a headless run reports
  one summary at its end. The summary has one row per stage with count, mean, smallest and
  largest in milliseconds, reusing `IntervalSummary` or its arithmetic. It is requested by an
  option, so it is shown without `--verbose`.
- A configuration-file key is not added.

### Documentation

- `README.md`: the option, how to read the rows, and a short recipe for profiling the guest
  with `perf` and the `profiling` profile.

## Risks / Trade-offs

- [Timing changes the frame] → It only reads clocks and writes queries; the differential
  tests run once with timing on and compare the same frames.
- [llvmpipe's timestamps mean little] → The tests check presence and ordering, not values.

## Follow-ups

- Splitting the guest's time (scripts, bitmap operations, extraction) needs either a protocol
  field or guest-side events; decide after the first numbers.
