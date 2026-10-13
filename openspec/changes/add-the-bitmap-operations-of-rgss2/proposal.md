## Why

RGSS2 adds four bitmap operations (`blur`, `radial_blur`, `gradient_fill_rect`,
`clear_rect`), draws text with a shadow by default, squeezes text one step further than
RGSS1, has other font defaults and differs in a few rules of `Bitmap.new`, `draw_text` and
`font=`. The scripts of a VX game use all of them in the title screen and the menu (the
menu background is a blurred snapshot; every window draws text with a shadow). All of it
was measured on the original runtime of VX on 2026-10-12 with no wrong value
(`vx-v1-bitmap-REPORT.md` with its models and samples under
`ref/tools/oracle/measurements/`, outside the repository), on both DLL versions.

## What Changes

- In RGSS2: `Bitmap#blur`, `radial_blur`, `gradient_fill_rect`, `clear_rect`.
- In RGSS2: `Font#shadow`, `Font.default_shadow`, the shadow of `draw_text`, the fifth
  squeeze step, the font defaults of RGSS2, a text argument of any class.
- In RGSS2: `Bitmap.new` raises for a file that cannot be decoded and for a size above
  4 194 304 pixels; `font=` does not copy the shadow.
- XP behaves as before in every case.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-bitmap`: the operations of RGSS2 and their arithmetic; the shadow and the squeeze.
- `rgss-bitmap`: the methods of `Bitmap` and `Font` by runtime.

## Impact

- `crates/vkxp-pixel`, `crates/vkxp-core` (`bitmap.rs`, `text.rs`, `font.rs`),
  `crates/vkxp-text` if the squeeze needs it, `crates/vkxp-binding` (`bitmap.rs`,
  `font.rs`, the method list of RGSS2 and its test).
- `docs/compatibility.md`.
- Not touched: the window and the scene (`crates/vkxp-core/src/window.rs`, `scene.rs`,
  `crates/vkxp-binding/src/window.rs`): the change `draw-the-window-of-rgss2` runs in
  parallel there; `graphics.rs` of the binding: `resize-the-screen-in-rgss2` runs there.
  The protocol, the host and the compositors are not touched.
