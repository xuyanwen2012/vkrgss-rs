## Why

RGSS2 has a brightness of the whole screen (`Graphics.brightness`, `fadeout`, `fadein`) and
steps a transition differently from RGSS1: the frames of a transition of N show steps 1 to N
and the last one is the new scene itself, where XP shows steps 0 to N-1. Both were measured
on the original runtime of VX for milestone V0 (`vx-v0-REPORT.md` under
`ref/tools/oracle/measurements/`, outside the repository). The title screen of a VX game
fades and cross-fades before anything else happens, so the side that shows frames has to
know both before the functions of `Graphics` can be bound.

## What Changes

- A frame packet carries a brightness from 0 to 255. Both compositors darken the composed
  frame by it as their last step, with the arithmetic of the original.
- The scene keeps the brightness and puts it into every packet.
- The transition message says by which rule its frames are stepped: that of RGSS1 as today,
  or that of RGSS2.
- The protocol version goes up by one.
- Nothing in the guest sets a brightness or asks for the rule of RGSS2 yet: a game of XP and
  a game of VX are shown as before this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: the brightness of a frame packet; the rule of a transition message.
- `core-scene`: the brightness of the scene.
- `frame-composition`: the brightness as the last step of a frame.
- `host-session`: the steps of a transition of RGSS2.

## Impact

- `crates/vkxp-proto`, `crates/vkxp-core` (scene and extraction), `crates/vkxp-compose-ref`,
  `crates/vkxp-vk` (shader and records), `crates/vkxp-host`.
- `crates/vkxp-binding`: only the lines that build or match the changed message and packet,
  so that it compiles; the rule it sends stays that of RGSS1.
- `docs/architecture.md` where it describes the packet and the transition.
- Not touched: `crates/vkxp` and the rest of the binding: the change
  `start-vx-games-at-their-size-and-rate` runs in parallel there.
