# Tasks

## 1. Archive (`xtask`)

- [x] 1.1 Implement the reader and writer of the archive format and `pack-rgssad`. Verify: the two scenarios of the packing requirement in `specs/oracle-harness/spec.md`, and a test that `vkxp` runs a generated game packed by the command.

## 2. Harness (`xtask`)

- [ ] 2.1 Add the probe inside the archive when a game has one. Verify: the two scenarios of the encrypted-archive requirement.
- [ ] 2.2 Describe both in `README.md`. Verify: the text matches the implemented commands.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate compare-packed-games --strict` passes.
