## Why

The standard scripts of RPG Maker VX Ace create `Viewport.new` without arguments in
`Scene_Base` before their first frame (fact 1 of the design of
`run-rgss3-games-in-a-guest-of-their-own`). In a game of RGSS3 that raises `ArgumentError`
today, because the classes `Sprite`, `Plane`, `Viewport` and the module `Graphics` are
those of RGSS2, and the scene of such a game is made with the rules of RGSS2
(`Runtime::Rgss3` maps to `Rules::Rgss2` in `runtime.rs`, the Follow-ups of the archived
`2026-10-13-draw-sprites-and-snapshots-as-rgss3`). What RGSS3 does differently was measured
on the original runtime on 2026-10-13 (`ace-v1-sprite-REPORT.md` sections 1 to 5 with
`ace-v1-sprite-explore-log.txt`, `ace-v1-window-REPORT.md` and `ace-v1-tilemap-REPORT.md`
on disposed viewports, and `RGSS301_api.md`, all outside the repository). The rules below
the binding exist in `vkxp-core`, `vkxp-compose-ref`, the protocol and the presenter; this
change hands the scripts of RGSS3 over to them. It is change 3 of the table "the changes
that follow" of `run-rgss3-games-in-a-guest-of-their-own` and on the path to milestone A1.

## What Changes

- The scene of a game of RGSS3 is made with `Rules::Rgss3`: a turned sprite in a viewport
  is drawn by the rule of RGSS3, everything else of the scene as RGSS2, as measured. The
  input state of such a game keeps the rules of RGSS2 until `bind-input-of-rgss3` marks the
  shown frames.
- In a game of RGSS3, and only there:
  - `Viewport.new` without arguments makes a viewport of the screen size of that moment;
    `Viewport.new(rect)` and `Viewport.new(x, y, w, h)` as before; any other count raises
    `ArgumentError` with the measured message (`wrong number of arguments (4 for 1)`).
  - Disposing a viewport disposes none of its sprites, planes, windows and tilemaps. They
    stay alive and usable, `viewport` returns the disposed object, and vkxp draws them
    nowhere while they are in it (not measured; recorded as a decision).
  - `Plane#viewport` is a reader without an argument and `Plane#viewport=` a writer.
  - The constructors of `Sprite`, `Plane` and `Tilemap` and the writers of objects of the
    three classes word their errors as the original does (`wrong number of arguments
    (1 for 2)`, `can't convert Fixnum into Viewport`, `can't convert NilClass into Color`).
  - `Graphics`: `freeze` keeps the frozen picture by the buffer rule of RGSS3;
    `snap_to_bitmap` uses the snapshot rule of RGSS3 (all of (0, 0, 0, 0) at brightness 0);
    `transition` takes its arguments as measured (no check without a freeze, a `nil` name
    raises, an empty name is no mask, a missing picture ends the freeze); `play_movie`
    exists with its measured errors and plays nothing for a file that exists; `_reset` is
    no public function. Everything else of `Graphics` (fades, waits, brightness, frame rate
    and count, `width`, `height`, `resize_screen` and its limits) is RGSS2's and is held by
    tests with the measured values.
- `vkxp-core` gets one function: disposing a viewport without its elements.
- Where the original faults (a disposed viewport given to `viewport=`), vkxp raises
  `RGSSError`; where it plays a movie, vkxp plays nothing. Both, and that the elements of a
  disposed viewport are not drawn, are recorded in `docs/compatibility.md`.
- XP and VX are unchanged: every existing test passes over both interpreters with its
  expected values unedited. That the original of VX keeps the elements of a disposed
  viewport alive too (measured on 2026-10-12, `vx-v1-api-REPORT.md`) is recorded as a known difference of VX and
  left for a follow-up.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-scene-classes`: viewport disposal by runtime; `Viewport`, `Plane#viewport`,
  constructors and writers of RGSS3; the rules of the scene of a game of RGSS3; the method
  surface of the bound classes.
- `rgss-modules`: `Graphics` of RGSS3 (method surface, freeze, snapshot, transition,
  `play_movie`, what stays RGSS2's).
- `core-scene`: disposing a viewport without its elements.
- `supported-runtimes`: the documents name the known differences and decisions.

## Impact

- `crates/vkxp-core/src/scene.rs` (one function and the documentation of the arena of
  elements) and its tests in `crates/vkxp-core/src/scene/tests.rs`.
- `crates/vkxp-binding/src`: `runtime.rs` (`rules`, a function for the rules of the input
  state), `guest.rs` (one line), `element.rs` (viewport disposal by runtime, the viewport
  argument and the object arguments of RGSS3, `Plane#viewport` of RGSS3), `viewport.rs`,
  `plane.rs`, `sprite.rs`, `tilemap.rs` (the constructor of RGSS3 calls the argument
  function of RGSS3), `graphics.rs`, `lib.rs` (documentation), `testing/host.rs` (one
  line: the harness calls `_reset` through `send`).
- New: `crates/vkxp-binding-rgss3/tests/scene_rgss3.rs` and `graphics_rgss3.rs`; one line
  of `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs` (the list `BOUND`); one new test each
  in `crates/vkxp-binding/tests/scene.rs` and `scene_rgss2.rs`.
- `docs/compatibility.md` (sections "RPG Maker VX Ace" and one sentence under "RPG Maker
  VX"), `docs/decisions.md` (section "VX Ace").
- Not touched: `vkxp-proto`, `vkxp-compose-ref`, `vkxp-host`, `vkxp-vk`, the program, the
  input module `input.rs`, `window.rs`, `color.rs`, `rect.rs`, `rb.rs`, `wrap.rs`,
  `ruby192.allow`, `tests/data/rgss3_api.txt`, `CHANGELOG.md`.
- Depends on two changes in flight, which must be merged first:
  `bind-the-values-of-rgss3` (`rb::check_count_rgss3`, `arity_rgss3.rs` with `BOUND`,
  `rgss3_api.txt`) and `run-rgss3-games-in-a-guest-of-their-own` (`Graphics.__reset__`,
  which calls `_reset`, and its edits of `runtime.rs`, `graphics.rs` and
  `testing/host.rs`).
- Other changes in flight that touch the same files: `run-rgss3-games-in-a-guest-of-their-own`
  (`runtime.rs`, `graphics.rs`, `testing/host.rs`, `lib.rs`, `docs/compatibility.md`,
  `docs/decisions.md`); `bind-the-values-of-rgss3` (`guest.rs`, `tests/scene_rgss2.rs`,
  `arity_rgss3.rs`); draft `bind-the-window-of-rgss3` (`arity_rgss3.rs`, `lib.rs`,
  `docs/compatibility.md`); draft `bind-audio-of-rgss3` (`graphics.rs`, `guest.rs`,
  `runtime.rs`, `docs/compatibility.md`, `docs/decisions.md`); draft `bind-input-of-rgss3`
  (`runtime.rs`, `guest.rs`, `graphics.rs`, `arity_rgss3.rs`, `docs/compatibility.md`),
  which builds on this change. `bind-the-tilemap-of-rgss3` is merged on `main`.
- Nothing of the original is committed: bitmaps of the tests are filled by script, and
  only rules and numbers are taken from the measurement.
