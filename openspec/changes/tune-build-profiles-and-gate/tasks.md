# Tasks

## 1. Profiles (root `Cargo.toml`)

- [x] 1.1 Measure the gate once on the unchanged tree with a warm build directory and note the total and the test step's time under "Measured" in `design.md`. Verify: the note is there.
- [x] 1.2 Add the `dev`, `release` and `profiling` profiles. Verify: the scenarios of "Build profiles" in `specs/workspace-structure/spec.md`; `cargo test --workspace --locked` passes with `VK_LOADER_DRIVERS_SELECT='*lvp*'`.

## 2. Gate (`xtask`)

- [x] 2.1 Add the documentation step, `--locked`, and the durations; fix what the documentation step reports. Verify: the scenarios of "Single merge gate"; unit tests in `xtask` for the step list and the report.

## 3. Lints (root `Cargo.toml`, crates as reported)

- [x] 3.1 Add the three lints and fix the reports. Verify: the scenario of "Workspace lints"; `cargo clippy --workspace --all-targets --locked -- -D warnings` is clean.

## 4. Documents

- [x] 4.1 Correct `docs/decisions.md`, `README.md`, and the gate line of `AGENTS.md` and `CLAUDE.md` as `design.md` says. Verify: `diff AGENTS.md CLAUDE.md` prints nothing; the README's gate table matches `xtask/src/gate.rs`.

## 5. Integration check

- [x] 5.1 Run the full gate and note its time under "Measured". Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate tune-build-profiles-and-gate --strict` passes.
