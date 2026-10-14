## Why

Seven frames after its first, the standard script set of RPG Maker VX Ace calls
`Input.repeat?(:DOWN)` (fact 1 of the design of `run-rgss3-games-in-a-guest-of-their-own`).
In a game of RGSS3 that raises `TypeError` today: the module `Input` is the one of RGSS1
and RGSS2, whose constants are Integers and whose queries convert their argument to an
Integer. The input of RGSS3 was measured on the original runtime on 2026-10-13
(`ace-v1-sprite-REPORT.md` section 6 with its samples and logs, `ace-v0-REPORT.md` section
4 and `RGSS301_api.md`, outside the repository): the constants are Symbols, the queries
take a Symbol or the Integer of RGSS2 and give `false` for anything else, the timing of
`trigger?`, `repeat?`, `dir4` and `dir8` is that of RGSS2, and a key is seen only through
the calls of `Graphics` that show frames. `vkxp-core` has the input state of RGSS3 since
`draw-sprites-and-snapshots-as-rgss3` (`Rules::Rgss3`, `InputState::frame_shown`); this
change binds it. It is change 5 of the table "the changes that follow" of
`run-rgss3-games-in-a-guest-of-their-own` and on the path to milestone A1.

## What Changes

- In a game of RGSS3, and only there:
  - The twenty constants of `Input` are the Symbols of their names (`Input::DOWN == :DOWN`).
  - `press?`, `trigger?` and `repeat?` take exactly one argument and never raise for its
    class: a Symbol of a button name or the Integer of RGSS2 of a button (2, 4, 6, 8, 11 to
    18, 21 to 23, 25 to 29) names that button; anything else gives `false`.
  - The input state follows the rules of RGSS3: an `Input.update` sees the input that had
    arrived when a frame was last shown, so `Input.update` alone never sees a new key, and
    a key that arrives before a `Graphics.update` is seen by the `Input.update` after it.
  - Repeat (24 and 6, one button at a time), `dir4` and `dir8` are those of RGSS2, as
    measured; the keys are those of VX (F5 to F9 included), which the host maps already.
- The guest marks every frame it shows (`Graphics.update` and each frame of `wait`,
  `fadeout`, `fadein` and `transition`) on the input state; with the rules of RGSS1 and
  RGSS2 that changes nothing.
- The known differences of the timing of keys (how many key messages a frame of the
  original takes, a key pressed and released between two frames, a key seen after
  `Kernel#sleep`) are recorded in `docs/compatibility.md`.
- XP and VX are unchanged: every existing test passes over both interpreters with its
  expected values unedited.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-modules`: `Input` of RGSS3 (constants, arguments, when a key is seen, the method
  surface); the requirements "Input" and "Method surface of the modules" qualified by
  runtime.
- `supported-runtimes`: the documents name the known differences.

## Impact

- `crates/vkxp-binding/src`: `input.rs` (the definition by runtime, the constants, the
  argument rule), `runtime.rs` (the input state of RGSS3 takes `Rules::Rgss3`: the function
  `input_rules` of `bind-the-scene-classes-of-rgss3` goes), `guest.rs` (one line),
  `graphics.rs` (one call in `frame`, where every shown frame passes), `lib.rs`
  (documentation).
- New: `crates/vkxp-binding-rgss3/tests/input_rgss3.rs`; one line of
  `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs` (the list `BOUND`); one new test in
  `crates/vkxp-binding/tests/graphics_rgss2.rs`.
- `docs/compatibility.md`: a subsection of "RPG Maker VX Ace".
- Not touched: `vkxp-core` (the rules exist), `vkxp-proto`, the host crates and the key
  maps of the program, `win32api.rs`, `ruby192.allow`, `tests/data/rgss3_api.txt`,
  `docs/decisions.md` (it records the rule of shown frames already), `CHANGELOG.md`.
- Depends on `bind-the-scene-classes-of-rgss3`, merged first, and through it on
  `bind-the-values-of-rgss3` (`arity_rgss3.rs` with `BOUND`).
- Other changes in flight that touch the same files: `run-rgss3-games-in-a-guest-of-their-own`
  (`runtime.rs`, `graphics.rs`, `lib.rs`, `tests/graphics_rgss2.rs`,
  `docs/compatibility.md`); `bind-the-values-of-rgss3` (`guest.rs`, `arity_rgss3.rs`);
  drafts `bind-the-scene-classes-of-rgss3` (`runtime.rs`, `guest.rs`, `graphics.rs`,
  `arity_rgss3.rs`, `docs/compatibility.md`), `bind-the-window-of-rgss3` (`arity_rgss3.rs`,
  `lib.rs`, `docs/compatibility.md`) and `bind-audio-of-rgss3` (`graphics.rs`, where it
  adds a call in the same place of `frame`, `guest.rs`, `runtime.rs`,
  `docs/compatibility.md`).
- Nothing of the original is committed: only rules and numbers are taken from the
  measurement.
