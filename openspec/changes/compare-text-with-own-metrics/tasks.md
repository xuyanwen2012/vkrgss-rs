# Tasks

## 1. Engine (`vkxp-text`)

- [ ] 1.1 Add the own-metrics configuration. Verify: the two scenarios of `specs/text-rendering/spec.md`.

## 2. Option (`vkxp-binding`, `vkxp`)

- [ ] 2.1 Add `--font-own-metrics` and its configuration key and pass it to the engine. Verify: the scenario of `specs/game-launch/spec.md` and argument and configuration tests.

## 3. Harness (`xtask`)

- [ ] 3.1 Pass the option when fonts are shared. Verify: the scenario of `specs/oracle-harness/spec.md`.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate compare-text-with-own-metrics --strict` passes.
