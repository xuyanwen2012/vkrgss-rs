# Tasks

## 1. Key map and gamepad (`vkxp-host`)

- [ ] 1.1 Make the key map a value built from defaults and per-button overrides with name parsing. Verify: tests for the two scenarios of the key-map requirement in `specs/host-session/spec.md`; the default-key-map tests still pass.
- [ ] 1.2 Add the gamepad mapping over an abstract pad state and the `gilrs` layer. Verify: tests for the four gamepad scenarios, none of which opens a device.

## 2. Configuration (`vkxp`)

- [ ] 2.1 Add the configuration structure, its layering with the options and `--config` and `--no-config`. Verify: tests for the scenarios of `specs/game-launch/spec.md`.
- [ ] 2.2 Document the file in `README.md` with a complete example. Verify: a test parses the README's example with the real loader.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-configuration-and-gamepad --strict` passes.
