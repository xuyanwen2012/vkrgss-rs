# Tasks

## 1. Line boxes (`vkxp-text`)

- [ ] 1.1 Add `LineBox`, the `line_boxes` field of `FontSources`, the built-in list of `design.md` with its status comments and `PLACEHOLDER` marks, and `default_line_box`. Verify: unit tests for case-insensitive lookup, caller precedence, and the scenario that the built-in line box of `Arial` equals the own line box of the Liberation Sans test font.
- [ ] 1.2 Make resolution return the line box of the replaced family and let scaling and baseline use it. Verify: tests for the four scenarios of the line-box requirement and the two remaining scenarios of the known-line-boxes requirement in `specs/text-rendering/spec.md`, on an engine built from the test fonts alone; the existing tests still pass unchanged.
- [ ] 1.3 Update the crate documentation (the "Metrics" section and the example of sources). Verify: `cargo doc -p vkxp-text --no-deps` builds without warnings.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate scale-substitutes-to-replaced-fonts --strict` passes.
