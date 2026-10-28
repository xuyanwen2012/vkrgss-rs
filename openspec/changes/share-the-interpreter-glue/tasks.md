# Tasks

## 1. Glue crate

- [x] 1.1 Create `vkrgss-ruby-glue` with the `Interpreter` trait; move `root.rs` and `host.rs` into it; implement the trait in both sys crates; re-export under the present paths; add the crate to the workspace, the deps rules of `xtask` (guest side) and `deny.toml` if needed. Verify: `cargo test -p vkrgss-ruby-sys -p vkrgss-ruby192-sys -p vkrgss-binding -p vkrgss-binding-rgss3` passes unchanged; the scenarios of "Interpreter glue in one place".
- [x] 1.2 Move the shared parts of `protect.rs` and `call.rs`; record in design.md what stays per interpreter and why. Verify: the same tests, the layout tests of both sys crates, `cargo xtask gate --quick` for each of the four crates.

## 2. Jumps and bignums

- [ ] 2.1 L1 and L2 in the shared `protect` and trampoline: restore only nil or an exception through `rb_set_errinfo`, resume `TAG_FATAL` with `rb_jump_tag`. Verify: the scenarios of "Jumps that are not exceptions cross host functions" for Ruby 1.8.7 and of the 1.9.2 requirement, as tests of the sys crates.
- [ ] 2.2 `rb_ll2inum` in `graphics.rs` (`frame_count`) and `audio.rs` (byte counts). Verify: a binding test reads `Graphics.frame_count` set to 2^40 back as an equal `Bignum` on both runtimes.

## 3. Documents and checks

- [ ] 3.1 `docs/architecture.md` (crate table, unsafe list, the paragraph on the two sys crates), `docs/development.md`, `CHANGELOG.md`. Verify: the document tests pass.
- [ ] 3.2 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical`, `openspec validate share-the-interpreter-glue --strict` passes.
