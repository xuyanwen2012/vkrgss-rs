# Tasks

## 1. Fonts (`xtask`)

- [ ] 1.1 Add `--font`, the family-name reader, the copies and the registry entries. Verify: tests for the three scenarios of `specs/oracle-harness/spec.md`, with a minimal font file built in the test for the family name.
- [ ] 1.2 Describe the option in `README.md`. Verify: the text matches the implemented option.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate share-oracle-fonts --strict` passes.
