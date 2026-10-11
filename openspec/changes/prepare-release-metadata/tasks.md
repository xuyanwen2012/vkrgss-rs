# Tasks

## 1. Metadata

- [x] 1.1 Workspace and crate metadata as in `design.md`, with a test in `xtask` beside the dependency rules that checks the scenario "Metadata of every crate" against `cargo metadata`. Verify: `cargo test -p xtask --locked` passes.

## 2. Version option

- [ ] 2.1 `--version` and `-V`. Verify: the scenarios of `specs/game-launch/spec.md` as tests; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.

## 3. Leftovers

- [ ] 3.1 The pass of `design.md`, with "Found" filled in. Verify: `cargo build --workspace --all-targets --locked` passes (or with an updated lock file committed if a dependency was removed).

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate prepare-release-metadata --strict` passes.
