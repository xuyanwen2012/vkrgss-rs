# Proposal

## Why

A headless run's frame dumps show the newest composited frame, not what a window would show:
freezes and transitions are invisible in them. The oracle harness compares dumps with the
original's screen, so every frame inside or right after a transition differs for a reason that
is not a rendering difference. Two more measurements on the original also disagree with the
session: a transition without a preceding freeze leaves the screen unchanged for its duration,
and a transition of duration 0 after a freeze leaves the frozen picture on screen until the
next frame.

## What Changes

- Headless frame dumps are the displayed image: the held frame while frozen, the cross-fade or
  masked mix during a transition.
- A transition that arrives while nothing is frozen shows the picture that was on screen for
  its N frames instead of the new frames.
- A transition of duration 0 after a freeze ends the freeze without showing anything new; the
  next frame is shown normally.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-session`: what dumps show, and two transition cases.

## Impact

- Crate `vkxp-host` (headless display, transition code of the session). `vkxp-vk` only if the
  off-screen presentation path lacks something the headless display needs.
