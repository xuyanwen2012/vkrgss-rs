## Why

The display side can darken a frame and step a transition by the rule of RGSS2 since
`show-brightness-and-rgss2-transition-steps`, and a game of VX starts at its size and rate.
The functions of `Graphics` that RGSS2 adds are still missing, and `Graphics.transition`
still follows XP. The scripts of every VX game call them from the title screen on
(`Graphics.wait`, `fadeout`, `fadein`, `snap_to_bitmap` for the menu background).

## What Changes

- In RGSS2: `Graphics.brightness`, `brightness=`, `wait`, `fadeout`, `fadein` and
  `snap_to_bitmap`, each as measured on the original runtime of VX.
- In RGSS2 `Graphics.transition` defaults to 10 frames, shows nothing and returns at once
  when nothing is frozen, sets the brightness to 255 after a freeze, and asks the host for
  the steps of RGSS2.
- XP behaves as before in every case; none of the new functions exists there.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-modules`: the functions of `Graphics` by runtime.

## Impact

- `crates/vkxp-binding` (`graphics.rs`, the testing host and tests).
- `crates/vkxp-core`: only what a snapshot needs that extraction does not offer today (the
  ops and pixels of the scene without touching the frame count or the pending deltas).
- `docs/compatibility.md` where it lists what of VX works.
- Not touched: the protocol, the host, the compositors.
