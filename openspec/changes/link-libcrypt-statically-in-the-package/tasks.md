# Tasks

## 1. The switch

- [x] 1.1 `VKXP_STATIC_CRYPT` in the build script. Verify: scenario "Development build"; `cargo test -p vkxp-ruby-sys --locked` passes.

## 2. The package command

- [x] 2.1 The variable, the two checks and their tests in `cargo xtask dist`. Verify: scenarios "Needed libraries of the package" and "Static library missing"; `cargo test -p xtask --locked` passes.

## 3. Texts

- [x] 3.1 `THIRD-PARTY.md`, `docs/releasing.md`, `docs/building.md` and the notes in `release.yml`. Verify: none of them lists libcrypt as a run-time need of the package; `cargo test -p xtask --locked` (the link check) passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate link-libcrypt-statically-in-the-package --strict` passes.
