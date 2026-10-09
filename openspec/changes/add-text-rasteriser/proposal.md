# Proposal

## Why

`Bitmap#draw_text` in `vkxp-core` lays text out but gets its glyphs from a rasteriser interface
that has no real implementation. Games ask for Windows fonts by name (the test game asks for
`楷体_GB2312`, `黑体` and `Arial`), which a Linux system does not have, and routinely draw
Chinese or Japanese text in a Latin font, relying on the system to find the glyphs elsewhere.
Without a rasteriser that handles both, a title screen has no text.

## What Changes

- `vkxp-text` gains a text engine that:
  - builds a font database from the system's fonts and from directories given by the caller,
    the latter taking priority;
  - resolves a list of requested family names through a substitution table (built-in defaults
    for the common Windows fonts, extendable by the caller) to an installed face, choosing bold
    and italic faces when they exist;
  - falls back per character to other fonts when the chosen face lacks a glyph;
  - measures a string and rasterises it into an 8-bit coverage mask at a pixel size, with glyph
    caching;
  - answers whether a family name is available.
- Two small openly licensed fonts are committed as test inputs, with their licences.

Out of scope: text shaping, bidirectional text, synthetic bold and italic, colour emoji,
subpixel rendering, reading the configuration file (the caller passes directories and
substitutions), and the adapter that implements core's rasteriser interface (it lives in the
binding, because `vkxp-text` and `vkxp-core` stay independent of each other).

## Capabilities

### New Capabilities

- `text-rendering`: finding fonts for requested family names and turning strings into glyph
  coverage at a pixel size.

### Modified Capabilities

None.

## Impact

- Crate `vkxp-text` and its tests. New dependencies: `fontdb` (font discovery) and `swash`
  (scaling and rasterising); `Cargo.toml` and `Cargo.lock` change accordingly.
- New test assets under `crates/vkxp-text/tests/fonts/` (about 1.1 MB): Liberation Sans Regular
  and Noto Sans Symbols 2, both under the SIL Open Font License, with the licence text.
- The binding will adapt this engine to `vkxp_core::TextRasterizer`.
