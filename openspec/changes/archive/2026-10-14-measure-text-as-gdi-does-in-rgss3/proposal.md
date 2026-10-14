## Why

`cargo xtask oracle` on the VX Ace game Crysalis, with `--font` making both sides draw with
the run-time package's `VL-Gothic-Regular.ttf`, differs only in text, and not only in glyph
shapes: vkxp's text is wider. In frame 6000 the battle log line "Mio attacks!" spans the
columns 17 to 125 on the original and 17 to 131 in vkxp, "Ratinga suffered 36 damage!" 17 to
266 against 18 to 281, "Ratinga has been defeated!" 17 to 252 against 18 to 271. The owner
accepted that glyph shapes differ; positions, widths and placement must be the original's.

The measurement of 2026-10-13 (`ace-v1-bitmap-REPORT.md`, section 8, and the `text_size`
table of VL Gothic and VL PGothic, sizes 6 to 96 in four styles, 28392 values, outside the
repository) says `text_size` is the sum of `gmCellIncX` of `GetGlyphOutlineW`, which Wine's
GDI takes from FreeType with hinting on. vkxp takes the unhinted advance of the face,
rounded: 22658 of the 28392 widths equal the table (every height does). The rule of this
change, worked out against the table on 2026-10-14, reproduces all 28392:

- FreeType hints VL Gothic with its automatic hinter (no `fpgm`, a `prep` of 7 bytes), which
  narrows "H" at 19 pixels per em from 9.5 to 9 pixels; VL PGothic with its interpreter.
- Wine gives a full-width glyph of a font whose panose says monospaced twice the average
  width (18 for "あ" at size 24 where the face's advance is 19).
- The left-bearing correction comes from the box of the hinted outline, slanted at its
  corners in italic, and in a synthesised bold from the emboldened outline as it is.

With it the battle log of frame 6000 is 114, 255 and 241 pixels wide, 6, 15 and 19 less than
vkxp draws it: exactly the differences of the frame.

## What Changes

- `vkxp-text` gets metric rules beside its mask rules: the plain rule, the default, is
  today's for every runtime; the rule of RGSS3 measures and draws from outlines hinted as
  FreeType 2.14 hints them (through `skrifa`, already in the dependency tree under `swash`,
  with the automatic hinter chosen by FreeType's rule): the hinted advance rounded up, Wine's
  fixed-pitch rule for full-width glyphs, synthesised bold plus 1, the overhangs from the
  hinted box (slanted in italic), the average width as GDI's `tmAveCharWidth`, and glyph
  images rasterised from the same hinted outlines.
- The guest of a game of RGSS3 sets the metric rule of RGSS3 (both DLL versions measured the
  same table). RGSS1 and RGSS2 keep the plain rule.
- The `text_size` table of VL Gothic and VL PGothic is committed as numbers, with a test that
  runs when `VKXP_RGSS3_FONTS` names a directory with the two fonts and says it is skipped
  otherwise; no font of the run-time package is committed or copied.
- `docs/decisions.md` replaces the items on the advance, the overhangs and the average width
  in its list of what stands for GDI in RGSS3; `docs/compatibility.md` says the text of VX
  Ace is measured from hinted glyphs; `CHANGELOG.md`.
- XP and VX are unchanged: for the fonts they were measured with (Noto Sans CJK SC, whose
  CFF hinting never changes an advance, and Liberation Sans, whose hinted advances equal the
  rounded ones at every size from 4 to 80 pixels per em) both rules give the same advances,
  so no measured value shows a correction there. The one-pixel "Missed!" window of
  Visions & Voices (a known limit of milestone V3, a game whose fonts are auto-hinted) is
  probably the same rule; it is a Follow-up that needs a probe of RGSS2.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-rendering`: metric rules; hinted outlines, advances, glyph box, average width and
  glyph images of RGSS3; the text size of RGSS3.
- `game-launch`: the metric rule of a game of RGSS3.
- `supported-runtimes`: the recorded text metrics of RGSS3 and the sentence of
  `docs/compatibility.md`.

## Impact

- `crates/vkxp-text`: `Cargo.toml` (`skrifa`), `src/engine.rs` (the metric rule, setting a
  request with it, the glyph cache key), `src/face.rs` (the tables the rule reads), a new
  module `src/hinted.rs` (hinting instances and the arithmetic of decisions 2 to 5),
  `src/glyph_cache.rs` (one field of the key), `src/lib.rs` (export and crate
  documentation); tests: `tests/rgss3_metrics.rs` (new), unit tests in the files named,
  `tests/data/rgss3-text-size-samples.txt` (new, numbers and strings of the probe).
- The workspace `Cargo.toml` (`skrifa = "0.44"`, the version `swash` 0.2.10 already pulls)
  and `Cargo.lock` (one line: `vkxp-text` depends on `skrifa`).
- `crates/vkxp-guest-main/src/font.rs`: the metric rule of a game, with its tests.
- `docs/decisions.md`, `docs/compatibility.md`, `CHANGELOG.md`.
- Not touched: `vkxp-core` (its `TextMetrics` and the RGSS3 rules of `draw_text` take the
  engine's numbers as they are), the binding, the harness's cell backend, the protocol, the
  host, the compositors; the squeeze (the width a font gets for an `lfWidth` stays an
  accepted limit).
- No original DLL, no font of the run-time package, nothing of a game is committed; FreeType
  is not linked.
