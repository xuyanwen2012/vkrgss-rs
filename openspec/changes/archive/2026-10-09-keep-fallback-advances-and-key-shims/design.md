# Design

## Context

- `vkxp-text` lays a string out character by character; since the text layout change a
  character the selected font lacks takes the selected font's missing-glyph advance although
  its glyph comes from a fallback font (marked PLACEHOLDER there, with the overlap noted as a
  follow-up). The requirement "Per-character fallback" already says the measured width is the
  sum of each character's advance in the font it came from.
- The committed width samples measured on the original contain characters the measured font
  lacks only as missing glyphs with no fallback in play; tests that pin them run with an engine
  that has no fallback font for those characters, or are adjusted to say so.
- `crates/vkxp-binding/src/win32api.rs` holds the table of emulated functions.

## Decisions

- Fallback advance: the fallback font is sized at the same whole pixels per em as the selected
  font and its rounded advance is used. Control characters and characters no font has keep the
  missing-glyph advance.
- The three functions go into the table as no-ops. They do not alter the raw key state that
  `GetAsyncKeyState` reports (PLACEHOLDER: a game that injects a key to read it back would
  need that).
