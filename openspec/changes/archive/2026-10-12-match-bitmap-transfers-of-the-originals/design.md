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

## Implementation notes

- `vkxp-pixel` gets `transfer_opacity` (rule 1) and `block_span` with its result `BlockSpan`
  (rule 2), both listed in `BITMAP_PROVENANCE`. `block_span` does the unmirrored clips as
  well, so a transfer of equal sizes goes through it whole and `transfer_axis` of `vkxp-core`
  keeps the scaled transfers only.
- `Bitmaps::blt` and `Bitmaps::stretch_blt` take the opacity as `impl Into<i32>` and the
  source as `impl Into<Option<BitmapId>>`. The callers outside the scope of this change
  (`window.rs`, `plane.rs`, the tests of `delta`) pass a byte and an id as before and compile
  unchanged; the binding passes the integer of the script and `None` for a `Bitmap` object
  that was disposed, which has no id any more.
- A disposed source is the transfer of the destination rectangle onto itself, in place, as
  the original does it (report, section 5a). A destination rectangle with a width or height
  of 0 or less draws nothing then as well; a negative one was not measured.
- The committed samples are `crates/vkxp-pixel/tests/data/bitmap-transfer-samples.txt`: all
  78 records of the opacity argument, 300 of the mirrored transfers of equal sizes along
  one axis (216 `blt`, 84 `stretch_blt`; they take every combination of the four clips that
  the probe reached, 13), the 8 `blt` cases mirrored or cut along both axes, and the 9 cases
  of a disposed source with their destinations before and after.

### Existing tests changed to the measured behaviour

- `crates/vkxp-core/src/bitmap/tests.rs`, `disposed_source_fails_and_leaves_the_destination`:
  asserted the disposed error. Now `disposed_source_blends_the_destination_onto_itself`
  (rule 3).
- `crates/vkxp-core/src/bitmap/tests.rs`, `naive_blt`, the model behind
  `blt_matches_the_naive_model` and `transfer_within_one_bitmap_reads_the_live_pixels`:
  mirrored first and clipped afterwards. Its mirrored axis now has the four clips (rule 2).
  The two tests themselves are unchanged.
- `crates/vkxp-binding/tests/cases/mod.rs`, the case "disposed source" of `BITMAP_DRAWING`:
  expected `RGSSError`. Now the measured pixels (rule 3).
- `crates/vkxp-pixel/src/bitmap.rs`, `both_functions_are_listed_and_neither_is_a_placeholder`:
  asserts the names of `BITMAP_PROVENANCE`, which got two entries. Now
  `every_function_is_listed_and_none_is_a_placeholder`. It asserts no behaviour of the five
  rules; it changed because the list did.
- `crates/vkxp-image/src/bmp.rs`, `thirty_two_bits_without_alpha`: expected alpha 255. Now
  alpha 0 (rule 4).

No existing test asserted that an opacity above 255 is limited (rule 1).

Rule 5 needed no change of code: `Color` of `vkxp-core` has limited red, green and blue from
below since the commit "Clamp colour components from below as well", and the binding builds
every `Color` through it. The existing tests of both ends pass unchanged; the scenario
"Lower bound, measured on the originals" got a test in `vkxp-core` and a case in the
binding.
