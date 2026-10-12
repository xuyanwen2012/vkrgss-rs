# Tasks

## 1. Tables

- [x] 1.1 `build.rs` generates both tables with their largest counters; the C side selects one before start-up; `Options::symbols`. Verify: the existing tests of the symbol numbers pass unchanged; `cargo test -p vkxp-ruby-sys --locked` passes.
- [x] 1.2 The tests of RGSS2 in a test file of their own. Verify: the scenarios of RGSS2 in `specs/embedded-ruby/spec.md`.

## 2. Record

- [x] 2.1 The two patch files of the symbol numbers regenerated, the hashes of `PATCHES.md` and of the test of the vendored source updated, the section on shared patches, `README.vkxp.md`. Verify: the test of "Vendored source matches upstream" passes.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate number-symbols-by-runtime --strict` passes.
