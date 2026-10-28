# Tasks

## 1. Split `runtime.rs` (`vkrgss-binding`)

- [x] 1.1 Move `runtime.rs` into the modules `design.md` names. Verify: `cargo test -p vkrgss-binding -p vkrgss-binding-rgss3` passes with no test removed; every crate that uses the binding compiles unchanged.

## 2. Split `win32api.rs` and `font.rs` (`vkrgss-binding`)

- [x] 2.1 Move `win32api.rs` and `font.rs` into the modules `design.md` names, their tests into files of their own. Verify: `cargo test -p vkrgss-binding -p vkrgss-binding-rgss3` passes with no test removed; the "File lengths" scenario of `specs/binding-over-two-interpreters/spec.md`.

## 3. Prelude line numbers (`vkrgss-binding`)

- [x] 3.1 Find every reader of positions inside the preludes and record them with evidence under "As built" in `design.md`. Verify: the section names each reader or the search that found none.
- [x] 3.2 Keep the padding with the reason written next to it, or replace it by a starting line, or remove it, as `design.md` decides from 3.1. Verify: the "Prelude positions" scenario; `cargo test -p vkrgss-binding -p vkrgss-binding-rgss3` passes.

## 4. Integration check

- [ ] 4.1 Run the full gate and the cross-architecture check. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` reports every game identical, and `openspec validate split-the-large-binding-modules --strict` passes.
