## Context

- `FramePacket` (`crates/vkxp-proto/src/frame.rs`) carries index, logical size, frame rate,
  deltas and ops. `GuestToHost::Transition { duration, mask, vague }`
  (`message.rs`) starts a transition; the session (`crates/vkxp-host/src/session.rs`,
  `display.rs`) gives the presenter a weight and a threshold per frame (requirements
  "Freeze and transition" and "Transition step values" of `host-session`).
- The CPU reference (`crates/vkxp-compose-ref`) and the Vulkan compositor produce the frame
  of a packet and must agree byte for byte.
- Measured on the original runtime of VX (2026-10-12, both DLL versions of the corpus; 24
  tests, 331 frames, 85729 sampled pixels, none wrong):
  - Brightness `b`, for a channel `v` of the scene: `v` if `b` is 255, else `(v * b) >> 8`.
    A level of 200 at brightness 100 shows 78.
  - `Graphics.freeze` keeps the picture on the screen with the brightness it had. While the
    display is frozen, later changes of the brightness do not change the picture.
  - `Graphics.transition(n)` after a freeze: frame k of 1 to n-1 shows
    `(old * (256 - a) + new * a) >> 8` with `a = 255 * k / n`; frame n shows the new scene
    itself. The old picture is not shown again. The new scene is at brightness 255.
  - `transition(0)` after a freeze shows no frame; the new scene appears with the next
    update.
  - Without a freeze the original shows no frame at all for a transition. That is decided
    in the guest and is not part of this change.
  - With a mask the frames are counted the same way. The thresholds per step were recorded
    and not fitted yet.

## Goals / Non-Goals

**Goals:** the display side can show what RGSS2 asks for: a darkened frame, and a
transition stepped by the rule of RGSS2. XP is shown exactly as before.

**Non-Goals:** `Graphics.brightness`, `wait`, `fadeout`, `fadein`, `snap_to_bitmap` and the
choice of the rule in the guest (the next change binds them); the measured thresholds of a
masked transition of RGSS2.

## Decisions

- The brightness belongs to the frame: `FramePacket::brightness`, a `u8`, 255 for "as it
  is". Both compositors apply it after the last op to the three colour channels of every
  pixel of the frame; the alpha of the frame is untouched. A held frame therefore keeps the
  brightness it was composed with, which is what the original shows after a freeze, and
  nothing in the presenter changes for it.
- 255 is the identity and is not multiplied (`(v * 255) >> 8` would darken).
- `Scene` keeps the brightness (default 255), clamps a set value to 0 to 255 and writes it
  into every extracted packet.
- `GuestToHost::Transition` gains `rule: TransitionRule` with the values `Rgss1` and
  `Rgss2`. With `Rgss1` the session behaves as today. With `Rgss2` the k-th frame after the
  message is shown at step `k` of N: for `k < N` with the cross-fade weight
  `an = 255 * k / N`, and the N-th frame shows the newest frame alone and ends the
  transition, so the frame after it is shown normally as well.
- A masked transition of `Rgss2` uses the threshold formula of "Transition step values"
  with `s = k` for `k < N`, and the N-th frame shows the newest frame alone. The step
  numbering is measured; the threshold values of RGSS2 are not, and `design.md` of the
  change that fits them replaces this. The code marks the formula as not measured for
  RGSS2 in the way `frame-composition` asks ("Measured formulas are marked").
- Duration 0 and a transition without a freeze keep the behaviour of today under both
  rules; the guest of RGSS2 will not send the second.
- The protocol version goes up by one. Recordings keep replaying as the requirement
  "Raw keys in recordings" and the recording format ask.
- In the binding, `Graphics.transition` sends `TransitionRule::Rgss1` and nothing sets a
  brightness. Those are the only edits there.

## Risks / Trade-offs

- The Vulkan compositor gets one more step per pixel. It is skipped at brightness 255 by
  the same condition as in the reference, so frames of XP cost what they cost today.
