# Proposal

## Why

A game sets text by pixel: `Font#size` is the height of the line box of the font it names, and
windows are laid out for the widths that font gives. When the font is not installed, the text
engine draws with a replacement and scales it by the replacement's own line box. Replacements
often have a much taller line box than the Windows font they stand for, so the text comes out
smaller. Measured with the test game: it asks for `黑体` at size 22; drawn with Noto Sans CJK SC,
a Chinese character is 15 pixels wide where the Windows font gives a full 22.

## What Changes

- The engine can know the line box (ascent and descent per em) of a family by name. When a
  font stands in for such a family through the substitution table, it is scaled and placed by
  the replaced family's line box, so its characters get the size the original font would give.
- A built-in list of line boxes for the Windows families of the substitution table, as far as
  they are established, and a way for the caller to give more.
- Fonts that are installed, replacements for families without a known line box, and fallback
  fonts keep their own line box, as today.

Out of scope: matching advance widths character by character (a replacement keeps its own
advances relative to its em), synthesising missing faces, the configuration file that will
expose the caller's entries.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-rendering`: what the requested size refers to when a font is a replacement.

## Impact

- Crate `vkxp-text` only: one new public type, one new field of `FontSources`, the built-in
  list. Callers that build `FontSources` with struct update syntax or its constructors are
  unaffected.
- Text of games that name Windows fonts becomes larger and wider on systems without those
  fonts, closer to the original layout.
