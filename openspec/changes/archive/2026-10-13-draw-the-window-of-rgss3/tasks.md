# Tasks

## 1. Window of RGSS3 in the scene

- [x] 1.1 The third rule set: creation, defaults, the paddings, `arrows_visible`, the tone and their readers; the new calls, ignored by windows of RGSS1 and RGSS2; background and frame with the tone and the normal blend, open and opening. Verify: the scenarios of "Window of RGSS3" and "Background of a window of RGSS3"; `cargo test -p vkxp-core --locked` passes and `cargo check --workspace --all-targets --locked` succeeds.
- [x] 1.2 The cursor: its pieces, its position, and its placement as state of the calls. Verify: the scenarios of "Cursor of a window of RGSS3" and "Cursor placement of a window of RGSS3"; the same test command.
- [x] 1.3 Contents with the paddings and the calls that show them; the built parts and `move`. Verify: the scenarios of "Contents of a window of RGSS3" and "Cached parts and move of a window of RGSS3"; the same test command.
- [x] 1.4 Scroll arrows with their conditions and their calls, the pause sign, the order of the parts. Verify: the scenarios of "Scroll arrows of a window of RGSS3" and "Order of the parts of a window of RGSS3"; the same test command.
- [x] 1.5 The window without area, disposed skin and contents, the calls at which the original faults. Verify: the scenarios of "Window of RGSS3 without area", "Disposed skin and contents of a window of RGSS3" and "Calls at which the original of RGSS3 faults"; the same test command; `git diff --stat` against the start of the change names no file of `crates/vkxp-binding`.

## 2. Skins smaller than the layout

- [x] 2.1 The shortened target for the background of windows of RGSS2 and RGSS3; the two digests of `crates/vkxp-core/tests/data/window-rgss2-cases.txt` and the head of that file. Verify: the scenarios "Background, measured on the originals of VX and VX Ace", "Two pictures of RGSS2 corrected" and "Nothing else of RGSS2 changes" of "Skins smaller than the layout"; `git diff` of `crates/vkxp-core/tests/data/` shows two changed `frame` lines and changed comment lines in that file and no other change to a file that existed; `cargo test -p vkxp-core -p vkxp-binding --locked` and `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked windows` pass.
- [x] 2.2 In RGSS3 the shortened target for the centre and edges of the cursor, and the cut width of a part of the skin; RGSS2 keeps both as they are. Verify: the scenario "Cursor of RGSS3, measured on the original"; the tests of `crates/vkxp-core/src/window/rgss2/tests.rs` pass with no expected value edited beyond task 2.1; `cargo test -p vkxp-core --locked` passes.

## 3. Whole scenes

- [x] 3.1 The format of `test_support::window_cases` for scenes of RGSS3; `window-rgss3-cases.txt` and `window-rgss3-random.txt` with the digests of the model of the measurement; both compared in the CPU reference. Verify: the scenarios "Cases of the measurement", "Random scenes held out" and "Nothing of the original" of "Whole windows of RGSS3 as measured"; the script that drove the model is under `local/` and not committed; `cargo test -p vkxp-core --locked` passes.
- [x] 3.2 Both files composed by both backends. Verify: the scenario "Both backends"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-core -p vkxp-compose-ref -p vkxp-vk --locked` passes; `git diff --stat` against the start of the change names no file under `crates/vkxp-proto`, `crates/vkxp-compose-ref/src` or `crates/vkxp-vk/shaders`, and of `crates/vkxp-vk/src` only `tests/windows.rs`.

## 4. Documentation

- [x] 4.1 `docs/decisions.md` (section "VX Ace"), `docs/compatibility.md` (section "The window of VX"), the documentation of the modules touched. Verify: the scenarios of `specs/supported-runtimes/spec.md`; the text names nothing that the code does not do; `git diff --stat` against the start of the change names no file under `docs/` but `decisions.md` and `compatibility.md`.

## 5. Integration check

- [x] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-the-window-of-rgss3 --strict` passes.
