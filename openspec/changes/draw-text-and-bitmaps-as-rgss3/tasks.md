# Tasks

## 1. Values

- [x] 1.1 `outline` and `out_color` in `Font` and `FontDefaults`, `FontDefaults::rgss3()`, the two fields where the binding writes a `Font` out. Verify: the scenarios of "Outline attributes of a font" and "Font defaults of RGSS3"; `initial_defaults` and `initial_defaults_of_rgss2` pass unchanged; `cargo test -p vkxp-core -p vkxp-binding --locked` passes.
- [x] 1.2 The empty colour and tone and the conversions between them. Verify: the scenarios of "Empty colour and tone, and copies between them"; `cargo test -p vkxp-core --locked` passes.

## 2. Masks and names

- [x] 2.1 The mask rules of the engine: the 65 levels and their scaling, the two rules for overlapping glyphs, the plain rule as the default. Verify: the scenarios of "Mask rules", "Coverage levels of RGSS3" and "Overlapping glyphs of RGSS3"; the existing tests of `crates/vkxp-text` pass with their expected values unchanged; `cargo test -p vkxp-text --locked` passes.
- [x] 2.2 Exact family names and the name of a name list; the three sizes measured on RGSS3. Verify: the scenarios of "Exact family names", "Name of a name list in RGSS3" and "Text size measured on RGSS3"; the same command.

## 3. Text of RGSS3

- [ ] 3.1 `TextRules`, `draw_text_as`, and `squeeze_width_rgss3` with the committed sweep pairs. Verify: the scenarios of "Text rules by runtime" and "Text squeezing of RGSS3"; the existing tests of `crates/vkxp-core/src/text/tests.rs` pass with their expected values unchanged; `cargo test -p vkxp-core --locked` passes.
- [ ] 3.2 Placement and the work rectangle, with the committed lines of the edge probes. Verify: the scenarios of "Text placement of RGSS3", "Work rectangle of RGSS3" and "Text measurement of RGSS3"; the same command.
- [ ] 3.3 The passes: outline, shadow, text, with the committed draws of the solid glyphs. Verify: the scenarios of "Text passes of RGSS3" and "Outline of RGSS3"; the same command.
- [ ] 3.4 The largest bitmap. Verify: the scenarios of "Largest bitmap of RGSS3"; the same command.

## 4. Version of the library

- [ ] 4.1 The version of a DLL from its bytes in `vkxp-vfs`, tested on bytes built in the test. Verify: the scenarios of "Version of the library of RGSS3"; no file of a game or of `ref/` is read by a test; `cargo test -p vkxp-vfs --locked` passes.

## 5. Decisions

- [ ] 5.1 `docs/decisions.md`, section "VX Ace": the decisions 2, 7 and 9 of `design.md`. Verify: the scenarios of "Recorded decisions on text and bitmaps of RGSS3"; `git diff --stat` names no file under `docs/` but `decisions.md`.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-text-and-bitmaps-as-rgss3 --strict` passes.
