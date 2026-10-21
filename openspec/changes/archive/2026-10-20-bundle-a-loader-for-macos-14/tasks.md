# Tasks

## 1. Loader

- [x] 1.1 `cargo xtask macos-loader` with the pinned tag and commits. Verify: it builds `libvulkan.1.dylib` whose `otool -l` shows `minos 14.0`.

## 2. App

- [x] 2.1 `cargo xtask app` uses it and checks every Mach-O file. Verify: scenarios "Minimum of every file" and "A file built for a newer macOS" (a unit test with a crafted header or Homebrew's loader); the headless frame of the bundle equals the development build's.

## 3. Documents

- [x] 3.1 `docs/building.md`, `THIRD-PARTY.md`, `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass on the Mac.

## 4. Checks

- [x] 4.1 On the Mac: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test -p xtask --locked`. The Linux gate is run at the merge by the owner's session.
