# Proposal

## Why

With the same font file on both sides, the original (through Wine's GDI) and vkxp still draw
text at different sizes: vkxp scales a replacement font to the line box of the Windows font it
stands for, while GDI sizes the given font by its own line box. That scaling is right for
playing a game without the Windows fonts, but it makes the harness's text comparison
meaningless: what the comparison can check is the rule "font size is the height of the font's
own line box" and the placement and advances that follow from it.

## What Changes

- The text engine can be told to size every font by its own line box, without the
  replaced-font scaling.
- `vkxp --font-own-metrics` and the configuration key `font_own_metrics` select that.
- The oracle harness passes the option to vkxp whenever `--font` is given.

Out of scope: changing the default behaviour, tuning the stand-in line boxes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-rendering`: sizing by a font's own line box on request.
- `game-launch`: the option.
- `oracle-harness`: uses it with shared fonts.

## Impact

- Crates `vkxp-text`, `vkxp-binding` (run configuration passes the flag on), `vkxp`, and
  `xtask`.
