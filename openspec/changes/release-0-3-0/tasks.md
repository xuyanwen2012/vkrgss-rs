# Tasks

## 1. Program

- [x] 1.1 Remove the warning of an experimental runtime for RGSS3 and the word in the usage message; the tests of the scenarios of `specs/game-launch/spec.md`. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp -p vkxp-guest-rgss3 --locked` passes.

## 2. Documents, banner and version

- [x] 2.1 `docs/decisions.md`: the rows A1 to A3 "Done" with the record of `design.md`, and the sentence that ends the experimental state. Verify: the scenario "Rows".
- [x] 2.2 README (first paragraph, table, banner file and alternative text), `AGENTS.md`, `docs/compatibility.md`, `docs/running.md`. Verify: the scenarios "Documents", "Banner" and "Length"; `grep -rn -i 'experimental' README.md AGENTS.md docs/compatibility.md docs/running.md crates/vkxp/src` finds nothing about VX Ace.
- [ ] 2.3 Version 0.3.0 in `Cargo.toml` and `Cargo.lock`; the section 0.3.0 of `CHANGELOG.md`. Verify: the scenarios "Version and lock file" and "Changelog".

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate release-0-3-0 --strict` passes.
