# Design

## Context

- `vkxp-text` resolves a requested family through caller directories, system fonts and a
  substitution table, sets the size as the height of the line box, and for a replacement font
  uses the line box of the family it replaces (`line_box.rs`) instead of its own.
- The harness option `--font FILE` installs the file on both sides and maps common game font
  names to it; a comparison of the test game's menu then showed the original's text smaller
  than vkxp's, by the ratio of the two line boxes.
- `vkxp` has `--font-dir` and layered configuration; the guest's run configuration carries the
  font settings to the text engine.

## Decisions

- A boolean in the text engine's configuration, default off, that skips the replaced-font line
  box; threaded from the option through the run configuration.
- The harness adds the option exactly when it shares fonts.
- The text engine is built in the binary (`FontBackend::for_game`) and handed to the guest's
  configuration as its text backend, so the flag goes from the option into the engine's
  sources there; `vkxp-binding` itself needs no change.

## Follow-ups

- Whether GDI's rounding of the scaled ascent and descent matches the engine's is what the
  first comparison with this option will show.
