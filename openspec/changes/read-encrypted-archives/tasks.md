# Tasks

## 1. Archive (`vkxp-vfs`)

- [x] 1.1 Implement the reader of the entry table and of entries. Verify: unit tests that an archive built by the test helper yields the names and bytes put in, including a name with non-ASCII characters and data whose length is not a multiple of four, and the two damaged-archive scenarios of `specs/game-files/spec.md`.
- [x] 1.2 Make the archive the first root of resolution. Verify: tests for the four scenarios of the encrypted-archive requirement; the existing tests still pass.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate read-encrypted-archives --strict` passes.
