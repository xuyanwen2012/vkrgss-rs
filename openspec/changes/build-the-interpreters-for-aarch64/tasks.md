# Tasks

## 1. Headers

- [x] 1.1 The aarch64 headers of both crates and the tests that compare them with x86_64. Verify: scenarios "Header of aarch64"; `cargo test -p vkrgss-ruby-sys -p vkrgss-ruby192-sys --locked` passes.

## 2. Targets

- [ ] 2.1 aarch64 in the target lists of both interpreter crates and the three dependent build scripts. Verify: scenario "Another target" (a test of the list, or of the function that decides, with `riscv64gc-unknown-linux-gnu`); the gate passes on x86_64; `design.md` says that aarch64 itself is verified after the merge.

## 3. Documents

- [ ] 3.1 `docs/compatibility.md`, `docs/building.md`, `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents and their links pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate build-the-interpreters-for-aarch64 --strict` passes.
