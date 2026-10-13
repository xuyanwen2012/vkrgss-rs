## Context

- `vkxp-core` orders elements by (z, creation) ("Draw order" of `core-scene`), applies a
  viewport's flash by the rule of a sprite, sizes the tile of a plane in 32-bit floats
  ("Planes tile a stretched tile"), and repeats input after 16 updates, then every 4
  ("Repeat" of `core-input`). The host maps keys to buttons for XP. The session marks the
  mask threshold of RGSS2 as a placeholder (`step_mix` in
  `crates/vkxp-host/src/session.rs`).
- The measurement: `../../ref/tools/oracle/measurements/vx-v1-sprite-REPORT.md` sections
  3 to 7 (seen from a clone under `wt/`), its model `vx-v1-sprite-model.py`, the frames
  `vx-v1-sprite-frames/`, `vx-v1-mask-*` and `vx-v1-input-*`. The report is the authority.

### Measured rules (RGSS2, both DLL versions identical)

1. Order: ascending (z, y, creation). y is the integer `y` of a sprite and 0 for a plane,
   a viewport, a window and the planes of a tilemap. Creation is the order of `new` and is
   never renewed (not by assigning z, `visible` or `viewport=`). Inside a viewport the same
   key orders its elements; a viewport takes its place with (its z, 0, its creation). A
   sprite at `y = 20, oy = 20` is in front of one at `y = 0`; a negative y sorts behind 0.
2. Viewport: tone, then the viewport's colour, then the flash colour with its current
   alpha, each by `s + (((c - s) * (a + 1)) >> 8)`. `flash(nil, d)`: for the duration the
   elements of the viewport are not drawn and its tone and colour are not applied.
3. Plane: `tw = floor(w * zoom_x)`, `th = floor(h * zoom_y)` in doubles (90 at 0.7 gives
   62); the rest as RGSS1.
4. Input: the model of RGSS1 (one repeating button, the highest number among buttons
   triggered in one update, the direction rules) with a delay of 24 and an interval of 6:
   a button held alone repeats on updates 1, 24, 30, 36. Keys: C is Enter, Space, Z; B is
   Esc, Insert, X, numpad 0; A is Shift; X, Y, Z are A, S, D; L is Q and Page Up; R is W
   and Page Down; the arrows and numpad 2, 4, 6, 8; Ctrl; Alt; F5 to F9. Shift presses A
   and SHIFT together.
5. Masked transition: the formulas of RGSS1 unchanged with step k of 1 to N
   (`T = 256 * k / N - 1`; `vague < 0` shows the new scene, `vague == 0` is a threshold,
   else `min(255, (L - T) * 255 / vague)`; the luminance and the tiling of the mask as
   RGSS1). The default vagueness is 40. (36 transitions, 1 109 760 pixels.)
6. `blend_type=` of a sprite and a plane raises `ArgumentError: bad value for blend_type`
   for a value outside 0 to 2; a Float is truncated first.

## Goals / Non-Goals

**Goals:** the six rules in a game of RGSS2. XP unchanged.

**Non-Goals:** the sprite's own drawing; gamepads and the F1 key dialog; whether XP shares
rules 2 and 6 (XP was not measured for them; a later measurement decides).

## Decisions

- The scene knows the rules of its runtime through one value given when it is created
  (the binding already creates it by runtime); order, viewport flash and plane size branch
  on it in `vkxp-core`. No new op is needed: the order is the order of the ops, the
  viewport flash is the existing rectangle effect applied once more, the plane's tile size
  is a number in its op.
- `Input` of the core takes its delay and interval from the same value.
- The host gets the key map of the run's runtime from the program; the map of XP stays the
  default. Shift sets both buttons, as measured.
- The mark "not measured for RGSS2" at `step_mix` is replaced by the provenance of the
  measurement; the formula does not change.
- The default vagueness is 40 in RGSS2.
- Tests: committed cases of the report's `order`, `viewport`, `flash`, `plane`, `size2`
  topics as scenes built by formula with expected pictures or digests from the model; the
  input sequences as numbers.
