# Tasks

## 1. The option

- [x] 1.1 `--sandbox`, `VKXP_SANDBOX`, the key `sandbox`, the help text. Verify: scenario "Unknown value" and tests of the precedence beside those of `--guest`; `cargo test -p vkxp --locked --lib --bins` passes.

## 2. The restriction

- [x] 2.1 The `landlock` dependency and the rules of `design.md` in the guest's start, with the reports of the best-effort mode. Verify: scenarios "Write inside", "Write outside", "Off" and "States" as end-to-end tests that skip with a printed reason where Landlock cannot be used; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes; `cargo xtask deps` passes; "Checked" is filled.

## 3. Documentation

- [x] 3.1 `docs/running.md` (a section "Sandbox" beside "Guest process", the option in the table, the sentence that a sandbox does not exist yet goes), `docs/decisions.md` (the sentence that the sandbox is still to come names this change), `README.md` where it lists options. Verify: the tests of `xtask` that check the documents pass; no document says that the sandbox does not exist.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate confine-the-guest-process --strict` passes.
