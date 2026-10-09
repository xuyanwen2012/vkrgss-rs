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
