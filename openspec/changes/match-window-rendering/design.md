# Design

## Context

- Measurements: `local/measurements/README.md`, the eight sections whose headings start with
  "Window:", `window-samples.txt`, and `window-model.py`, a reference implementation of every
  rule that reproduces all captured frames (all in this clone, ignored by git). Read the
  sections first; they hold the conditions and positions this document only names. A selection
  of the samples is committed as test data; the Python model is not committed, its rules are
  implemented in Rust.
- `vkxp-core` draws a window from cached bitmaps (background, frame, cursor) as normal-mode
  quads at the window's z, and contents, cursor, arrows and pause sign at z + 2, in an order
  the measurement confirms: lower layer background then frame, upper layer cursor, contents,
  arrows, pause sign. `stretch_blt` and `blt` already follow the original's rules.
- `vkxp_proto::BlendMode` has normal, additive and subtractive; `PROTOCOL_VERSION` is 3. The
  blend arithmetic lives in `vkxp-pixel` and is mirrored in `shaders/pixel.glsl`.
- The removed requirements also said that a window keeps its place in the draw order and its
  layers' z; that behaviour stays and is covered by the "Draw order" requirement of the same
  spec. If a scenario of a removed requirement states something the new requirements do not
  and the measurement confirms it, keep its test.

## Decisions

- New `BlendMode::Window`. The odd-pixel rule needs, per row, the visible width of the quad:
  the intersection of its destination coverage with its clip and the frame. Window layers are
  axis-aligned and unscaled on screen, so the compositors compute that span from the quad
  itself; the rule is defined only for such quads and other quads in this mode fall back to
  the plain window formula.
- Background: a stretch quad cannot express the numerator 127, so the stretched background is
  rendered into a cached bitmap of the inner size with the rule of the README and drawn 1:1;
  the tiled form likewise. Frame and cursor are composed into cached bitmaps with `blt`, as
  measured. Caches are rebuilt when size, skin or the skin's content changes.
- Cursor and pause state follow the measured counters exactly; `active = true` on an active
  window and `pause = true` while pausing leave the counters alone (not measured; PLACEHOLDER).
- Provenance of the new blend function: reverse-engineered,
  `oracle measurement 2026-10-09, RGSS103J, window probes`.

## Follow-ups

- Viewport tone, colour and flash on windows; the two unmeasured assignments above.
