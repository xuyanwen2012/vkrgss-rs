# Tasks

## 1. Loading the loader

- [ ] 1.1 Bundle-relative loading and the ICD variable on macOS, with today's fallback. Verify: the tests of `vkrgss-vk` pass on the Mac with the environment of `docs/building.md`; a unit test of the path logic.

## 2. The app

- [ ] 2.1 `cargo xtask app`: executables, Frameworks, ICD, install names, Info.plist, licences, ad-hoc signature, zip. Verify: scenarios "No Homebrew needed" and "Headless frame from the bundle".

## 3. Asking for a game

- [ ] 3.1 The folder chooser and the remembered directory. Verify: scenario "No game directory" (simulate the cancel by an `osascript` that returns the cancel error, e.g. through a test seam).

## 4. Documents

- [ ] 4.1 `docs/building.md`, `docs/running.md` (macOS), `THIRD-PARTY.md`, `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass on the Mac.

## 5. Checks

- [ ] 5.1 On the Mac: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` with the environment of `docs/building.md`. The Linux gate is run at the merge by the owner's session.
