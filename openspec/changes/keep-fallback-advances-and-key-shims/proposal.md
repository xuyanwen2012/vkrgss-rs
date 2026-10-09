# Proposal

## Why

Two small things found after the text layout change and the survey of more games:

- A character that the selected font lacks is drawn from a fallback font but set at the
  selected font's missing-glyph advance. That is what the original shows under Wine, which has
  no font linking; on Windows a linked font keeps its own advance. For play it makes Chinese
  text in a Latin-only font overlap (the missing-glyph advance of the Latin test font is
  three quarters of an em).
- Three games call `keybd_event` or `SetKeyboardState` through `Win32API` to synthesise or
  clear key presses; vkxp reports each as not emulated on every start.

## What Changes

- A fallback glyph keeps its own advance, in measuring and in drawing.
- `keybd_event`, `SetKeyboardState` and `mouse_event` are accepted silently and do nothing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-rendering`: advances of fallback glyphs.
- `rgss-runtime`: three more accepted functions.

## Impact

- Crates `vkxp-text` and `vkxp-binding` (`win32api.rs`).
