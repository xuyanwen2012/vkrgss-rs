# Tasks

## 1. Interrupt

- [x] 1.1 Keep Ruby's handler of SIGINT in script mode and deliver it to the interpreter, for Ruby 1.8.7 and 1.9.2. Verify: the scenarios of the requirement "Interrupt of a script" as integration tests for `rgss1` and `rgss3`; the signal tests of the interpreter crates pass unchanged.

## 2. Documents

- [x] 2.1 `docs/running.md` (script mode), `CHANGELOG.md`; the follow-up "Interrupt" of the archived `run-a-ruby-script` stays as it is (archived). Verify: the tests that check the documents pass.

## 3. Checks

- [x] 3.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical`, `openspec validate interrupt-a-script --strict` passes.
