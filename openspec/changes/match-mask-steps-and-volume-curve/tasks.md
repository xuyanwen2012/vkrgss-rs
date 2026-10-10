# Tasks

## 1. Transition steps (`vkxp-host`, `vkxp-binding`)

- [ ] 1.1 Compute the mask threshold as measured and show the held frame alone at step 0. Verify: the scenarios of `specs/host-session/spec.md` and the table of T in `design.md` (durations 2, 3, 5, 8, 20 and the first ten of 50) as tests; `cargo test -p vkxp-host --locked` passes with `VK_LOADER_DRIVERS_SELECT='*lvp*'`.
- [ ] 1.2 Default vagueness 20. Verify: the scenario "Default vagueness"; `cargo test -p vkxp-binding --locked` passes.

## 2. Volume curve (`vkxp-audio`)

- [ ] 2.1 Turn volume and fade into the gain of `design.md`. Verify: the scenarios of `specs/audio-playback/spec.md` and the gain table of `design.md` as tests; `cargo test -p vkxp-audio --locked` passes; the note at the mixer no longer says `PLACEHOLDER`.

## 3. Documentation

- [ ] 3.1 Correct what `README.md` and the crates' documentation say about the transition threshold, the default vagueness and the volume. Verify: `grep -rn "vague = 40\|255 \* s - 1" README.md crates --include=*.rs --include=*.md` prints nothing that describes current behaviour.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-mask-steps-and-volume-curve --strict` passes.
