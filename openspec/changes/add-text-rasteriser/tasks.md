# Tasks

## 1. Fonts and lookup (`vkxp-text`)

- [x] 1.1 Add the dependencies and copy the two test fonts from `local/fonts/` into `crates/vkxp-text/tests/fonts/` together with the SIL Open Font License text and a short README naming their origin. Verify: `cargo build -p vkxp-text` has no warnings, `cargo xtask deps` reports no violation, and the directory holds exactly two font files and the licence files.
- [x] 1.2 Implement the font database from system fonts and caller directories with caller priority, and the availability query. Verify: tests for the two font-source scenarios and the two availability scenarios in `specs/text-rendering/spec.md`, using engines built from temporary directories that contain copies of the test fonts.
- [x] 1.3 Implement the substitution table (built-in entries of `design.md`, caller entries first) and the family lookup with its cache. Verify: tests for the four family-lookup scenarios and the two default-substitution scenarios, including that every family listed in the requirement has an entry.

## 2. Measuring and rasterising (`vkxp-text`)

- [x] 2.1 Implement scaling to the requested line height, the baseline, bold and italic face selection and whole-pixel advances. Verify: tests for the size-bold-italic scenarios that can run with the test fonts (height equals size at 12, 22 and 52; a bold request on a regular-only family uses the regular face) and for the two measurement scenarios.
- [x] 2.2 Implement glyph rendering into coverage masks with the bounded glyph cache. Verify: tests for the four rasterisation scenarios.
- [x] 2.3 Implement per-character fallback, including a character no font has. Verify: tests for the two fallback scenarios, using a symbol that only Noto Sans Symbols 2 covers.
- [x] 2.4 Add the smoke test on system fonts and a throughput check. Verify: an engine built from system fonts measures and rasterises an ASCII string and a Chinese string without error; rasterising 1000 short strings at size 22 after warm-up takes under one second in a release-profile test run recorded in the commit message.

## 3. Integration check

- [x] 3.1 Document the crate in `crates/vkxp-text/src/lib.rs` (sources, lookup order, metrics rule marked as placeholder) and run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-text-rasteriser --strict` passes.
