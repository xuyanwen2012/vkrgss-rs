# Tasks

## 1. Interpreter

- [ ] 1.1 `recv` at a name error of method dispatch; `__member__` in `struct.c`; the patch log and chain. Verify: the scenarios of `specs/embedded-ruby/spec.md`; `cargo test -p vkxp-ruby-sys --locked` passes.

## 2. Binding

- [ ] 2.1 `owner` and `observer` at the measured triggers; the six names of `Win32API`; the names of the tilemap. Verify: the scenarios "First uses" and "Tilemap" of `specs/rgss-runtime/spec.md`; `cargo test -p vkxp-binding --locked` passes.
- [ ] 2.2 The test of the measured steps. Verify: the scenario "Every measured step"; the same command.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate intern-the-lazy-names-of-rgss2 --strict` passes.
