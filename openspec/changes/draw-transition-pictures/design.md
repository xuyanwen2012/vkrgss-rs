# Design

## Context

- `GuestToHost::Transition { duration, mask, vague }` already carries the mask's `BitmapId`;
  the guest announces that bitmap in the frame packets that follow.
- The presenter mixes a held and a newest frame with one weight in the fragment pass that also
  scales to the swapchain; the session steps the weight and reports masks as unimplemented.
- The compositor owns the uploaded bitmaps as sampled images.
- mkxp's transition shader uses the formula of the spec; the original's own arithmetic is not
  yet measured.

## Decisions

- The present pass gets one more sampled image (the mask, or a 1 by 1 dummy) and push
  constants for progress, vagueness and a flag. The mask view comes from the compositor's
  bitmap table by id at the time each transition frame is presented, because the bitmap arrives
  with the first frame after the transition message.
- The off-screen target used by headless runs and tests goes through the same pass.
- The session keeps the mask id and vagueness with its transition state and drops them when
  the transition ends. A vagueness of 0 is treated as 1.
- PLACEHOLDER: formula and the use of the red channel follow mkxp until the oracle harness has
  compared a masked transition.

## Risks / Trade-offs

- [The guest disposes the mask before the transition's last frame] → The guest keeps it until
  `Graphics.transition` returns; an absent bitmap falls back to the cross-fade.

## Follow-ups

- The session's masked-transition scenario is tested on the recording display, which shows
  that the mask, the vagueness and the progress reach the display; the pixels of the two halves
  are checked by the presenter's tests. A test of the whole path on pixels needs a headless
  display that holds a frame, which `HeadlessDisplay` does not do yet.
- The display trait gained `has_bitmap`, which the session asks after each frame of a masked
  transition to find a mask that never arrived. Is a report per transition the wanted rate, or
  should it be once per session as the "not implemented" note was?
- When the held frame has another size than the newest, the mask is stretched over the
  rectangle of the newest frame. The spec says only "stretched to the frame".
- At progress 0 the formula already shows the newest frame where the mask is fully black. The
  session never shows a masked mix at progress 0, so this is left to the oracle comparison.
