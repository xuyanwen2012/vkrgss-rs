# Tasks

## 1. Subscriber and options (`vkxp`)

- [x] 1.1 Add `tracing` and `tracing-subscriber`, install the subscriber, add `--verbose` and `VKXP_LOG`. Verify: the scenarios "Default level", "Level from the environment" and "Malformed filter" of `specs/game-launch/spec.md` as tests of the executable; the option parser's unit tests.

## 2. Events (`vkxp`, `vkxp-host`, `vkxp-audio`, `vkxp-binding`)

- [x] 2.1 Turn the run-time diagnostics into events with the levels of `design.md`, and pass the line adapter where the executable passed the error stream. Verify: no `eprintln!` remains in the `src` directories of the four crates except for the two kinds `design.md` keeps direct; the existing tests of the four crates pass unchanged; the scenario "Script error at every level".
- [x] 2.2 Add the informational events the proposal lists. Verify: the scenario "Verbose".

## 3. Documents

- [ ] 3.1 Describe `--verbose` and `VKXP_LOG` in `README.md` and in the usage text. Verify: the usage text test.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate log-through-tracing --strict` passes.
