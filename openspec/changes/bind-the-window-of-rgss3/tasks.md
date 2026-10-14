# Tasks

## 1. The window of RGSS3 as a class

- [ ] 1.1 Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding` and of `vkxp-binding-rgss3` under `local/`. The third branch of `window::define` and `Window.new` of RGSS3 (decisions 1 and 2), with `openness`, `viewport=` and no `stretch`; `crates/vkxp-binding-rgss3/tests/window_rgss3.rs` with the scene support of `vkxp-binding`; the test of RGSS1 in `crates/vkxp-binding/tests/scene.rs` and that of RGSS2 in `tests/scene_rgss2.rs` for the scenarios "XP and VX" (decision 9). Verify: the scenarios of "Window construction in RGSS3" but the readers `padding`, `padding_bottom`, `arrows_visible`, `tone`, `open?` and `close?`, which come with tasks 1.2 and 2.1; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists.
- [ ] 1.2 `padding`, `padding_bottom`, `arrows_visible`, `move`, `open?`, `close?` (decisions 1 and 3). Verify: the scenarios of "Window methods added in RGSS3" and, of "Window construction in RGSS3", everything but `tone`; the same test command.

## 2. Tone and the calls of the scene

- [ ] 2.1 `Owner::WindowTone`, the window's `Tone` object, `tone` and `tone=` (decision 4). Verify: the scenarios of "Window tone in RGSS3" and all of "Window construction in RGSS3"; the same test command.
- [ ] 2.2 Tests of the assignments as calls (decision 3); no code is expected beyond what the tasks before made. Verify: the scenarios of "Assignments of a window in RGSS3 are calls of the scene"; the same test command; `git diff --stat` against the start of the change names no file of `crates/vkxp-core`.

## 3. Disposal, faults and the method list

- [ ] 3.1 Tests of disposal, of disposed bitmaps and of the calls at which the original faults (decisions 5 and 6). Verify: the scenarios of "Disposal and disposed bitmaps of a window in RGSS3" and "Calls at which the original of RGSS3 faults, through the class"; the same test command.
- [ ] 3.2 `Window` in the list of bound classes of `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs` (decision 8). Verify: the scenarios of "Method surface of Window in RGSS3" and of "The window of RGSS1 and RGSS2 is unchanged by RGSS3"; `git diff` against the start of the change is empty for `crates/vkxp-binding/tests/data/rgss3_api.txt` and for `crates/vkxp-binding-rgss3/tests/ruby192.allow`, and in `tests/scene.rs` and `tests/scene_rgss2.rs` of `vkxp-binding` it adds lines and removes none; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 -p xtask --locked` passes.

## 4. Documents

- [ ] 4.1 `docs/compatibility.md`: the subsection on the window of VX Ace (decision 10); the documentation of `window.rs`, `wrap.rs` and of `lib.rs` where it describes the window. Verify: the scenarios of `specs/supported-runtimes/spec.md`; the text names nothing that the code does not do; `cargo test -p xtask --locked` passes.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate bind-the-window-of-rgss3 --strict` passes.
