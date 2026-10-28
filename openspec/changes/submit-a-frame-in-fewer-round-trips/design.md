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
- `docs/running.md`, "Frame times", still says that the `submit` row includes the wait for
  the uploads and the compose and that `present` waits for its pass alone. In a window that
  wait is now part of `present`; `FrameTiming` in `vkrgss-vk` says so. The page is outside
  the scope of this change.

## As built

### What waits where

- `render` waits for the compose of the `render` before it, as every method that uses the
  device does before it starts, except the four below; a `Bitmaps` message and
  `forget_resources` also wait for their own submission before they return. `Drop` waits for
  the compose with the bounded wait before it idles the device: a test replaces
  `vkDeviceWaitIdle` with a stub that does not idle, and the timeline semaphore was destroyed
  under a pending submission until `Drop` waited for it itself.
- The second command buffer is used not only by `present` but by every submission other
  than the compose: `present`, `freeze`, `freeze_into_buffer`, `read_frame` and
  `scale_frame` therefore submit after the compose without waiting for it first, and their
  one wait covers both. When such a method submits nothing (a window without pixels, no
  frame to read), it waits for the compose before it returns, so after it nothing is in
  flight.
- `has_frame` is set when the compose is submitted, not when it is seen complete: every
  reader waits for the compose first on the host or comes after it on the queue, and a
  compose that fails on the device is a lost device, after which nothing reads the target.
  This is what "readers wait for the pending value first" observes.
- The held frame, the buffer of RGSS3 and the stand-in for a mask are no longer cleared in a
  submission of their own when they are made: the presenter keeps the images that are to be
  cleared, and the next commands recorded into the second buffer clear them first. So a
  freeze into a new buffer clears and copies in one submission, the first present of a
  compositor makes its held frame and stand-in in the presentation pass's submission, and a
  freeze before the first frame clears the held frame with the next submission instead of
  one of its own. When the submission fails, the clears are recorded again with the next
  commands.
- A freeze in the middle of a cross-fade still draws the mix into an off-screen target in
  one submission and copies it in a second; the design names the clear and the copy only.
- The frame timing: `submit` is the submission alone, plus the wait for the compose where a
  call waits for it alone, as taking the timing does in a headless run; in a window the wait
  is part of `present`. `take_frame_timing` waits for the compose of a measured frame so
  that its device time is known.
- The per-method tests are in `tests/round_trips.rs`: one test for each method that uses the
  device, and one that calls each of the fifteen methods that only set or report state
  right after a render of its own. `attach_window` needs a window and is exercised through
  `attach_headless_surface`, which makes the same call.

### Same frames

`cargo xtask xarch baseline --at 9d37416d` and then `cargo xtask xarch check`, both with
`VK_LOADER_DRIVERS_SELECT='*lvp*'` over the ten games of the owner's list, found every game
identical: 81 frames and 17 save files, none different. The games of VX Ace run their guest
in a process, which needs `XDG_RUNTIME_DIR` to name a directory the sandbox of the run can
write; with the default one of the session both failed to start in a first baseline run,
before any frame, and both runs above set it to a directory below `TMPDIR`.

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

After, at the commit of task group 3 (`cf9e1b89`), with the same commands:

| Device | Run | `submit` mean | `submit` largest | `compose (device)` mean | `compose (device)` largest | Wall time (s) |
|---|---|---|---|---|---|---|
| llvmpipe | 1 | 8.367 | 13.140 | 8.108 | 13.074 | 32.1 |
| llvmpipe | 2 | 8.189 | 15.644 | 8.018 | 15.212 | 31.4 |
| llvmpipe | 3 | 8.224 | 13.736 | 8.027 | 13.609 | 31.6 |
| RADV | 1 | 0.683 | 3.038 | 0.633 | 1.850 | 3.2 |
| RADV | 2 | 0.686 | 2.912 | 0.634 | 1.906 | 3.2 |
| RADV | 3 | 0.679 | 2.795 | 0.632 | 1.638 | 3.2 |

No improvement, as expected of a headless run: it presents nothing, and its one wait per
frame, for the compose, is still there, now in `take_frame_timing` (counted in `submit`)
instead of in `render`. On RADV the numbers are those of before within 0.01 ms. On
llvmpipe the `submit` means are 2 to 4 % above those of the two comparable runs before
(8.19 to 8.37 ms against 8.03 and 8.06 ms) and the wall times 0.5 to 1.3 s longer; the
`compose (device)` means rose by as much, so the device work itself took longer, and these
runs do not tell whether that is the change or the load of the machine. The frames are
identical (task 4.3). The change removes the wait of `render` from a shown frame in a
window, which this machine could not measure (see Follow-ups); there the tests count one
wait per rendered and presented frame instead of two.
