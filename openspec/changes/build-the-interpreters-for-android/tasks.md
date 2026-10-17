# Tasks

## 1. Ruby 1.8.7

- [x] 1.1 Header, build script, io.c patch, crypt, ITIMER_VIRTUAL, zlib, errno of `Math`. Verify: `cargo build -p vkrgss-ruby-sys --target aarch64-linux-android` with the environment of `env34.sh` succeeds; the tests of the crate pass on Linux.

## 2. Ruby 1.9.2

- [x] 2.1 The same for `vkrgss-ruby192-sys`, the target lists, the guest programs link. Verify: `cargo build -p vkrgss-guest-rgss12 -p vkrgss-guest-rgss3 --target aarch64-linux-android` succeeds; the tests of the crate pass on Linux.

## 3. Device runs

- [ ] 3.1 The adb runner and the test changes for paths. Verify: the scenarios of `specs/supported-platforms/spec.md` on both phones (or recorded in `design.md` for the merge if adb is not reachable).

## 4. Documents

- [ ] 4.1 `docs/building.md` (Android), `docs/decisions.md` (API 34, decided 2026-10-17), `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate build-the-interpreters-for-android --strict` passes.
