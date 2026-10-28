# Tasks

## 1. Pin the messages (`vkrgss-host`)

- [x] 1.1 Add tests that pin the `MissingColon` and `InvalidFrame` messages and results of both parsers, including blanks, empty entries, a trailing comma and a frame beyond `u64`. Verify: `cargo test -p vkrgss-host script states` passes before the change.

## 2. One reader (`vkrgss-host`)

- [x] 2.1 Add the shared reader of entries and make `InputScript::parse` and `StateSchedule::parse` use it, as `design.md` says. Verify: the scenarios of `specs/host-session/spec.md`; the tests of task 1 and all earlier tests of both modules pass unchanged; `cargo clippy -p vkrgss-host --all-targets -- -D warnings` is clean.

## 3. Integration check

- [x] 3.1 Run the full gate and the cross-architecture check. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` reports every game identical, and `openspec validate parse-frame-schedules-once --strict` passes.
