# Tasks

## 1. Constants and arguments of Input of RGSS3

- [x] 1.1 Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding` and of `vkxp-binding-rgss3` under `local/`. Check that `bind-the-scene-classes-of-rgss3` is merged on `main` (`runtime::input_rules`, `Graphics` in `BOUND` of `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs`); stop and report if not. Verify: both lists are kept; `git log` names the change.
- [x] 1.2 `input::define` by runtime, the constants of RGSS3 and `button_arg_rgss3` for `press?`, `trigger?` and `repeat?` (decisions 1 to 3); `crates/vkxp-binding-rgss3/tests/input_rgss3.rs` with the scenarios of "Arguments of Input of RGSS3", "Button constants of RGSS3, measured on the original" and "Functions and constants, measured on the original"; `Input` added to `BOUND` (decision 8). Verify: those scenarios and "Bound"; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists.

## 2. When a key is seen

- [x] 2.1 The input state of RGSS3 with `Rules::Rgss3`, `runtime::input_rules` removed (decision 4), and `InputState::frame_shown` in `graphics::frame` (decision 5); the documentation of `input.rs`, `graphics.rs` and of `lib.rs` where it describes `Input`. Verify: the scenarios of "When Input of RGSS3 sees a key" and "Repeat and directions of Input of RGSS3" in `input_rgss3.rs`; the same test command.
- [x] 2.2 What did not change. Verify: the scenarios of "Input of RGSS1 and RGSS2 is unchanged by RGSS3", "XP and VX" being one new test in `crates/vkxp-binding/tests/graphics_rgss2.rs`; the tests of `Input` in `crates/vkxp-binding/tests/modules.rs` pass unedited over both interpreters; `git diff` against the start of the change is empty for `crates/vkxp-binding-rgss3/tests/ruby192.allow` (unless the mechanism reported an entry that now passes, which is then removed with its marker) and for `crates/vkxp-core`, and in the existing test files it adds lines and removes none; `git diff` of `arity_rgss3.rs` touches the list `BOUND` alone.

## 3. Documents

- [x] 3.1 `docs/compatibility.md`: the subsection on `Input` of VX Ace (decision 10). Verify: the scenarios of `specs/supported-runtimes/spec.md`; the text names nothing that the code does not do; `git diff --stat` names under `docs/` only `compatibility.md`; `cargo test -p xtask --locked` passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate bind-input-of-rgss3 --strict` passes.
