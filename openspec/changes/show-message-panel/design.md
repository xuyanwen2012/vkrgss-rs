# Design

## Context

- `GuestToHost::Modal { text }` and `HostToGuest::ModalDismissed` exist. The guest sends the
  message and pumps host messages until the dismissal; the host answers at once and prints the
  text.
- The guest owns the scene, bitmaps and the text rasteriser; the host has no text rendering and
  must not depend on guest crates. The host-session spec says drawing a panel is the guest's
  business.
- The session delivers input snapshots and knows the key map.

## Decisions

- Guest: on a modal call, create a bitmap with the rendered text (default font, white on a
  box of black at about 80% opacity with a light border, at most 560 by 400, centred), show it
  through a scene sprite with the highest draw order and no viewport, send `Modal`, then loop:
  extract and send a frame, pump with waiting for the tick, until the dismissal has arrived;
  dispose sprite and bitmap. `Graphics.frame_count` is not advanced by these frames. A reset
  or quit request during the wait is handled as in `Graphics.update`.
- Host: the session keeps a pending-modal flag. Headless: answer at once (unchanged). Windowed:
  answer on the next newly pressed key mapped to C or B, or mouse click, and suppress that key
  until it is released.
- `p` log-only: a flag of the run configuration, `--log-p` and the configuration key `log_p`.
- Tests on the guest side use the scripted host of the binding's test support, told to dismiss
  after a number of frames.

## Risks / Trade-offs

- [A frozen screen (`Graphics.freeze`) hides the panel] → The host shows held frames while
  frozen; a message during a freeze is then only on the error stream until the transition.
  Accepted for now and noted as a follow-up.

## Follow-ups

- A message during a freeze is not visible (see the risk above), yet a windowed session still
  waits for its dismissal. Should the panel be shown over the held frame?
- Only the keyboard and the mouse dismiss a message. Should a gamepad button mapped to C or B
  dismiss it too, and be suppressed until released like a key?
- A key mapped to C or B that is already down when the message arrives does not dismiss it;
  it has to be released and pressed again. Its hold stays visible to the game.
- When a reset or quit request ends the guest's wait, the windowed session still holds the
  message as pending: the next confirm or cancel press is swallowed and its dismissal reaches
  a guest that no longer waits for it. Should the guest tell the host that it gave up, which
  would need a protocol message?
- The guest always sends at least one frame with the panel, also when the host dismisses at
  once, so a headless run shows every message for one frame. The frames of the wait advance
  the emulated tick count of `Win32API` but not `Graphics.frame_count`.
- With an input recording or replay in a windowed session a message still waits for the
  player; the dismissing press is not part of the recording.
