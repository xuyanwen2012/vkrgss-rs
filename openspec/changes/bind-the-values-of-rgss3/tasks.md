# Tasks

## 1. Colour and tone

- [ ] 1.1 Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding` and of `vkxp-binding-rgss3` under `local/`. Hand the runtime to `color::define_color`, `color::define_tone`, `rect::define` and `table::define` (decision 1), with nothing registered differently yet. Verify: `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists.
- [ ] 1.2 `rb::check_count_rgss3` (decision 5) and the functions of RGSS3 for `initialize` and `set` of `Color` and `Tone` (decisions 2 and 4); `crates/vkxp-binding-rgss3/tests/values_rgss3.rs` with their scenarios; the test of RGSS1 in `crates/vkxp-binding/tests/ruby.rs` and that of RGSS2 in `tests/scene_rgss2.rs` for the scenarios "XP and VX" (decision 13). Verify: the scenarios of "Color and Tone of RGSS3", and "Color and Tone, measured on the original", the first three calls of "Numbers of the wrong kind" and "A method of fixed arity" of "Argument errors of the value classes in RGSS3"; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes.

## 2. Rectangle

- [ ] 2.1 The functions of RGSS3 for `initialize` and `set` of `Rect`, both with arity -1 (decisions 3 and 4). Verify: the scenarios of "Rect of RGSS3", "Seen by the element" among them with the scene support of `vkxp-binding`, and "Rect, measured on the original", "Out of range" and the calls on `Rect` of "Numbers of the wrong kind"; the same test command.

## 3. Table, comparison, serialized and text forms

- [ ] 3.1 The count message of `Table#[]` and `Table#[]=` in RGSS3 (decision 5) and the tests of the table. Verify: the scenarios of "Table of RGSS3", and "Table, measured on the original" and the calls on `Table` of "Numbers of the wrong kind"; the same test command.
- [ ] 3.2 `_load` of `Color`, `Tone` and `Rect` in RGSS3 (decision 8). Verify: the scenarios of "Serialized forms of the value classes in RGSS3"; the same test command.
- [ ] 3.3 Tests of what needs no code (decisions 7 and 9). Verify: the scenarios of "Comparison of the value classes in RGSS3" and "Text forms of the value classes in RGSS3"; the same test command; `git diff --stat` against the start of the change names, under `crates`, files of `crates/vkxp-binding/src`, `crates/vkxp-binding-rgss3/tests/values_rgss3.rs` and the two shared test files of decision 13 only.

## 4. Method list of RGSS3

- [ ] 4.1 `crates/vkxp-binding/tests/data/rgss3_api.txt`, whole, and `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs` with the list of bound classes (decision 12). Verify: the scenarios of "Method surface of the value classes in RGSS3"; the file holds lines of the form `Class#method arity` or `Class.method arity` and nothing else; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 -p xtask --locked` passes.
- [ ] 4.2 What did not change. Verify: the scenarios of "The value classes of RGSS1 and RGSS2 are unchanged by RGSS3"; `git diff` against the start of the change is empty for `crates/vkxp-binding-rgss3/tests/ruby192.allow`, and in `tests/ruby.rs` and `tests/scene_rgss2.rs` of `vkxp-binding` it adds lines and removes none.

## 5. Documents

- [ ] 5.1 `docs/compatibility.md`: the subsection on the value classes of VX Ace (decision 14); the documentation of `color.rs`, `rect.rs`, `table.rs`, `rb.rs` and of `lib.rs` where it describes these classes. Verify: the scenarios of `specs/supported-runtimes/spec.md`; the text names nothing that the code does not do; `git diff --stat` against the start of the change names no file under `docs/` but `compatibility.md`; `cargo test -p xtask --locked` passes.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate bind-the-values-of-rgss3 --strict` passes.
