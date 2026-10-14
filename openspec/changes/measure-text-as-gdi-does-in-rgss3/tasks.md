# Tasks

## 1. Metrics of RGSS3 in the text engine

- [x] 1.1 Before anything changes, keep `cargo test --locked -- --list` of `vkxp-text`, `vkxp-guest-main`, `vkxp-guest-main-rgss3` and `vkxp-core` under `local/`. Verify: the lists are kept.
- [x] 1.2 `skrifa` in the workspace dependencies and in `vkxp-text`; `MetricRules` with `metric_rules` and `set_metric_rules`, exported; the module `src/hinted.rs` with the choice of the hinting engine, the hinting instances per face and pixels per em, and the arithmetic of design decisions 3 to 5 (advance with the fixed-pitch rule, box and overhangs, average width), with unit tests on the numbers of decisions 3 to 5 and the tests of design decision 8 on the test fonts and their copies in the new `tests/rgss3_metrics.rs`; `TextEngine::layout` and `measure` by the metric rule (design decision 1). Verify: the first two scenarios of "Metric rules" and the scenarios of "Hinted outlines of RGSS3", "Advances of RGSS3", "Glyph box of RGSS3" and "Average width of RGSS3"; `cargo test -p vkxp-text --locked` passes; `cargo xtask deps` passes; `git diff` of `crates/vkxp-text/tests/` against the start of the change adds lines and removes none.
- [x] 1.3 Glyph images of RGSS3 from the hinted outline (design decision 6), the glyph cache key with the metric rule, and the overhangs of the mask from the box of decision 4. Verify: the scenarios of "Glyph images of RGSS3" and the last scenario of "Metric rules"; the same test command passes.
- [x] 1.4 `tests/data/rgss3-text-size-samples.txt` from the `ts` lines and the `STR` line of the samples (design decision 8 and the implementation notes) and `tests/rgss3_metrics.rs` with the test over `VKXP_RGSS3_FONTS` and the measured sizes of `Hello`, `W` and `il` at 24 in Liberation Sans with the metric rule of RGSS3; `tests/measured.rs` keeps its tests as they are. Verify: the scenarios of "Text size measured on RGSS3"; `cargo test -p vkxp-text --locked` passes without the variable and prints that the table test is skipped; the samples file holds 728 lines `ts` with 39 values each and names no font file of the repository.

## 2. The metric rule of a game

- [x] 2.1 In `crates/vkxp-guest-main/src/font.rs`: the metric rule of a game by its runtime, set by `FontBackend::for_game`, with its unit tests. Verify: the scenarios of "Metric rule of a game of RGSS3"; `cargo test -p vkxp-guest-main -p vkxp-guest-main-rgss3 --locked` passes; `git diff --stat` against the start of the change names no file of `crates/vkxp-core`, `crates/vkxp-binding`, `crates/vkxp-guest`, `crates/vkxp-proto` or `crates/vkxp-host`.

## 3. Documents

- [x] 3.1 `docs/decisions.md`, `docs/compatibility.md` and `CHANGELOG.md` (design decision 9); the crate documentation of `vkxp-text` in `src/lib.rs` ("Metrics") and the documentation of `crates/vkxp-guest-main/src/font.rs` where it describes the backend of a game. Verify: the scenarios of `specs/supported-runtimes/spec.md`; the text names nothing that the code does not do; `git diff --stat` against the start of the change names no file under `docs/` but those two; `cargo doc -p vkxp-text --no-deps` has no warning.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: every name of the lists kept in 1.1 is there again; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate measure-text-as-gdi-does-in-rgss3 --strict` passes.
