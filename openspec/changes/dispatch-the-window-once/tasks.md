# Tasks

## 1. The kind of a window (`vkrgss-core`)

- [ ] 1.1 Replace `Window::rgss2` and `rgss2::State::rgss3` by one kind enum as `design.md` says, and make windows of each runtime with it. Verify: `cargo test -p vkrgss-core` passes with no expectation changed; `grep -n "rgss2: Option\|rgss3: Option" crates/vkrgss-core/src` finds nothing for windows.

## 2. One dispatch (`vkrgss-core`)

- [ ] 2.1 Make `Window::call`, `refresh_caches`, the window assembly and the `rgss3.rs` accessors match the kind once instead of unwrapping options. Verify: the scenarios of `specs/core-scene/spec.md`; `cargo test -p vkrgss-core -p vkrgss-binding -p vkrgss-binding-rgss3` passes; `cargo clippy -p vkrgss-core --all-targets -- -D warnings` is clean.

## 3. Integration check

- [ ] 3.1 Run the full gate and the cross-architecture check. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` reports every game identical, and `openspec validate dispatch-the-window-once --strict` passes.
