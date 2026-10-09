# Proposal

## Why

Measured on the original runtime: when `Graphics.transition(N)` returns, the screen does not
yet show the new frame alone but the mix at step N-1 of N; the new frame alone appears with the
next `Graphics.update`. For a white screen fading to black the centre pixel is 32 after
`transition(8)`, 64 after `transition(4)`, 13 after `transition(20)` and still 255 after
`transition(1)`, and 0 after the following update. vkxp reaches the new frame one step early,
which the first oracle comparison of the test game shows as a brighter frame during start-up.

## What Changes

- The k-th frame after a transition message of duration N is shown at step k-1 of N, for
  cross-fades and masked transitions alike; frames are shown normally from the frame after
  the N-th.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-session`: transition steps.

## Impact

- Crate `vkxp-host` (the transition code of the session and its tests).
