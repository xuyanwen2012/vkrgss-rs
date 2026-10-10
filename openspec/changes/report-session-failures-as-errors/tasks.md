# Tasks

## 1. Reporter (`vkxp-host`, `vkxp`)

- [x] 1.1 Add the reporter, give each report its severity, and pass the event reporter from the executable. Verify: the scenarios of `specs/host-session/spec.md`; the classification of the headless run's reports is under "Checked" in `design.md`; `cargo test -p vkxp-host -p vkxp --locked` passes with `VK_LOADER_DRIVERS_SELECT='*lvp*'`.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate report-session-failures-as-errors --strict` passes.
