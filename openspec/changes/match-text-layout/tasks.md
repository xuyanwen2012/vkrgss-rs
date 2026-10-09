# Tasks

## 1. Engine (`vkxp-text`)

- [x] 1.1 Size fonts at whole pixels per em with the measured rule and report height, ascent, advances, overhangs and average width. Verify: the scenarios of the two modified requirements in `specs/text-rendering/spec.md`, with a selection of the measured samples committed as test data.
- [x] 1.2 Synthesise bold and italic when the family has no such face, and rasterise runs with given advances. Verify: the bold-face-absent scenario and a test that a condensed run has the requested advances.

## 2. Drawing (`vkxp-core`, `vkxp-binding`)

- [ ] 2.1 Extend the rasteriser interface and its adapter, and implement vertical placement, overhang-aware alignment and the whole-cell blend. Verify: the scenarios of the text-drawing and text-measurement requirements in `specs/core-bitmap/spec.md` except the squeeze ones.
- [ ] 2.2 Implement squeezing by steps. Verify: the two squeeze scenarios and the measured-levels scenario.
- [ ] 2.3 Raise the argument errors. Verify: Ruby tests for the two scenarios of `specs/rgss-bitmap/spec.md`.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-text-layout --strict` passes.
