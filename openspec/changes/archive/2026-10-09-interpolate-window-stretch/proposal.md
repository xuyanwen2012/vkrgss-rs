# Proposal

## Why

After the window rules were implemented, the test game's menu still differs from the original:
small windows are identical, but in large windows almost every background pixel is off by 1 or
2. The cause was measured: when the original stretches the background, or the centre of the
cursor, with the reduced numerator (127 for the background, 27 for the cursor centre) it
interpolates bilinearly; it does not pick the nearest texel. The first measurement missed this
because its synthetic skins were ramps on which a truncating bilinear filter returns the
nearest texel. With the corrected rule a model reproduces the real menu and nine further
probe frames with no differing pixel.

Two assignments were also left unconnected by the previous change: the binding sets a
window's `active` and `pause` as plain fields, so `active = false; active = true` and
`pause = false; pause = true` without an update in between do not reset the counters as
measured.

## What Changes

- The window background and the cursor centre are stretched with the measured bilinear rule
  when the reduced numerator applies; every other window stretch stays nearest.
- The binding assigns `active` and `pause` through the functions that keep the counters as
  measured.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: bilinear window stretch.

## Impact

- Crates `vkxp-core` (window drawing) and `vkxp-binding` (two assignments).
