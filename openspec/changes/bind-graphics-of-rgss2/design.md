## Context

- `crates/vkxp-binding/src/graphics.rs` defines `Graphics`; what is defined by runtime is
  decided there (`Graphics.width` and `height` exist in RGSS2 only).
- `Scene` keeps a brightness and writes it into every packet. `GuestToHost::Transition`
  carries `rule: TransitionRule`; the binding sends `Rgss1`.
- The binding knows whether the display is frozen (the flag it sets in `Graphics.freeze`).
- `vkxp-compose-ref` renders a packet on the CPU and applies the brightness of the packet.
- Measured on the original runtime of VX (2026-10-12, `vx-v0-REPORT.md` section 6 under
  `ref/tools/oracle/measurements/`, outside the repository; both DLL versions):

  | call | frames shown | note |
  |---|---|---|
  | `Graphics.update` | 1 | |
  | `Graphics.wait(n)` | n | none for n below 1; each frame as an update shows it |
  | `Graphics.fadeout(n)`, `fadein(n)` | n | none for n = 0, and the brightness still ends at 0 or 255 |
  | `Graphics.freeze` | 0 | |
  | `Graphics.transition(n)` after a freeze | n | default n is 10; none for n = 0 |
  | `Graphics.transition(n)` with nothing frozen | 0 | returns at once and shows nothing |
  | `Graphics.snap_to_bitmap` | 0 | |

  - `brightness=` clamps to 0 to 255.
  - `fadeout(n)` from brightness b0: frame k of 1 to n is the scene at `b0 - b0 * k / n`;
    the brightness stays 0. `fadein(n)` from b0: frame k at `b0 + (255 - b0) * k / n`; it
    ends at 255. Integer division. The scene is drawn anew in every frame.
  - While the display is frozen, `update`, `wait`, `fadeout` and `fadein` count their
    frames, the fades change the value, and the picture stays the frozen one.
  - `transition` after a freeze sets the brightness to 255, also with n = 0. Without a
    freeze it leaves the brightness.
  - `wait`, `fadeout`, `fadein` and `transition` do not call a `Graphics.update` that a
    script redefined, and no `Input.update` happens inside them.
  - `snap_to_bitmap` returns a bitmap of the size of the frame that holds the scene as the
    objects are at that moment: a change not yet shown is in it, and so is a change made
    after `Graphics.freeze`. The brightness is applied (level 200 at brightness 100 gives
    78). Its alpha is 255 everywhere. The frame count does not change.
- Decided by the owner on 2026-10-12: the snapshot is composed in the guest with the CPU
  reference compositor; no protocol message brings pixels back from the host.

## Goals / Non-Goals

**Goals:** the functions above in a game of RGSS2, with the frames, values and pictures of
the original. XP unchanged.

**Non-Goals:** `Graphics.resize_screen` (its limits are being measured); the thresholds and
the default vagueness of a masked transition in RGSS2 (being measured; the arguments are
read as today); the repeat timing of `Input`.

## Decisions

- The new functions are defined only when the run's runtime is RGSS2.
- A frame of `wait` and of a fade is the frame of `Graphics.update`: one extraction, one
  frame message, one tick awaited, host requests (reset, quit) raised as there. The Ruby
  method `Graphics.update` is not called.
- A fade sets the brightness of the scene before each of its frames by the formulas above
  and leaves the end value when it has no frame. A count below 0 is taken as 0; that case
  was not measured and is marked so in the code.
- `Graphics.transition` in RGSS2:
  - nothing frozen: no message, no frame, the brightness as it is, `nil` at once. The
    picture file is not loaded either (not measured; the cheapest reading);
  - frozen: brightness 255, then the transition message with `TransitionRule::Rgss2` and
    the frames as today. Default duration 10.
- `snap_to_bitmap` renders the scene with the CPU reference into a new bitmap of the
  guest's arena. It must not disturb what the host is owed: the frame count, the frame
  index and the pending bitmap and table deltas stay as they were. If extraction cannot be
  asked for a packet without those effects, `vkxp-core` gains the smallest function that
  can (the ops of the scene now, and the full contents of the bitmaps and tables they
  refer to); the reference then renders that packet in a compositor of its own that lives
  only for the call. The alpha of the result is set to 255.
- A snapshot costs a full CPU frame. The original takes about as long for the same call
  when read back; no cache is built.

## Risks / Trade-offs

- A scene with a tilemap or many large bitmaps makes the one-off packet large. Games call
  the function once per menu or battle start; measured cost is a follow-up, not a reason
  for machinery now.
