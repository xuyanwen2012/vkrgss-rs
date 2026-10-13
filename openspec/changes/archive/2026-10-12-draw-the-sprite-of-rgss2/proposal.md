## Why

A sprite of RGSS2 is placed and sized in double precision where RGSS1 uses 32-bit floats,
draws its bush rows by other rules with a `bush_opacity`, and can wave. These were measured
on the original runtime of VX on 2026-10-12 (`vx-v1-sprite-REPORT.md` sections 1 and 2
under `ref/tools/oracle/measurements/`, outside the repository; 325 frames, 73 548 800
pixels and a held-out random set with none wrong, both DLL versions). Characters on a map
and battlers are sprites; a position off by one pixel fails the standard.

## What Changes

- In RGSS2 an unrotated sprite is placed by truncation and sized in doubles.
- In RGSS2 the bush rows follow `bush_opacity` and are drawn without tone, colour, flash
  colour and, when unrotated, mirror; a bush depth beyond the height follows the measured
  walk.
- In RGSS2 `wave_amp`, `wave_length`, `wave_speed`, `wave_phase` exist and an unrotated
  sprite waves as measured.
- In RGSS2 an unstretched mirrored sprite whose source rectangle leaves the bitmap is cut
  first and mirrored then; `bitmap=` with the same object keeps `src_rect`.
- XP behaves as before in every case.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: the sprite of RGSS2.
- `rgss-scene-classes`: the new properties; the pending list.

## Impact

- `crates/vkxp-core` (the sprite's extraction: `extract.rs`, `turned.rs`, `element.rs`),
  `crates/vkxp-binding` (`sprite.rs`, the method test), tests in `crates/vkxp-compose-ref`
  and `crates/vkxp-vk`, `docs/compatibility.md`.
- The protocol and the compositors only if the measured rules cannot be said with the ops
  that exist.
