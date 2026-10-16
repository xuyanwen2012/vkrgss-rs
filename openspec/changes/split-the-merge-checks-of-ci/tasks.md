# Tasks

## 1. Parts of the gate

- [x] 1.1 Measure the test binaries, define the parts, `--job`, and the test of the union. Verify: scenarios "Union of the parts" and "Unknown part"; each part runs and passes locally with `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate --job <name>`; record the measured times of each part in `design.md`.

## 2. Workflows

- [x] 2.1 The jobs of `ci.yml` with sccache, the issue job of `nightly.yml`, and their tests. Verify: scenarios "One job per part" and "Nightly failure"; the scenarios of "Workflows", "Pinned inputs of the workflows" and "aarch64 job" still hold; `cargo test -p xtask --locked` passes.

## 3. Documentation

- [x] 3.1 `docs/development.md` describes the parts and the CI jobs; `CHANGELOG.md` under Unreleased. Verify: the tests of `xtask` that check the documents and their links pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate split-the-merge-checks-of-ci --strict` passes.
