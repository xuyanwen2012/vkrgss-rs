# Tasks

## 1. Vulkan and host

- [x] 1.1 Portability, timestamp `WAIT`, headless default device per OS. Verify: scenario "Vulkan backend on the Apple GPU" on the Mac; the tests of `vkrgss-vk` and `vkrgss-host` pass on Linux.

## 2. Guest

- [x] 2.1 Landlock Linux-only, parent watch with kqueue on macOS, no confinement on macOS. Verify: the tests of `vkrgss-guest` and the guest-process tests of `vkrgss` pass on the Mac and on Linux.

## 3. Interpreters

- [x] 3.1 Darwin headers and build changes for both crates, the errno of `Math` in the shim, tests. Verify: scenarios "Interpreters" and "Errors of Math on macOS" on the Mac; the crates' tests pass on Linux.

## 4. Linux-dependent tests and documents

- [x] 4.1 The tests as `design.md` says; `docs/building.md` (macOS section), `docs/compatibility.md`, `docs/decisions.md` (the confinement decision), `CHANGELOG.md`. Verify: `cargo test --workspace` on the Mac with the environment of `design.md` passes; the tests of `xtask` that check the documents pass.

## 5. Integration check

- [ ] 5.1 Run the full gate on Linux. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate build-and-test-on-macos --strict` passes.
