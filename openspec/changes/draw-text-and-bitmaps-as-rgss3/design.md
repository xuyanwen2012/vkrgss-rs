## Context

- Text is laid out and blended in `crates/vkxp-core/src/text.rs` over a `TextRasterizer`
  that the embedder supplies; `crates/vkxp-text` is that rasteriser's engine (lookup,
  metrics, glyph coverage). Neither knows an interpreter. The binding
  (`crates/vkxp-binding/src/bitmap.rs`, `font.rs`) chooses the rules of a runtime today by
  passing `Squeeze::Rgss1` or `Squeeze::Rgss2` and by building `FontDefaults::new` or
  `FontDefaults::rgss2`; for `Runtime::Rgss3` it passes the values of RGSS2.
- `Runtime` is a type of `vkxp-vfs`, which `vkxp-core` and `vkxp-text` do not depend on.
  In these two crates the runtime is therefore a value of their own (`Squeeze` today), and
  the binding maps `Runtime` to it.
- The interpreter of RGSS3 (change `vendor-ruby-1-9-2`) and the binding over it do not
  exist. No script of RGSS3 can run, so nothing of this change can be tested through Ruby.
- The measurement: `../../ref/tools/oracle/measurements/ace-v1-bitmap-REPORT.md` (seen from
  a clone under `wt/`), its check script `ace-v1-bitmap-draw-check.py`, the samples
  `ace-v1-bitmap-samples.txt` and the record files `ace-v1-bitmap-*-samples.bin.gz`. The
  report is the authority for every rule below; where this text is shorter, the report
  decides. Measured on DLL 3.0.1.1 and repeated on 3.0.0.1, under Wine 11.0.
- vkxp does not reproduce the glyph shapes of GDI. It holds layout, advances, placement and
  blending to the original; the quantities GDI answers in the original come from
  `vkxp-text` (decision 9).

### Measured rules (RGSS3; integers, `/` truncates toward zero, straight alpha)

`blt_blend(D, S, 255)` is the arithmetic of the block transfer, `vkxp_pixel::blend_normal`.

1. Font defaults: name `"VL Gothic"`, size 24, bold and italic false, shadow false, outline
   true, colour (255, 255, 255, 255), out colour (0, 0, 0, 128). (Report, section 4.)
2. Measuring. The width of a text is the sum of its advances with the left-bearing
   correction, its height the font's `tmHeight`: what `text_size` returns, the same values
   as RGSS2 for the same font, with outline or shadow on or off. `draw_text` places the
   text by `E` and `th`: `E` is that width with the last glyph counted by its right
   overhang, plus 2 with the outline; `th` is the height, plus 2 with the outline.
   (Section 6b.)
3. Coverage: GDI gives each glyph pixel a value `v` from 0 to 64 and the mask holds
   `trunc(v * 255 / 64)`: 1 gives 3, 32 gives 127, 49 gives 195, 56 gives 223, 63 gives
   251, 64 gives 255. RGSS2 holds `min(255, 4 v)`. A later glyph overwrites an earlier one
   where its own value is above 0 (3.0.1.1), or over its whole black box, zeros included
   (3.0.0.1). (Section 6c, section 12.)
4. Squeeze, one step, in doubles: `r = 1.0`; if `E > w`: `r = w / E - 0.15`, at least 0.6;
   the font is made again with `lfWidth = trunc(size * r * A / H)`, evaluated in this order,
   and `E` is measured again in it. `A` and `H` are the average character width and the
   height of the stock System font, 7 and 16 under Wine. There is no loop and no check that
   the squeezed text fits. `E <= 0` draws nothing. (Section 6d; 24 sweeps, 2330 widths.)
5. Placement: `top = y + max(0, (h - th) / 2)`; `left = x` for align 0 and any other value,
   `x + max(0, (w - E + (1 if outline else 0)) / 2)` for align 1, `max(x, x + w - E)` for
   align 2. (Section 6e.)
6. Work rectangle: the mask's pixels (u, v) with `0 <= u < min(w, 640)` and
   `0 <= v < min(h, 480)`, put at (left, top); the glyphs start at its column 0 and row 0.
   The whole of it is blended, also where it has no ink, right of the rectangle given when
   the text is centred or right aligned and below it when the text is centred vertically;
   only the bitmap cuts it. Nothing is kept between calls. `w <= 0`, `h <= 0` or an empty
   string draw nothing. (Section 6e; the lines of the edge probes in the samples.)
7. Passes. `ca = trunc(colour alpha)`; colour components truncated, modulo 256.
   `pass(dx, dy, rgb, alpha, lo)`: nothing when `alpha` is 0; otherwise for every (u, v) of
   the work rectangle with `lo <= u < w - lo` and `lo <= v < h - lo`:
   `D = dest[top + v + dy][left + u + dx]`, skipped outside the bitmap,
   `D = blt_blend(D, (rgb, (mask(u, v) * alpha + 127) / 255), 255)`.
   With the outline: `lo = 1`, `oa = out alpha * ca / 255`, then `pass(-1, -1)`,
   `pass(+1, -1)`, `pass(+1, +1)`, `pass(-1, +1)` in the out colour at `oa`, then with the
   shadow `pass(+1, +1)` in black at `ca`, then `pass(0, 0)` in the colour at `ca`.
   Without: `lo = 0`, the shadow pass if the shadow is on, the text pass. The mask is not
   moved to make room for the outline. (Section 6f; 1136 draws, 16358400 pixels.)
8. `Font.exist?(name)` is an exact, case-sensitive test against the family names of the
   loaded fonts. A name given as an Array stands for its first element for which the test
   holds, and for its first element when none does; an empty Array is no name. The chosen
   name then goes to the font mapper of GDI, which matches without regard to case and
   replaces a missing name. (Section 7.)
9. `Bitmap.new(w, h)` has no limit of 4194304 pixels; the original's limit is the free
   address space of its process, not a number. 16384 by 16384 was created, 32768 by 32768
   and 20000 by 20000 were not. (Section 2.)
10. `Color.new` and `Tone.new` without arguments are (0, 0, 0, 0), `Rect.new` is
    (0, 0, 0, 0); `set(other)` copies the four values, between `Color` and `Tone` too.
    (Section 3.)
11. Version 3.0.0.1 differs from 3.0.1.1 in two things only: the overwrite of rule 3, and
    `Color#==` with an object that is no `Color`, which raises `TypeError` where 3.0.1.1
    returns false. (Section 12.)

## Goals / Non-Goals

**Goals:** rules 1 to 7 in `vkxp-core`, rules 3 and 8 in `vkxp-text`, the values of rule 10
in `vkxp-core`, the version of rule 11 readable in `vkxp-vfs`, each chosen by a value and
tested against the measured numbers. XP and VX unchanged.

**Non-Goals:** the glyph shapes of GDI; the interpreter; the classes of the binding and
everything a script would see through them (Follow-ups); text wider than 640 pixels beyond
being cut at the work rectangle (the original wraps it into the row above, seen on two
draws and not fitted); a font without a name; the image-file size limit;
`Graphics.snap_to_bitmap`; anything the report does not measure.

## Decisions

1. **A value of the runtime in each crate.** `vkxp-core` gets `TextRules` with the values
   `Rgss1`, `Rgss2` and `Rgss3`, and `Bitmaps::draw_text_as(id, rect, text, align, rules,
   rasterizer)`. `draw_text` and `draw_text_squeezed` keep their signatures and results and
   call it with the rules their `Squeeze` names; `Squeeze` stays. `Rules` of the scene is
   not touched: it has no value for RGSS3 yet, other changes will give it one, and text
   needs none of it. `vkxp-text` gets `MaskRules`, set on the engine, with the values
   `Plain` (the default: what the engine does today), `Rgss3` (rule 3 as 3.0.1.1) and
   `Rgss3Early` (rule 3 as 3.0.0.1). The binding will map `Runtime::Rgss3` to
   `TextRules::Rgss3` and the guest will set the mask rules; neither is done here.
2. **vkxp follows 3.0.1.1; the glyph rule of 3.0.0.1 is selected by the DLL the game
   ships.** The owner may overrule this decision.
   - Why 3.0.1.1 is the default: it is the later release, two of the three games of the
     corpus ship it, and the overwrite of 3.0.0.1 is a fault its publisher removed (one
     added test in the routine): a glyph erases what its left neighbour drew inside its
     box.
   - Why the rule of 3.0.0.1 is still built: the main test game, Crysalis, ships 3.0.0.1,
     and `docs/decisions.md` says a game is compared with the DLL it ships. The standard of
     the project is the original a game runs on. The rule costs one branch in the routine
     that composes glyphs.
   - What it changes in practice: little. The two rules differ only where the black box of
     a glyph reaches into its left neighbour's ink: in italic styles (98 of 3593 records of
     the draw probe, all italic) and in pairs such as "fW" and "Hj il" in regular Arial (46
     of 1532 records of the VX text probe). And text glyphs are outside the pixel-exact
     standard anyway: a frame comparison against Crysalis differs in the text pixels with
     either rule, as it does for XP and VX. The rule of 3.0.0.1 buys the look of
     overlapping glyphs on a game that ships 3.0.0.1, not identical frames.
   - How the version is known: not by the file name. Capella's Promise ships the 3.0.1.1
     file under the name `RGSS300.dll`. The DLL has a version record (`VS_FIXEDFILEINFO`:
     the signature 0xFEEF04BD, the structure version 0x00010000, then the file version in
     two 32-bit words). Each of the three DLLs of the corpus holds the signature once, and
     the words that follow give 3.0.0.1 and 3.0.1.1. `vkxp-vfs` reads it from the bytes of
     the file by searching for the signature; no parser of the executable format is added.
     A hash of the file would also tell the two apart but knows only the files measured: a
     DLL of another language or a patched one has another hash and the same version.
   - The selection: a version whose first three numbers are 3.0.0 selects `Rgss3Early`;
     every other version, a file without the record and a game without the DLL select
     `Rgss3`. This table is recorded in `docs/decisions.md`; applying it is the guest's
     work (Follow-ups).
   - `Color#==` of 3.0.0.1 is not followed on any DLL: vkxp will return false, as 3.0.1.1.
     Raising where a comparison is false is a fault no script can rely on without
     rescuing it. Recorded as a known difference.
   - The alternative, 3.0.1.1 always: one rule less and one reader less, and italic text
     on a game that ships 3.0.0.1 keeps the ink the original erases. If the owner chooses
     it, task group 4 and the value `Rgss3Early` are dropped and nothing else changes.
3. **Coverage levels.** With mask rules other than `Plain`, the coverage `c` (0 to 255)
   that the rasteriser gives a glyph pixel becomes a level `v = c * 65 / 256` (0 to 64) and
   the mask holds `trunc(v * 255 / 64)`. The second step is rule 3. The first replaces
   GDI's own 65-level answer and is marked `PLACEHOLDER`: it is how Wine's GDI, under which
   the rule was measured, is believed to reduce its rasteriser's 256 levels, and was not
   checked against Wine's source. The requirement fixes only that 0 gives level 0, 255
   gives level 64 and a larger coverage never gives a smaller level. With `Plain` the
   coverage is used as it is, as today for XP and VX.
4. **Overlapping glyphs.** With `Rgss3`, composing a glyph into the mask writes its value
   wherever its level is above 0, also over a larger value of an earlier glyph. With
   `Rgss3Early` it writes every pixel of the glyph's box, zeros included. The box is the
   bitmap the rasteriser returns for the glyph, which stands for GDI's black box. With
   `Plain` the larger value stays, as today.
5. **Work rectangle.** For `TextRules::Rgss3` the draw does not blend the text cell but
   rule 6: `mask(u, v)` is the coverage of the rasteriser's mask at (u, v) and 0 outside
   it. The 640 and 480 are constants with the report as provenance. Nothing is stored
   between calls, so there is no state to add. Text wider than the rectangle after the
   squeeze, and text wider than 640, end at the work rectangle.
6. **Squeeze.** `squeeze_width_rgss3(size, w, E)` returns the `lfWidth` of rule 4 in `f64`
   in the order written, with the constants 7 and 16 named for the System font and marked
   `PLACEHOLDER` (Wine's values; a Windows of another language has another System font).
   The text is then set once with `TextMetrics::narrowed` of that width, the model vkxp has
   for a font of a given average width, and drawn with whatever width that gives. With the
   outline the test and `r` use `E` with the 2 included, and the squeezed `E` gets the 2
   again.
7. **Largest bitmap of RGSS3.** The limit is the one `vkxp-core` has for every bitmap,
   `MAX_BITMAP_PIXELS` = 268435456 pixels (1 GiB of pixels), the limit of the compositors'
   bitmap stores. Reason: the original has no rule to reproduce, only the address space of
   a 32-bit process; every size the measurement saw created (the largest 16384 by 16384,
   exactly this limit) is created, and a bitmap that could not be uploaded is refused
   before memory is taken. The original created 8192 by 44954 in one run, which vkxp
   refuses; recorded as a known difference. Nothing in `vkxp-core` changes; the limit of
   4194304 is in the binding of RGSS2 and stays there.
8. **`Font.exist?` and substituted names.** `TextEngine::has_family_exactly` is true for a
   name that equals, character for character, a family name an available font declares, or
   a name of the substitution table as the table spells it when one of its replacements is
   available. The second half is vkxp's: the original tests against the fonts Windows has,
   and the table is what stands for them here (`Arial` on a machine with Liberation Sans).
   `TextEngine::first_existing(names)` gives the index rule 8 chooses. The lookup of the
   chosen name stays the "Family lookup" of today, which ignores case as GDI's mapper does.
   Names beginning with `@` (GDI's vertical variants, `"@VL Gothic"` is true on the
   original) are not known to the engine and answer false.
9. **What stands for GDI's answers.** Recorded in `docs/decisions.md` with this list.
   - `gmCellIncX`: the advance of the face at the whole pixels per em, rounded as FreeType
     rounds it (requirement "Measurement" of `text-rendering`). Unchanged.
   - The left-bearing correction: the left overhang of the first glyph. The original also
     moves the pen when a later glyph would start left of column 0; vkxp does not. Accepted
     limit.
   - `tmHeight` and `tmAscent`: the rounded ascent plus the rounded descent, and the
     rounded ascent, at that pixels per em ("Size, bold and italic"). Unchanged; the
     decrement in the choice of pixels per em stays a `PLACEHOLDER` of Wine.
   - The right overhang of the last glyph: the glyph's box against its advance. Unchanged.
   - Bold without a bold face: every advance one pixel wider ("Bold and italic faces").
     The original adds 1 per UTF-16 unit; vkxp adds 1 per character, which is the same
     inside the Basic Multilingual Plane. Unchanged; outside it accepted limit.
   - Italic without an italic face: the regular advances and the overhangs of the slanted
     box. Unchanged, GDI's.
   - The 65-level coverage: decision 3.
   - The width a font gets for an `lfWidth`: `TextMetrics::narrowed`. Accepted limit, see
     Follow-ups: today's Wine gives other widths for one measured font.
   - The System font's 7 and 16: constants (decision 6).
   - The font chosen for a missing name: the substitution table and the fallback families.
   All of them are Wine's answers in the measurement; none was measured on Windows.
10. **Font value.** `Font` and `FontDefaults` get `outline` and `out_color`.
    `FontDefaults::rgss3()` is rule 1 with the one name `VL Gothic`. `FontDefaults::new`
    and `FontDefaults::rgss2` have the outline off; their out colour is never read. The
    size stays an integer: the binding will keep the Float a script gave and hand the
    truncated value down, as rule 4 and the report's section 4 want.
11. **Values.** `Color::default()` and `Tone::default()` are (0, 0, 0, 0); `Rect` has that
    default already. `From<Tone> for Color` and `From<Color> for Tone` copy the four
    numbers, which have the same limits in both classes. The raw copies between `Rect` and
    the others are not imitated.

## Implementation notes

- **Samples.** `crates/vkxp-core/tests/data/rgss3-text-samples.txt` is written from
  `ace-v1-bitmap-samples.txt` and holds numbers only:
  - the 24 sweep lines of the gdi probe as size, natural `E` and the pairs `w:lfWidth`
    (2330 pairs), for `squeeze_width_rgss3`;
  - the lines of the edge probes that give a rectangle, an alignment, the outline and the
    touched box, without the lines on text wider than 640 and on fonts without a name;
  - the sixteen draws of the solid glyphs "■" and "|" of the explore probe (cases 0 to 7
    and 16 to 23) with their options and pixel boxes.
  The edge probes fill the bitmap with (7, 8, 9, 0) and draw in the colour
  (250, 180, 60, 255) with the out colour (1, 2, 3, 255) ("the probe's colours" in the
  scenarios); a pixel is touched, or changed, when it no longer is (7, 8, 9, 0), so the
  pixels without ink count.
  A test rasteriser stands for the font: for the edge lines a text 9 wide and 24 high with
  ink anywhere inside; for the solid glyphs a mask whose coverage is the alpha of the draw
  without outline on the transparent bitmap (cases 0 and 16), as the check script takes its
  coverage from the reference draws. The other fourteen draws must then come out pixel for
  pixel. The rule in executable form is `ace-v1-bitmap-draw-check.py`.
- Nothing else of the measurement is committed: no record file, no coverage of letters of
  the fonts of the run-time package, no table of `text_size` in "VL Gothic" (the font is
  not in the repository), no byte of a DLL. The test of the version reader builds its bytes
  in the test.
- **Fonts in tests.** `vkxp-text` tests use `tests/fonts/LiberationSans-Regular.ttf` as
  today. The measurement's "Arial" was Liberation Sans (Wine's replacement), so its
  `text_size` values for "Arial" at size 24 ("Hello" 49 by 23, "W" 20 by 23, "il" 10 by
  23) are values of the committed font. The tests of the mask rules compose glyphs given
  as numbers, as the unit tests of `blit` do today.
- **XP and VX unchanged.** No expected value of an existing test is edited. The tests that
  prove it, all to pass as they are:
  - `crates/vkxp-core/src/text/tests.rs`, all of it, among them `measured_squeeze_levels`,
    `measured_colour_and_alpha`, `squeeze_limit_clips_what_does_not_fit`,
    `zero_coverage_is_blended_like_the_rest_of_the_cell`,
    `without_the_shadow_the_draw_is_that_of_rgss1`,
    `the_shadow_leaves_the_rectangle_and_not_the_bitmap`,
    `the_step_changes_at_the_widths_the_loop_gives` and
    `below_the_last_step_of_rgss2_the_text_starts_at_the_left_edge`;
  - `crates/vkxp-core/src/font.rs`: `initial_defaults`, `initial_defaults_of_rgss2`;
  - `crates/vkxp-text/tests/measured.rs` (`measured_sizes_of_liberation_sans`,
    `measured_widths`, `measured_overhangs`), `tests/raster.rs` (`repeatable`,
    `glyphs_are_placed_at_the_sum_of_the_advances`), `tests/lookup.rs`
    (`substituted_name_counts_as_available`, `case`), and in `src/engine.rs`
    `overlapping_glyphs_keep_the_larger_coverage`;
  - the tests of the text backend over both crates (`measured_squeeze_levels`,
    `measured_squeeze_sweep`, `measured_alignment`, `measured_vertical_placement`), in
    `crates/vkxp/src/font.rs` or where another change has moved them;
  - `crates/vkxp-binding/tests/bitmap_rgss2.rs` (`the_font_defaults_are_those_of_rgss2`,
    `text_is_drawn_with_a_shadow_by_default`, `text_is_squeezed_to_a_fifth_step`,
    `a_new_bitmap_has_at_most_4194304_pixels`) and the cases "font" and "bitmap text" of
    `tests/cases/mod.rs`.
- **Edits outside the named files.** `crates/vkxp-binding/src/font.rs` writes one `Font`
  out field by field (`FontData::blank`); it gets `outline: false` and an out colour. That
  is all the binding gets. If another crate turns out to build a `Font` or a
  `FontDefaults` by its fields, it gets the same two fields and nothing more.
- **As built.** `squeeze_width_rgss3` returns `None` for a text that fits (`E <= w`), where
  the samples write an `lfWidth` of 0. The version of a DLL is `LibraryVersion` of
  `vkxp-vfs`, in the new module `library_version`. The tests of RGSS3 in `vkxp-core` are in
  `src/text/rgss3_tests.rs`, the test of the largest bitmap among them. No file outside the
  scope of the proposal was edited.
- The cost of a draw of RGSS3 is the work rectangle times the passes, at most 640 by 480
  six times. The pixels without ink cannot be skipped: a destination pixel of alpha 0
  takes the colour of the pass with alpha 0, which a later `blur` or `get_pixel` sees.

## Follow-ups

Waiting for the binding over Ruby 1.9.2. Each is measured; the section is that of the
report.

- `Font#outline`, `Font#out_color`, `Font.default_outline`, `Font.default_out_color` and
  the defaults of rule 1 in a game of RGSS3 (`initial_defaults` returns those of RGSS2 for
  `Runtime::Rgss3` today); `Font.default_name` is a String, not an Array. `outline=` and
  `shadow=` read false for nil and false and true otherwise; the `default_` writers keep
  the object given; `out_color=` takes a `Color` only and copies it. (Section 4.)
- `Font#size=` and `Font.default_size=`: 6 to 96 on the truncated value, else
  `ArgumentError` "bad value for size" or "bad value for default_size"; the Float is kept
  for the reader (`size` reads 24.7) and the font is made with the integer part. The
  binding has the limits today and drops the fraction. (Section 4.)
- `Font.new` with three arguments: `ArgumentError` "(3 for 0..2)"; the name is not
  checked. `Font` has no `==` of its own; `Marshal.dump` of a font raises `TypeError`.
- `Bitmap#font=` copies name, size, bold, italic, shadow, outline, colour and out colour;
  `dup` and `clone` copy all eight. (Section 5.)
- `Bitmap#draw_text` calls `draw_text_as` with `TextRules::Rgss3`; align is an integer
  (1.9 is 1; nil or a String raise `TypeError`). (Section 6g.)
- `Font.exist?` through `has_family_exactly`: a Symbol is converted; nil, an Integer and
  an Array answer false without an error. `Font#name=`: an Array through `first_existing`;
  anything else by its `to_s`; the name is cut to 31 characters before the lookup. A font
  without a name (`""`, `[]`) draws nothing; its `text_size` is not to be reproduced.
  (Sections 6g and 7.) `TextBackend` of the binding needs the two new questions.
- `Bitmap.new(w, h)` without the limit of RGSS2 (the binding applies it to
  `Runtime::Rgss3` today); the errors of section 2: `(0.9, 5)` raises `RGSSError`,
  `(10, -5)` is 10 by 5.
- Methods called with no argument raise `ArgumentError` with the messages of section 2;
  the texts of Ruby 1.9 (`wrong number of arguments (E for G)`, `can't convert`).
- `Color.new`, `Tone.new`, `Rect.new` without arguments; `set(other)` returning the
  receiver; `Color#set` and `Tone#set` without arguments; `Rect#set` without arguments
  raising `ArgumentError`; `set(1)` raising `TypeError` "wrong argument type Fixnum
  (expected Data)"; the counts of section 3; `==` with an object of another class false;
  `hash` not by value. (Section 3.)
- The guest sets the mask rules: `Rgss3` for a game of RGSS3, `Rgss3Early` when the
  program read a version 3.0.0 from the DLL the game's `Library` names. The version has to
  reach the guest process in its settings record (change
  `hand-the-guest-process-its-settings`), because the confined guest does not read the
  DLL.
- `docs/compatibility.md` gets the text and bitmap rules of VX Ace and these known
  differences when a game of VX Ace runs: `Color#==` of 3.0.0.1; the largest bitmap;
  `"@VL Gothic"`; substituted names counting as existing; the message of the original when
  "VL Gothic" is missing ("Unable to find VL Gothic font."), which vkxp does not show.

Open after this change, whoever takes it:

- The width for an `lfWidth`. The report says to use the model vkxp has. That model
  (`ceil(natural * k / ave)`) was fitted to the XP measurement, where "HH" in Liberation
  Sans at size 20 is 12, 11, 10 and 8 pixels per letter for the widths 9, 8, 7 and 6. The
  measurements of VX and of VX Ace under today's Wine give 11, 10, 9, 8 and 6 per letter
  for 9, 8, 7, 6 and 5 with the same font and size (the sweep "Arial 20" of the samples:
  `lfWidth` 7 draws 90, vkxp's model 100; 5 draws 60, the model 70). The report of VX says
  the same of its example. This change tests the `lfWidth` asked for, which is the
  runtime's, and not the width it gives, which is GDI's; the model needs a measurement of
  its own before it is changed, for all three runtimes at once.
- "VL Gothic" is an open font but not in the repository, so its measured tables (28392
  values of `text_size`, section 8) are not a test. A test that is skipped without the
  font under `local/fonts`, as for Noto Sans CJK SC today, can hold them.
- Whether "VL Gothic" should get an entry of the substitution table, and whether the
  `Fonts` directory of the run-time package of VX Ace is among the font directories of a
  run, is for the change that starts a game of RGSS3.
- All of decision 9 on Windows.
