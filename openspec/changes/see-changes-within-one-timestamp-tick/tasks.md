# Tasks

## 1. Listing cache

- [ ] 1.1 The racy rule in `dir.rs` and the timing-independent test. Verify: scenario "Change within one tick" fails before the fix and passes after; `cargo test -p vkrgss-vfs --locked` passes.

## 2. Documentation

- [ ] 2.1 `CHANGELOG.md` under Unreleased, Fixed. Verify: the tests of `xtask` that check the documents pass.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate see-changes-within-one-timestamp-tick --strict` passes.
