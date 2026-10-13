## Why

A game of RPG Maker VX Ace draws every line of text with an outline, in "VL Gothic" at size
24, and RGSS3 changed how `draw_text` squeezes, what it touches and how glyph coverage is
scaled. All of it was measured on the original runtime on 2026-10-13 with no wrong pixel
(`ace-v1-bitmap-REPORT.md` with its check script and samples under
`ref/tools/oracle/measurements/`, outside the repository), on both DLL versions that games
ship. The interpreter of RGSS3 and its binding do not exist yet, but these rules live in
crates that know no interpreter. Implemented and tested there now, behind a value that
names the runtime, the pixels are right on the day the binding first runs over Ruby 1.9.2.

## What Changes

- `vkxp-core` draws text by the rules of a runtime given as a value. For RGSS3: the drawn
  width and the text height with the outline's 2, the placement with the `+ 1` when
  centring, the work rectangle (the whole rectangle given, at most 640 by 480, nothing kept
  between calls), the one-step squeeze with the System font's 7 and 16, and the passes:
  four diagonal outline passes in the out colour, the shadow, the text.
- The font value of `vkxp-core` gets `outline` and `out_color`, and the font defaults of
  RGSS3: "VL Gothic", 24, no shadow, outline, out colour (0, 0, 0, 128).
- `Color` and `Tone` of `vkxp-core` get the value of the no-argument forms, (0, 0, 0, 0),
  and the copy between the two classes that `set(other)` makes.
- `vkxp-text` can make a mask by the rules of RGSS3: coverage in 65 levels scaled by
  `trunc(v * 255 / 64)`, and a later glyph replacing an earlier one, by the rule of DLL
  3.0.1.1 or by that of 3.0.0.1. It answers `Font.exist?` of RGSS3 (exact, case-sensitive)
  and chooses the name of a name list.
- `vkxp-vfs` reads the version of an RGSS3 DLL from its bytes, because the file name does
  not tell it (one game of the corpus ships 3.0.1.1 as `RGSS300.dll`).
- `docs/decisions.md` records three decisions: vkxp follows 3.0.1.1 and selects the glyph
  rule of 3.0.0.1 by the version of the DLL a game ships; the largest bitmap of RGSS3; the
  quantities of GDI that vkxp supplies itself.
- XP and VX behave as before in every case: the defaults of every new switch are the rules
  of today, and no existing test changes its expected values.
- Not in this change: the classes of the binding (`Font#outline`, `Bitmap#font=`, the
  argument errors, `Bitmap.new` without the limit of RGSS2, the value classes' new forms),
  which wait for the binding over Ruby 1.9.2. `design.md` lists each with its measured rule.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-bitmap`: text drawing by runtime; placement, work rectangle, passes and squeeze of
  RGSS3; the largest bitmap of RGSS3.
- `core-values`: the outline attributes and the font defaults of RGSS3; the empty colour
  and tone and the copy between them.
- `text-rendering`: coverage levels and overlapping glyphs of RGSS3; exact family names and
  the choice from a name list.
- `game-files`: the version of the library of a game of RGSS3.
- `supported-runtimes`: the recorded decisions.

## Impact

- `crates/vkxp-core` (`text.rs`, `font.rs`, `color.rs`, `lib.rs`, tests and
  `tests/data/rgss3-text-samples.txt`), `crates/vkxp-text` (`engine.rs`, `index.rs`,
  `substitution.rs`, `lib.rs`, tests), `crates/vkxp-vfs` (a new module and its export),
  `docs/decisions.md`.
- `crates/vkxp-binding/src/font.rs`: only the two new fields where a `Font` is written out
  field by field. No class, method or default of the binding changes.
- Not touched: the rest of `crates/vkxp-binding`; `crates/vkxp` (the change
  `hand-the-guest-process-its-settings` moves its text backend); `crates/vkxp-pixel` (the
  blend arithmetic is that of today); the scene, the window and the sprite of `vkxp-core`
  and its `Rules`; the protocol, the host, the compositors; `docs/compatibility.md`, which
  says nothing of VX Ace until a game of it runs.
- No original DLL, no font of the run-time package and nothing of a game is committed: the
  samples are numbers of the probe's own.
