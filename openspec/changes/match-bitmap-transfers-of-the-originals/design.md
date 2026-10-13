## Context

- The measurement: `../../ref/tools/oracle/measurements/vx-v1-bitmap-REPORT.md` (seen from
  a clone under `wt/`), sections 5a, 5b, 5d and 10, with the model `vx_v1_bitmap_xp.py`
  (`transfer2`, `block_axis` are the corrected transfer) and the sample files
  `vx-v1-bitmap-{xfer,clip,extra2}-samples*.bin.gz`. The report is the authority. Every
  rule below gave the same bytes on `RGSS103J.dll`, `RGSS202E.dll` 2.0.2.1 and 2.0.0.1.

### Measured rules

1. `blt` and `stretch_blt` with an opacity above 255 leave the destination unchanged, as
   0 and negative values do (256, 300, 65536, 1073741823). A Float is truncated first
   (255.9 is 255).
2. A transfer of equal sizes (`blt`, and `stretch_blt` whose two rectangles have equal
   absolute sizes) with a negative source width or height, per axis with `n = |sw|`,
   destination span `[dl, dr) = [dx, dx + n)`, low end `sl = sx + sw`, high end `sh = sx`:
   if `sl < 0`: `dl += -sl; sl = 0`; if `dl < 0`: `sh += dl; dl = 0`; if `sh > ssize`:
   `dr += ssize - sh; sh = ssize`; if `dr > dsize`: `sl += dr - dsize; dr = dsize`;
   destination `dl + i` takes source `sh - 1 - i`. The unmirrored case keeps the four clips
   vkxp has. Scaled transfers keep their rule. (3648 `blt` and 1368 `stretch_blt` cases.)
3. A disposed source: no error. Every pixel p of the destination rectangle (for `blt` at
   x, y with the absolute size of the given source rectangle; clipped to the bitmap)
   becomes the blend of p onto itself at the given opacity:
   `a = (pa * opacity + 127) / 255`, new alpha `(255 * pa + 255 * a - pa * a) / 255`, the
   colour unchanged. The position of the source rectangle is not used.
4. A 32-bit BMP whose alpha bytes are all 0 decodes with alpha 0 everywhere.
5. `Color.new` and the writers of `Color` limit red, green and blue to -255 to 255 and
   alpha to 0 to 255.

## Goals / Non-Goals

**Goals:** the five rules, for both runtimes.

**Non-Goals:** the pixels `draw_text` touches without ink (finding 5 of the report's
section 10; it needs the work area the original keeps between calls and gets a change of
its own); `Tone`.

## Decisions

- No branch by runtime: the originals agree.
- The opacity argument is not limited in the binding any more; the core takes the integer
  and draws nothing outside 1 to 255.
- A disposed source is a case of the transfer in `vkxp-core`, not an error: the requirement
  "Disposal" keeps its error for every other use of a disposed bitmap. A disposed
  destination still raises.
- The committed samples are taken from the report's sample files (inputs by formula,
  outputs of the originals) and cover every branch of rule 2.
- The provenance list of `vkxp-pixel` gets the entries.
