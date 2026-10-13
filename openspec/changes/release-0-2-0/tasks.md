# Tasks

## 1. Program

- [x] 1.1 Remove the warning of an experimental runtime, its doc comment and the word in the usage message; replace its tests by one of the scenario "VX". Verify: the scenarios of `specs/game-launch/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.

## 2. Documents and version

- [x] 2.1 `README.md`, `AGENTS.md`, `docs/compatibility.md`, `docs/running.md`: XP and VX supported, VX Ace planned. Verify: the scenario "Documents after V3"; `grep -rn -i experimental README.md AGENTS.md docs crates/vkxp/src` finds no statement about VX.
- [x] 2.2 `docs/decisions.md`: the frame times under their risk, and the sentence that ends "Before VX is done". Verify: the scenario "The record is there".
- [x] 2.3 Version 0.2.0 in `Cargo.toml` and `Cargo.lock`; the section of `CHANGELOG.md`. Verify: the scenario "Version and lock file".

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate release-0-2-0 --strict` passes.
