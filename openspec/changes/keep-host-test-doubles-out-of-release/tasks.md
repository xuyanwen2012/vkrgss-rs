# Tasks

## 1. Feature (`vkxp-host`)

- [x] 1.1 Put the two modules behind the feature and make tests and examples enable it. Verify: the scenarios of `specs/workspace-structure/spec.md`.

## 2. Documents

- [ ] 2.1 Adjust the README where it runs the demo examples, if the command changes. Verify: the commands in the README run as written.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate keep-host-test-doubles-out-of-release --strict` passes.
