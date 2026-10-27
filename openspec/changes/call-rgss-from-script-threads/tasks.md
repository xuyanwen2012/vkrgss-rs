# Tasks

## 1. Measurements on the originals

- [ ] 1.1 Run the script-thread probe on the original VX Ace runtime through `cargo xtask oracle`; record the table in design.md under "As built". Verify: the probe's output file of the original run is listed there with every call's result.
- [ ] 1.2 Run the tilemap-name probe on the original XP runtime through `cargo xtask oracle`; record the numbers and names in design.md. Verify: the output of the original run is listed there.

## 2. Guest state of the process

- [x] 2.1 Make the guest state, class handles and queues of collected objects one state of the process behind a cell that needs the interpreter's token; free functions queue where any thread's next access drains. Verify: `rgss_classes_work_from_a_script_thread`, `collection_on_a_script_thread_frees_bitmaps_and_elements`, and the existing tests of `vkrgss-binding` and `vkrgss-binding-rgss3` pass with `--test-threads=1` and with the default threads.
- [x] 2.2 Correct the safety comments at `Ruby::new()` in `host.rs` of both interpreter crates. Verify: the comment says the caller holds the interpreter; `cargo clippy` with the workspace lints passes.

## 3. Tilemap names of RGSS1

- [ ] 3.1 Intern for a tilemap of RGSS1 what the original interns, at its moments. Verify: `tilemap_names_of_rgss1_as_measured` and the existing symbol tests of RGSS1 pass.

## 4. Documents

- [ ] 4.1 `docs/decisions.md` (script threads), `docs/compatibility.md` (the measured table; the names of a tilemap of RGSS1), `CHANGELOG.md`. Verify: the tests that check the documents pass.

## 5. Checks

- [ ] 5.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical` (or, if task 3.1 changed what XP saves hold, the difference is the one the measurement predicts and is explained in design.md); `openspec validate call-rgss-from-script-threads --strict` passes.
