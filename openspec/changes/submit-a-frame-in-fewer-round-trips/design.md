# Design

## Context

See the proposal. Today one command buffer (`VkCompositor::commands`) is recorded, submitted
and waited for by every method, and a timeline semaphore (`submit_with`) counts the
submissions. `bound-the-compositor-and-survive-device-loss` replaces the `u64::MAX` wait
with a bounded wait that returns a limit or device-loss error; this change builds on it and
does not change that error handling.

Callers: the window session calls `render` then `present` per frame, `freeze_by` on
`Graphics.freeze`; the headless session calls `render` and `read_frame` / `scale_frame` for
captures; the tests call every method.

## Decisions

### One submission may be in flight: the compose of the last `render`

`render` records as today, submits with the next timeline value and returns without
waiting; it stores that value as the pending compose. Every method that records into the
compose command buffer, reads an image or buffer back to the host, replaces or destroys a
buffer or image the pending work uses, or reports whether a frame is held, first waits for
the pending value (the bounded wait). `Drop` waits for it before destroying anything.

`render` itself waits for the pending value of the previous `render` before it writes the
staging buffer or the per-frame buffers, so at most one compose is in flight and the
per-frame buffers stay single (the requirement "Per-frame buffers are reused" is unchanged).
Double-buffering them for deeper pipelining is a follow-up, not part of this change.

### `has_frame` follows completion

`has_frame` (the held-frame contract, `render.rs:27-28`) becomes true when the pending
compose is known to have completed. Readers of it (`read_frame`, `present`, a freeze) wait
for the pending value first, so what they observe is the same as today. A device error of
the compose is returned by the first method that waits for it; `render` documents that its
device execution errors may surface there. The tests that render a frame and check its
error assert on the error of the next call where the device fails, if any such test exists.

### `present` uses a second command buffer and waits once

The presentation pass is recorded into a command buffer of its own, so recording it does not
need the compose buffer to be idle. It is submitted after the compose submission on the same
queue; the compose's existing barrier that hands the frame target to the fragment stage, and
submission order on one queue, order the two (a pipeline barrier's first scope includes
earlier submissions on the queue). The pass waits for the acquire semaphore and signals the
present semaphore as today, then the host waits once for the present submission's timeline
value, which is greater than the compose's, so after that wait nothing is pending.

### A freeze submits once

`clear_held` and `copy_to_held` record into one command buffer when both run in a freeze;
one submission, one wait.

### Restated contract

The doc comment of `VkCompositor` becomes: "At most the compose submission of the last
`render` is in use by the device when a method returns. Every method that records, reads
back, replaces or destroys waits for it first, and dropping the compositor waits for it;
after `present`, a freeze or a read-back nothing is in use." The spec requirement says the
same.

### Measurement

Before and after, on RADV and on llvmpipe, a run with `--frame-stats` over the same game
and route (one of the `xarch` games, headless where frame stats are available, or windowed
on RADV) records the mean and the 99th percentile frame time; the numbers go into this
design under "As built". The change is kept only if frames are identical; a frame time that
does not improve is recorded, not hidden.

## Risks

- A method that forgets to wait for the pending compose reads or destroys something in use:
  covered by the validation layer in the tests (synchronization validation where the tests
  enable it) and by a test per public method that calls it right after `render`.
- Frame timing stages: "waiting for the submission" moves from `render` to the first waiting
  call; the timing of a frame records the wait where it happens.

## Follow-ups

- `--frame-stats` reports no percentile, so the 99th percentile frame time that the
  measurement above asks for is not recorded; the largest value stands in. Reporting
  percentiles is a change of `vkrgss-host`, outside this change.
- No display was available for the measurement, so no windowed run on RADV was made; a
  headless run presents nothing, and the waits this change removes from a shown frame do not
  occur in it.

## As built

### Frame times

Machine: AMD Ryzen 5 9600X, its integrated GPU with RADV, and llvmpipe (Mesa 26.2.4), no
display, so the runs are headless. A release build (`cargo build --release -p vkrgss`) runs
Visions & Voices (a copy of the game of the `xarch` list) along the input schedule of the
oracle's measurement V0 for 3700 frames: title, name input, the first maps with messages and
weather. The command, with `<D>` empty for llvmpipe and `--device RADV` for RADV:

    VK_LOADER_DRIVERS_SELECT='*lvp*'      # or '*radeon*' with VKRGSS_TEST_DEVICE=RADV
    target/release/vkrgss <copy of vx-visions-and-voices> --headless --no-audio \
      --frame-stats --runtime rgss2 --rtp RPGVX=<RTP of VX> \
      --builtin-scripts <rpg_builtin_rgss2.rb> --seed 1 --frames 3700 \
      --input @ref/tools/oracle/measurements/vx-v0-vv-input.txt <D>

A headless run presents nothing, so its `present` rows count no frame, and `--frame-stats`
reports the mean, the smallest and the largest value of a stage but no percentile; the
largest value stands in for the 99th percentile (see Follow-ups). The wall time is that of
the whole process. Times in milliseconds, three runs each.

Before, at `9d37416d`:

| Device | Run | `submit` mean | `submit` largest | `compose (device)` mean | `compose (device)` largest | Wall time (s) |
|---|---|---|---|---|---|---|
| llvmpipe | 1 | 9.299 | 2064.450 | 8.981 | 2064.122 | 35.6 |
| llvmpipe | 2 | 8.030 | 12.696 | 7.844 | 12.564 | 30.8 |
| llvmpipe | 3 | 8.057 | 12.602 | 7.873 | 12.551 | 30.9 |
| RADV | 1 | 0.682 | 3.025 | 0.632 | 1.869 | 3.6 |
| RADV | 2 | 0.678 | 2.788 | 0.632 | 1.510 | 3.2 |
| RADV | 3 | 0.681 | 2.901 | 0.632 | 1.692 | 3.2 |

The first llvmpipe run has one frame of two seconds that the later runs do not have, which
raises its mean. `encode`, `bin` and `upload` have means below 0.006 ms in every run.
