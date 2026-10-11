# Tasks

## 1. Package command

- [x] 1.1 `cargo xtask dist` as in `design.md`, with tests of the list of files and of the missing file. Verify: the scenarios of "Release package command"; the unpacked `vkxp --version` prints `vkxp 0.1.0`; `cargo test -p xtask --locked` passes.

## 2. Workflows and templates

- [x] 2.1 `.github/workflows/ci.yml` and `release.yml`. Verify: both parse as YAML; the scenarios of "Workflows".
- [x] 2.2 The issue and pull request templates. Verify: the files exist with the fields of `design.md`.

## 3. Documentation

- [x] 3.1 `docs/releasing.md`, linked from `docs/development.md` and the README's list of documents. Verify: `cargo test -p xtask --locked` (the link check) passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-ci-and-release-packaging --strict` passes.
