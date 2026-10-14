## Context

The oracle on Crysalis with `--font` (both sides draw with the run-time package's
`VL-Gothic-Regular.ttf`) shows text wider in vkxp by about half a pixel per character; the
frames are under `target/oracle-ace-crysalis-font/` of the main checkout (not committed).
The investigation of 2026-10-14 found the rule; its scratch files are under
`target/text-width/` of the main checkout (Python over the system FreeType through ctypes,
and a Rust probe over `skrifa` and `vkxp-text`), none of them part of this change.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-text/src/{engine,face,glyph_cache,line_box,lib}.rs`,
  `crates/vkxp-text/tests/{common/mod.rs,measured.rs,styles.rs,raster.rs}`;
  `crates/vkxp-guest-main/src/font.rs` (`mask_rules`, `FontBackend::for_game`);
  `crates/vkxp-core/src/text.rs` (`TextMetrics`, `narrowed`, `draw_text_rgss3`), read only.
- `docs/decisions.md`, "VX Ace", the part "Text and bitmaps of RGSS3", list "What stands for
  the answers of GDI"; `docs/compatibility.md`, "Font and text of VX Ace".
- The measurement, outside the repository: `../../ref/tools/oracle/measurements/
  ace-v1-bitmap-REPORT.md` ("the bitmap report": sections 6a, 6b, 8, 12) with
  `ace-v1-bitmap-samples.txt` ("the samples": the lines `ts ...` after line 494 and the
  line `STR ...` that names their 39 strings; the `sweep` lines after line 448) and
  `ace-v1-bitmap-tsize-probe.rb`.
- The skrifa API: `skrifa::outline::{HintingInstance, HintingOptions, Engine, Target,
  SmoothMode, DrawSettings, OutlinePen}` in the registry copy of `skrifa-0.44.0`.

What was established, against the 28392 values of the table (VL Gothic and VL PGothic, sizes
6 to 96, regular, bold, italic, bold italic, 39 strings):

| Rule | Widths equal to the table |
|---|---|
| vkxp today (unhinted advance, rounded as FreeType rounds it; overhangs of its own box model) | 22658 (VL PGothic 14091 of 14196, VL Gothic 8567 of 14196); heights 28392 |
| FreeType's hinted advance rounded up, upright styles only, without Wine's fixed-pitch rule | 12574 of 13832 non-empty upright values (only the full-width glyphs of VL Gothic wrong) |
| the same with Wine's fixed-pitch rule | 13832 of 13832 upright |
| with the italic box slanted at its corners (snapped box) | all upright and italic; 6524 of 6916 bold italic |
| with the box of the bold glyph unsnapped (decision 4) | **28392 of 28392** |
| the same, computed by `skrifa` 0.44 with FreeType's choice of engine (decision 2) | **28392 of 28392** |
| the same, with the left-bearing correction for the first glyph only (as vkxp does) | 27664 of 27664 non-empty |

The same rule gives the drawn width `E` (`measure(..., last = true)`, the bitmap report 6b)
of the 12 unsqueezed sweeps of the samples: 12 of 12, among them Liberation Sans ("Arial",
Wine's replacement) 130 for `HHHHHHHHHH` at size 20 and 153 for italic `Hello World` at 33.

Frame 6000 of the oracle (battle log, size 24, outline on, text at x 16): the rule gives the
widths 114, 255 and 241 for the three lines where vkxp has 120, 270 and 260. The ink of the
original ends at 125, 266 and 252, that of vkxp at 131, 281 and 271: the differences 6, 15
and 19 are exactly those of the widths. The glyph starts of the rule (for example `R` at
16 + 1 = 17, the digits of "36" at 177 and 186) are the original's columns, where vkxp's
unhinted `R` starts at 18. The right-aligned numbers of the status window already agreed:
the digits of VL Gothic are 10 pixels in both rules at size 24.

## Goals / Non-Goals

**Goals:**

- `text_size`, the drawn width and the placement of glyphs in a game of RGSS3 are those of
  the original under Wine for every font, as far as `skrifa` hints as FreeType does.
- RGSS1 and RGSS2 do not change in any value.

**Non-Goals:**

- Glyph shapes of GDI. The hinted outlines bring the shapes closer, but the pixels of text
  stay outside the pixel-exact standard.
- The width a font gets for an `lfWidth` (squeezed text): an accepted limit of
  `docs/decisions.md`. The squeezed advances keep their model; only the natural advances
  and the average width they start from change (Follow-ups).
- The left-bearing correction for a glyph after the first (an accepted limit of
  `docs/decisions.md`; no measured value needs it).
- RGSS1 and RGSS2 with fonts that FreeType auto-hints or that are monospaced by panose
  (Follow-ups).
- Linking FreeType or any C library; a font of the run-time package in the repository.

## Decisions

1. **A metric rule beside the mask rule.** `MetricRules { Plain, Rgss3 }` (default `Plain`)
   with `TextEngine::metric_rules` and `set_metric_rules`, as `MaskRules` has. The
   requirement "Mask rules" says sizes do not depend on the mask rule, and the two DLL
   versions, which differ in the mask rule, measured byte-identical `text_size` tables (the
   bitmap report 12), so the metric rule is a separate switch. Everything cached that
   depends on it carries it in its key: the glyph cache key gets `hinted: bool`; per-glyph
   hinted metrics are cached by face, glyph, pixels per em and style.

2. **The hinting engine is FreeType's choice, run by `skrifa`.** Wine's GDI loads glyphs
   with `FT_LOAD_DEFAULT | FT_LOAD_IGNORE_GLOBAL_ADVANCE_WIDTH` and the normal target. With
   that FreeType 2.14.3 (the library Wine 11.0 rasterised with) auto-hints a TrueType font
   without `fpgm` whose `prep` is at most 7 bytes, whatever `maxSizeOfInstructions` says
   (probed on copies of VL Gothic with the table sizes changed: `prep` 0, 6, 7 auto-hinted,
   8 not; `maxSizeOfInstructions` 46 or 0 made no difference), and uses the font's own
   engine otherwise (the TrueType interpreter, version 40; the PostScript hinter for CFF).
   `skrifa`'s own `Engine::AutoFallback` picks the interpreter for any non-empty `prep`,
   which is wrong for VL Gothic, so the engine passes `Engine::Auto(None)` or
   `Engine::Interpreter` itself, with `Target::Smooth { mode: SmoothMode::Normal,
   symmetric_rendering: true, preserve_linear_metrics: false }` (FreeType "behaves as if"
   the first is on and the second off, by `skrifa`'s documentation). `swash`'s own hinting
   cannot be used: it hints for an LCD target with linear metrics preserved.
   Evidence that `skrifa` 0.44 hints as FreeType 2.14.3: per glyph and size from 4 to 80
   pixels per em, the hinted advance in whole pixels was equal in 51051 of 51051 pairs over
   Liberation Sans, Noto Sans Symbols 2, VL Gothic and VL PGothic; the box on whole pixels
   was equal in all but 44 pairs, all composite glyphs of VL Gothic (accented capitals)
   whose top or bottom differs (x always equal).
   One `HintingInstance` per face and pixels per em, made on first use and kept (sizes are
   6 to 96; the auto-hinter's instance computes style metrics once).

3. **The advance (`gmCellIncX`).** The hinted advance (`AdjustedMetrics::advance_width`,
   26.6 in FreeType) rounded up to whole pixels. Then Wine's fixed-pitch rule: in a font GDI
   reports as fixed pitch, with `a = xAvgCharWidth`, `s = MulDiv(ppem, 65536, upm)` and
   `avg = MulFix(a, s)`, a glyph whose rounded-up advance equals `MulFix(2 a, s)` gets
   `2 avg`. "Fixed pitch" is taken as OS/2 panose `bProportion` = 9 (VL Gothic has 9, VL
   PGothic 3; neither has `post.isFixedPitch` or FreeType's fixed-width flag). `MulFix` and
   `MulDiv` round as FreeType does (`(a * b + 0x8000) >> 16` on the magnitude; `(a * b + c /
   2) / c`). A synthesised bold adds 1, as today. Measured: `あ` 18 at size 24 (ppem 19, the
   face's hinted 19), 6 at size 6 (ppem 5, hinted 5); VL PGothic, not fixed pitch, keeps its
   advances. Control characters and characters outside the Basic Multilingual Plane keep
   today's handling (the missing glyph, twice for a pair); the table agrees.

4. **The box and the overhangs (`gmptGlyphOrigin.x`, `gmBlackBoxX`).** The box is the
   control box of the hinted outline (every point, off-curve ones included, as
   `FT_Outline_Get_CBox`):
   - not emboldened: on whole pixels outwards (left and bottom down, right and top up), as
     FreeType grid-fits the metrics of a hinted glyph;
   - emboldened (synthesised bold): the box of the hinted outline after the emboldening
     vkxp already applies (`Outline::embolden` with the strength of today, which grows the
     outline by a 24th of the em to the right and upwards; Wine's synthesised bold, as far
     as its source is remembered and not read here, is `FT_Outline_Embolden` by
     `ppem / 24`), not on whole pixels;
   - italic (synthesised): the four corners of that box slanted by `x + y / 4` (the slant
     of vkxp today, which reproduces the table), the left edge rounded down, the right edge
     rounded up.

   The left overhang of the first glyph is `max(0, -left)`; the right overhang of the last
   glyph is `max(0, right - advance)`, which makes `E` = the pen plus
   `max(left + width, advance)` of the bitmap report 6b. Measured: the bold italic values
   need the unsnapped box (VL Gothic `j` at size 6: hinted box (0, -1) to (2, 5) slants to
   -0.25, a left overhang of 1, as measured in italic; the emboldened box (0.516, -1) to
   (1.906, 5.031) slants to 0.266, no overhang, as measured in bold italic). Whether the
   emboldening of the box is applied made no difference to the 28392 values; the strength
   is the placeholder vkxp has today.

5. **The average width (`tmAveCharWidth`).** `MulFix(xAvgCharWidth, MulDiv(ppem, 65536,
   upm))`, at least 1, plus 1 for a synthesised bold. Measured: 9 for VL Gothic at size 24
   (the samples, "face selected for VL Gothic ... ave 9"), where today's `round(a * ppem /
   upm)` gives 10; it equals the measured average width of Liberation Sans at all 91 sizes
   of `tests/data/text-samples.txt`, as today's rule does. In RGSS3 the average width only
   enters `TextMetrics::narrowed`, the squeeze model.

6. **Glyph images from the hinted outline.** With the rule of RGSS3 the mask of a glyph is
   rasterised from the same hinted outline, so that ink lies inside the box of decision 4
   and the pen places it as GDI does. Implementation: `swash`'s `Scaler::scale_outline`
   gives the unhinted outline as today; its points are replaced, in order, by those of
   `skrifa`'s hinted draw of the same glyph (both are `skrifa` draws with FreeType's path
   style; the point sequences were equal for every glyph tried, 15192 draws over the four
   fonts); then the outline is emboldened, slanted and condensed as today and rendered with
   `swash::zeno` (`Mask` with the transform and `Format::Alpha`), which is what `Render`
   does inside. A glyph whose sequences differ in length is a bug the tests must show, not
   a case with a second path.

7. **RGSS1 and RGSS2 keep the plain rule.** The rule is Wine's GDI, so it would hold under
   Wine for RGSS1 and RGSS2 as well. But for the fonts they were measured with it changes
   nothing: Noto Sans CJK SC is a CFF font, whose hinter does not change advances beyond
   the rounding vkxp already does; Liberation Sans has hinted advances equal to the rounded
   ones in 59752 of 59752 glyph and size pairs (4 to 80 pixels per em), and its committed
   samples of XP (`M`, `Q`, `P` lines) are reproduced by both rules. No measured value of
   RGSS1 or RGSS2 shows a correction, so none is made here. One observation points the
   other way and is left to a measurement of its own (Follow-ups): the known limit of
   milestone V3, a "Missed!" window one pixel wider on the original in Visions & Voices,
   whose `Fonts` hold the Steinem family without `fpgm` or `prep` (auto-hinted): at size 20
   the rule gives 60 for "Missed!" where vkxp gives 59.

8. **Tests and fonts.** The repository has two open test fonts, and the rule is tested on
   them with measured values only where a measurement exists:
   - Liberation Sans (TrueType with `fpgm` and `prep`: the interpreter): the measured
     `text_size` of RGSS3 (`Hello`, `W`, `il` at 24), the measured sizes and average widths
     of all 91 `M` lines, and the measured drawn widths 130 and 153 of the sweeps.
   - Noto Sans Symbols 2 (neither table: the auto-hinter): the engine choice, whole-pixel
     advances, ink inside the box.
   - Copies of Liberation Sans in a temporary directory with the table directory entries of
     `fpgm` and `prep` renamed or their lengths changed (the helper `copy_font` exists):
     the choice at 0, 7 and 8 bytes, and that a hinted mask differs from the plain one.
   - The arithmetic of decisions 3 to 5 as unit tests with the numbers of the measurement
     (VL Gothic: 1000 units per em, `xAvgCharWidth` 500, the hinted advances and boxes as
     FreeType gives them).
   - The table of VL Gothic and VL PGothic: `tests/data/rgss3-text-size-samples.txt` holds
     the 728 lines `ts` with the 39 strings written as hexadecimal UTF-8, as
     `text-samples.txt` writes them. A test in `tests/rgss3_metrics.rs` reads the two fonts
     from the directory `VKXP_RGSS3_FONTS` names and checks all 28392 values; without the
     variable it prints that it is skipped and passes, as `measured.rs` does for Noto Sans
     CJK SC. The flow does not set it; the owner runs it after the merge.

9. **Documents.** `docs/decisions.md`, list "What stands for the answers of GDI" of RGSS3:
   the item on the advance becomes decisions 2 and 3; the item on the right overhang and
   those on synthesised bold and italic take the box of decision 4; a new item for the
   average width (decision 5) and one for the glyph images (decision 6); a sentence for
   decision 7. The item on the left-bearing correction keeps its accepted limit. The items
   on `tmHeight`, the coverage levels, the `lfWidth`, the System font and the missing name
   are unchanged. `docs/compatibility.md`, "Font and text of VX Ace": one sentence that
   text of a game of VX Ace is measured and placed from glyphs hinted as Wine's FreeType
   hints them, glyph shapes still differing. The crate documentation of `vkxp-text`
   ("Metrics") gets the metric rules. `CHANGELOG.md`: one line under "Fixed".

## Implementation notes

- A flow reads `../../ref/` and copies nothing from it but rules and numbers: the `ts`
  lines and the `STR` list of the samples go into `rgss3-text-size-samples.txt` (strings
  re-encoded as hexadecimal UTF-8, a header that says what they are, where they were
  measured and that no font is committed). No Wine, no original runtime, no FreeType. No
  file of `../../ref/ace/rtp/` or `../../games/` is copied anywhere in the clone, also not
  under `local/`.
- Before anything changes, keep `cargo test --locked -- --list` of `vkxp-text`,
  `vkxp-guest-main`, `vkxp-guest-main-rgss3` and `vkxp-core` under `local/`: every name
  must be there at the end, and no expected value of an existing test is edited.
- `skrifa` 0.44.0 is in `Cargo.lock` already; add `skrifa = "0.44"` to
  `[workspace.dependencies]` and `skrifa.workspace = true` to `vkxp-text`. `cargo deny`
  and `cargo xtask deps` are part of the gate. If `cargo` cannot write its cache, set
  `CARGO_HOME="$PWD/target/cargo-home"`.
- `skrifa::FontRef` is built over the same bytes `LoadedFace` holds; the
  `OutlineGlyphCollection` borrows them, so a face keeps the collection or rebuilds it per
  call (cheap). `HintingInstance` is owned and can be cached beside the face.
- Arithmetic in 26.6 integers where FreeType uses them: the hinted advance and the box come
  as `f32` pixels from `skrifa`; multiply by 64 and round to the nearest integer before
  rounding to whole pixels (they are exact multiples of 1/64).
- The slant of decision 4 in 26.6: `x + MulFix(y, 0x4000)`.
- As built: the choice of decision 2 is public as `HintingEngine` and
  `TextEngine::hinting_engine`, which the scenarios of the engine choice test. Per-glyph
  hinted advances are cached by face, pixels per em and glyph (the style adds only the
  pixel of a synthesised bold, which is added after the cache). Where `skrifa` cannot make a
  hinting instance (a program of the font fails), the glyph is drawn unhinted and its
  advance rounded, as `skrifa` does for an instance whose programs turn hinting off; where a
  glyph cannot be drawn at all, it keeps the plain advance. A hinted point sequence whose
  length differs from the unhinted one fails a debug assertion, so the tests show it.
- The VL Gothic table test after the merge (not a flow task):
  `VKXP_RGSS3_FONTS=/home/doremy/Desktop/rgss/ref/ace/rtp/Fonts cargo test -p vkxp-text
  --locked --test rgss3_metrics`, then `cargo xtask oracle` on Crysalis with `--font` as
  before: frame 6000 should show the three lines ending at 125, 266 and 252.

## Risks

- `skrifa`'s auto-hinter differs from FreeType's in the top or bottom of some composite
  glyphs (44 of 11165 glyph and size pairs of VL Gothic). Only the vertical placement of
  such a glyph and, in italic, its left overhang can follow; no measured value of the
  table did.
- A later `skrifa` may hint differently; the version is the one `swash` pins today.
- Hinting costs time on first use of a face and size; the instances and per-glyph metrics
  are cached. The throughput test of `vkxp-text` stays as it is.

## Follow-ups

- The width for an `lfWidth` (squeeze) in RGSS3: a rough check (advances only, overhangs
  left out) of the model `ceil(advance * k / average)` against the 92 distinct squeezed
  widths of the sweeps gave 68 with the hinted advances and the average width of this
  change and 56 with today's average width; misses remain, for example "Arial" (Liberation
  Sans) 20 `HHHHHHHHHH`, where `lfWidth` 7 draws 90 and the model gives 100. Wine probably
  hints at the narrowed width; it needs its own measurement (accepted limit of
  `docs/decisions.md`).
- RGSS1 and RGSS2 with a font FreeType auto-hints or one whose panose says monospaced:
  their widths under Wine would differ from vkxp's in the same way. The V3 limit of
  Visions & Voices (decision 7) is probably this: the game's Steinem fonts have neither
  `fpgm` nor `prep`, and "Missed!" at size 20 is 60 by the rule and 59 in vkxp (which font
  and size the game draws it in was not checked). A `text_size` probe of RGSS2 (and RGSS1)
  with such a font, against both rules, decides whether the metric rule of RGSS3 becomes
  theirs; `docs/decisions.md` then loses that limit.
- Wine's fixed-pitch test was seen on a font whose panose is monospaced only; a font with
  `post.isFixedPitch` set and another panose was not measured.
- The left-bearing correction for a glyph after the first (bitmap report 6b) stays an
  accepted limit; no measured value needs it.
- The vertical placement of a glyph from its hinted box was not measured against the
  original apart from the frames of the oracle.
