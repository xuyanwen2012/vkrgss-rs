# Design

## Context

- The session steps a transition's progress by 1/N with each frame and shows the k-th frame at
  k/N, ending the transition at the N-th.
- The measurement in `proposal.md` was made with a probe script in the original runtime under
  Wine: freeze a white screen, draw black, call `Graphics.transition(N)`, capture the screen
  when it returns and again after one `Graphics.update`.

## Decisions

- Show the k-th frame at progress (k-1)/N and keep the transition state until the frame after
  the N-th has been shown. The same progress value feeds the masked mix.
- Only the session changes; the presenter's mix is untouched.
