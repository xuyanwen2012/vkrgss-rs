# Tasks

## 1. Archive and probe (`xtask`)

- [x] 1.1 Implement the script archive reader and writer. Verify: the archive round-trip scenario of `specs/oracle-harness/spec.md` and a test that vkxp runs a game whose archive the writer produced.
- [x] 1.2 Implement the probe template and its generation for both sides, and the copy of the game directory. Verify: the source-untouched scenario and the scheduled-input scenario, the latter by running the probe in vkxp on a generated game.

## 2. Runs and capture (`xtask`)

- [x] 2.1 Implement the vkxp side: build the options, run headless, collect the dumps. Verify: the capture-in-vkxp scenario.
- [x] 2.2 Implement the original side: prefix preparation, Xvfb and Wine start and shutdown, request polling, window capture with `x11rb`, timeout. Verify: the capture-in-the-original scenario where the prerequisites exist, otherwise as `design.md` states; the missing-prerequisites scenario.

## 3. Report (`xtask`)

- [ ] 3.1 Implement the comparison, the difference pictures and the summary. Verify: the two report scenarios.
- [ ] 3.2 Add the `oracle` command with its usage text and a README section. Verify: every command in the section matches the implemented options.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-oracle-harness --strict` passes.
