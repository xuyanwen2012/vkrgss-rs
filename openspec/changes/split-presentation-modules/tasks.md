# Tasks

## 1. Tests out of line

- [x] 1.1 Move the `mod tests` of `compositor/present.rs`, `device.rs` and `compositor.rs` into files of their own. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes with the same number of tests as before the move (state both numbers in the commit message).

## 2. Presenter

- [ ] 2.1 Split `compositor/present.rs` into the modules of `design.md`. Verify: the test command of 1.1 passes; `git diff --stat` of the commit shows no file outside `crates/vkxp-vk/src`.

## 3. Compositor

- [ ] 3.1 Split `compositor.rs` by its sections. Verify: the test command of 1.1 passes; scenario "File lengths" (`find crates/vkxp-vk/src -name '*.rs' | xargs wc -l`, each file over 1000 lines holds only tests).

## 4. Probes

- [ ] 4.1 Gather the test-only fields, records and methods in one probe type per module as in `design.md`. Verify: scenario "Test attributes" with `grep -rn '#\[cfg(test)\]' crates/vkxp-vk/src/compositor.rs crates/vkxp-vk/src/compositor`; the test command of 1.1 passes; `cargo build -p vkxp --release --locked` succeeds.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate split-presentation-modules --strict` passes; scenario "Nothing else changes".
