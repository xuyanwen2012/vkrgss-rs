# Tasks

## 1. Upgrades

- [x] 1.1 `sha2` 0.11. Verify: `cargo test -p vkxp-ruby-sys --locked` passes.
- [ ] 1.2 `x11rb` 0.14. Verify: `cargo test -p xtask --locked` and `cargo clippy -p xtask --all-targets --locked -- -D warnings` pass.
- [ ] 1.3 `symphonia` 0.6, by the rule of `design.md`. Verify: `cargo test -p vkxp-audio --locked` passes; the outcome is under "Checked" in `design.md`.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate upgrade-symphonia-sha2-x11rb --strict` passes.
