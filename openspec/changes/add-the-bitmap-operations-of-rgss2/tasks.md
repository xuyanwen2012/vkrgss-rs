# Tasks

## 1. Pixel operations

- [x] 1.1 `blur`, `gradient_fill_rect`, `clear_rect`: arithmetic, provenance entries, core functions. Verify: the scenarios of "Blur of RGSS2", "Gradient of RGSS2" and "Clearing a rectangle"; `cargo test -p vkxp-pixel -p vkxp-core --locked` passes.
- [x] 1.2 `radial_blur`. Verify: the scenarios of "Radial blur of RGSS2"; the same command.

## 2. Text

- [ ] 2.1 The shadow layer and the squeeze of RGSS2. Verify: the scenarios of "Text shadow of RGSS2" and "Text squeezing of RGSS2"; `cargo test -p vkxp-core -p vkxp-text --locked` passes.

## 3. Classes

- [ ] 3.1 The four methods of `Bitmap`, `Font#shadow` and `Font.default_shadow`, the font defaults, the text argument, `Bitmap.new` and `font=` by runtime; the method list and its pending list. Verify: the scenarios of `specs/rgss-bitmap/spec.md`; `cargo test -p vkxp-binding --locked` passes.

## 4. Documentation

- [ ] 4.1 `docs/compatibility.md`: the bitmap and text rules of VX and what is outside the standard. Verify: the text names nothing that the code does not do.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-the-bitmap-operations-of-rgss2 --strict` passes.
