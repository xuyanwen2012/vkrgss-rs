# Tasks

## 1. A viewport disposed alone in the core

- [ ] 1.1 Before anything changes, keep `cargo test --locked -- --list` of `vkxp-core`, `vkxp-binding` and `vkxp-binding-rgss3` under `local/`. Check that `bind-the-values-of-rgss3` and `run-rgss3-games-in-a-guest-of-their-own` are merged on `main` (`rb::check_count_rgss3`, `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs` with `BOUND`, `Graphics.__reset__`); stop and report if not. Verify: the three lists are kept; `git log` names both changes.
- [ ] 1.2 `Scene::dispose_viewport_alone` and the documentation of the arena of elements in `crates/vkxp-core/src/scene.rs` (decision 4), with tests in `crates/vkxp-core/src/scene/tests.rs`. Verify: the scenarios of "Disposing a viewport without its elements" of `specs/core-scene/spec.md`; `cargo test -p vkxp-core --locked` passes with every name of the kept list.

## 2. The scene, viewports and their disposal in RGSS3

- [ ] 2.1 `runtime::rules` gives `Rules::Rgss3` for RGSS3 and `runtime::input_rules` keeps the input state of RGSS3 at `Rules::Rgss2`, used by `guest.rs` (decision 2); `Viewport#initialize` of RGSS3 (decision 3); `element::dispose_viewport` by runtime (decision 4); `Plane#viewport` and `viewport=` of RGSS3 (decision 5); the documentation of `element.rs` and of `lib.rs` where it says what disposing a viewport does. Verify: `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists.
- [ ] 2.2 `crates/vkxp-binding-rgss3/tests/scene_rgss3.rs` with the scenarios of "Viewport disposal" for RGSS3, "The scene of a game of RGSS3", "Viewport of RGSS3" and "Plane viewport of RGSS3"; the scenario "XP and VX" of "The scene classes of RGSS1 and RGSS2 are unchanged by RGSS3" as one new test in `crates/vkxp-binding/tests/scene.rs` and one in `tests/scene_rgss2.rs`. Verify: those scenarios; the same test command.

## 3. Constructors, object arguments and the method surface of the scene classes

- [ ] 3.1 `element::viewport_arg_rgss3` for `Sprite.new`, `Plane.new` and `Tilemap.new` of RGSS3, and the wording `can't convert <class> into <class>` of the object arguments of `Sprite`, `Plane` and `Viewport` in RGSS3 (decision 6). Verify: the scenarios of "Constructors and object arguments of RGSS3", the measured ones and "Plane writers"; the same test command.
- [ ] 3.2 `Sprite`, `Plane`, `Viewport`, `Tilemap` and `Tilemap::BitmapArray` added to `BOUND` of `arity_rgss3.rs` (decision 11), nothing else of that file or of `tests/data/rgss3_api.txt` edited. Verify: the scenarios of "Method surface of the scene classes in RGSS3" without `Graphics`; the same test command; `git diff` of `arity_rgss3.rs` against the start of the change touches the list `BOUND` alone.

## 4. Graphics of RGSS3

- [ ] 4.1 `_reset` private in RGSS3 and `Graphics.send(:_reset)` in `testing::host::run_game` (decision 7); `play_movie` (decision 9); `freeze`, `snap_to_bitmap` and `transition` of RGSS3 (decision 8); `Graphics` added to `BOUND`. Verify: `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists.
- [ ] 4.2 `crates/vkxp-binding-rgss3/tests/graphics_rgss3.rs` with the scenarios of `specs/rgss-modules/spec.md` but "XP and VX", which is a new test in `crates/vkxp-binding/tests/scene_rgss2.rs` (decision 12). Verify: the scenarios of "Method surface of Graphics in RGSS3", "Freeze and snapshot of RGSS3", "Transition of RGSS3", "Movies of RGSS3" and "Graphics values of RGSS3"; the scenario "Every listed method exists" of "Method surface of the scene classes in RGSS3"; the same test command.
- [ ] 4.3 What did not change. Verify: the scenarios "Names of the tests" and "XP and VX" of "The scene classes of RGSS1 and RGSS2 are unchanged by RGSS3"; `git diff` against the start of the change is empty for `crates/vkxp-binding-rgss3/tests/ruby192.allow` (unless the mechanism reported an entry that now passes, which is then removed with its marker), and in the existing test files of `crates/vkxp-binding/tests` it adds lines and removes none; `git diff --stat` names under `crates` only the files of the Impact of `proposal.md`.

## 5. Documents

- [ ] 5.1 `docs/compatibility.md`: the subsection on sprites, planes, viewports and `Graphics` of VX Ace, the bullet of the tilemap of VX Ace, the sentence of VX (decision 13); `docs/decisions.md`, section "VX Ace": decisions 4 and 9. Verify: the scenarios of `specs/supported-runtimes/spec.md`; the text names nothing that the code does not do; `git diff --stat` names under `docs/` only these two files; `cargo test -p xtask --locked` passes.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate bind-the-scene-classes-of-rgss3 --strict` passes.
