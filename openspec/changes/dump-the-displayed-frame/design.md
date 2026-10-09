# Design

## Context

- The session drives a display trait. The windowed display presents through `vkxp-vk`'s
  presenter, which holds a frame and mixes; the headless display writes dumps from the
  composited frame and holds nothing, so freeze and transition do not reach dumps.
- `vkxp-vk` can render the presenter's output into an off-screen target of a chosen size and
  read it back.
- Measured on the original runtime with a probe script (white screen, then black drawn):
  `Graphics.transition(8)` without a freeze leaves white on screen when it returns and black
  appears with the next update; after `Graphics.freeze`, `Graphics.transition(0)` leaves white
  on screen until the next update; two freezes in a row keep the first frozen picture; an update
  while frozen changes nothing on screen; `Graphics.frame_count` advances by N over a
  transition of duration N.
- An earlier change made the k-th frame of a transition show step k-1 of N.

## Decisions

- The headless display keeps the held frame and the transition state like the windowed one and
  produces a dump by the off-screen presentation path at the frame's logical size (scale 1, no
  letterbox), so the same shader code decides both. When neither a freeze nor a transition is
  active the dump is the composited frame, bit for bit as before.
- Session: a transition while nothing is frozen first holds the frame on screen and then runs
  as a transition whose every step shows the held frame; duration 0 clears the freeze and
  changes nothing on the display.
- A second freeze while frozen keeps the first held frame (add a test; change the code only if
  it does otherwise).

## Risks / Trade-offs

- [Dumps of existing tests change] → Only where a test dumped inside a freeze or transition;
  those expectations are updated to the displayed image.
