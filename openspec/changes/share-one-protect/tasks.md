# Tasks

## 1. Export and use (`vkxp-ruby-sys`, `vkxp-binding`)

- [x] 1.1 Export `protect` with its contract and test it from an integration test. Verify: the first two scenarios of `specs/embedded-ruby/spec.md`; `cargo test -p vkxp-ruby-sys`.
- [x] 1.2 Use it in `perform` and remove the copy. Verify: the third scenario; `ffi::rb_protect` is no longer called in `vkxp-binding/src/rb.rs`; `cargo test -p vkxp-binding`.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate share-one-protect --strict` passes.
