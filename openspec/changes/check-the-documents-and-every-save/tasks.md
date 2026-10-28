# Tasks

## 1. Every save

- [x] 1.1 `xarch`: a save is a file the run wrote (absent from or different from the game's directory); per-game `saves = N` with a failure below it; the report line gains the expected count; `xarch.example.toml`, `docs/development.md`. Verify: the three scenarios of "Every save of a comparison game is compared" as unit tests of `xtask`; set `saves` in `local/xarch.toml` for the games whose input saves, run `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` and record the per-game save counts in design.md.

## 2. Document test

- [x] 2.1 `xtask/tests/architecture.rs` with the three checks. Verify: the scenarios of "The architecture document matches the workspace" (the last two on documents given to the check function).

## 3. Documents

- [ ] 3.1 Every item under "Items" of design.md: check against the code, correct what still disagrees, note in design.md the items found already fixed. Verify: the document test of group 2 and `doc_links` pass; `cargo doc --workspace --no-deps` with the gate's rustdoc flags passes.
- [ ] 3.2 `CHANGELOG.md`. Verify: the document tests pass.

## 4. Checks

- [ ] 4.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical`, `openspec validate check-the-documents-and-every-save --strict` passes.
