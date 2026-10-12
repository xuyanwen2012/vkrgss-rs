# Tasks

## 1. Mask level

- [x] 1.1 The rule in `present.frag`, the samples file moved beside the old one and read by the test, the corrected comment. Verify: scenarios "Grey" and "Measured colours"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes.

## 2. Blends

- [x] 2.1 One formula in `arith.rs` and `pixel.glsl`, the provenance list, the tests in `vkxp-pixel` and `vkxp-compose-ref`. Verify: the scenarios of `specs/frame-composition/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-pixel -p vkxp-compose-ref -p vkxp-vk --locked` passes with no expected frame regenerated.

## 3. Threshold

- [x] 3.1 The lower bound in `step_mix`. Verify: scenario "Long transition"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host --locked` passes.

## 4. Documentation

- [ ] 4.1 `docs/compatibility.md`: the entries "Blending onto a translucent bitmap" and "Transition masks" go; a sentence says which mask file formats were measured. Verify: `grep -rn PLACEHOLDER crates/vkxp-vk crates/vkxp-pixel` shows neither the mask level nor the two blends.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-coloured-mask-levels-and-drop-translucent-blends --strict` passes.
