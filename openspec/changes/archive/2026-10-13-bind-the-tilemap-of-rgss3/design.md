# Design

## Context

The table "the changes that follow" of `run-rgss3-games-in-a-guest-of-their-own` names
this change as number 7: "`Tilemap#flags`, the four layers, visibility", source
`ace-v1-tilemap-REPORT.md`, core API `Scene::create_tilemap_rgss3`, depends on nothing.
The Follow-ups of the archived `2026-10-13-draw-the-tilemap-of-rgss3` say what the binding
has to do: call `Scene::create_tilemap_rgss3`; `flags` and `flags=` in the place of
`passages` and `passages=` with the same reference and nil rules, refreshing the tilemap;
accept a write to `bitmaps[]` of a disposed tilemap; the method list of RGSS3.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{tilemap,element,table,wrap}.rs`,
  `crates/vkxp-binding/tests/tilemap_rgss2.rs` and `scene_support/`,
  `crates/vkxp-binding-rgss3/tests/` (`rgss3_script.rs` is the model of a test file of
  RGSS3), `crates/vkxp-core/src/scene.rs` (`create_tilemap_rgss3`) and
  `tilemap/rgss2.rs`.
- The measurement of the original runtime of RGSS3, outside the repository:
  `../../ref/tools/oracle/measurements/ace-v1-tilemap-REPORT.md` ("the tilemap report"),
  sections 4, 5, 7, 8 and 10, with `ace-v1-tilemap-api-samples.txt`; `ace-v1-api-REPORT.md`
  ("the API report"), section 4.1; `../../ref/RGSS301_api.md` ("the method list").

What is there today:

| Place | Today |
|---|---|
| `tilemap::define` | two shapes: RGSS1, and everything else as RGSS2 (`initialize_rgss2`, `bitmaps`, `map_data=` that refuses `nil` once set, `passages`, `passages=`) |
| a game of RGSS3 | gets the shape of RGSS2 and a tilemap of the scene with the rule set of RGSS2 |
| `Scene::create_tilemap_rgss3` | makes a tilemap of RGSS2 whose rule set is that of RGSS3; its field `rgss2.passages` holds the flags table; a map table is read with the layers it has, missing ones empty; nothing is drawn while `visible` is false |
| `set_sheet` (`bitmaps[i] = b`) | on a disposed tilemap: keeps the object in the array and changes nothing of the scene |
| the tests of the binding over Ruby 1.9.2 | run every test file of `vkxp-binding` a second time; files that exist in `crates/vkxp-binding-rgss3/tests` alone hold what only RGSS3 has |

The measurement found the class of RGSS3 equal to that of RGSS2 in everything a script
can call except three things (tilemap report, item 16 and section 10): `passages` is gone,
`flags` is new, and a write to a slot of a disposed tilemap ends the original.

## Goals / Non-Goals

**Goals:**

- A game of RGSS3 sees the class `Tilemap` the method list has, with the values and
  errors of the measurement, and what it draws follows the rule set of RGSS3.
- RGSS1 and RGSS2 do not change: every existing test passes over both interpreters with
  its expected values unedited.

**Non-Goals:**

- The drawing rules themselves (tilemap report 0 to 9): they are `vkxp-core`'s and are
  tested there against the model of the measurement.
- `Viewport#dispose` of RGSS3. On the original a tilemap whose viewport is disposed is
  not disposed and still answers (tilemap report 5 and 10); vkxp disposes the elements of
  a disposed viewport in every runtime today. That rule is the viewport's and concerns all
  four element classes; see Follow-ups.
- The wording of errors that other classes share: the count message of the constructors,
  the `TypeError` of a value that is no `Bitmap`, the `RangeError` of 2**31.
- The names `bmp0` to `bmp8` at their first use: over Ruby 1.9.2 symbols are not numbered
  and a dump does not depend on the numbers (the decision on the binding in
  `docs/decisions.md`; API report 4.1 and 4.2). The array keeps its nine objects under
  these names as in RGSS2, which no script can see.

## Decisions

1. **A third shape of `tilemap::define`, chosen by the runtime of the game.** For RGSS3:
   `initialize` (a new function that differs from `initialize_rgss2` in the one call,
   `scene.create_tilemap_rgss3(viewport)`), `bitmaps`, `map_data=` with the rule of nil of
   RGSS2 (`set_map_data_rgss2`), `flags` and `flags=`, and what all three shapes share.
   `passages` and `passages=` are not defined. `define_autotiles` already gives every
   runtime but RGSS1 `Tilemap::BitmapArray`. No `cfg`, no check of the interpreter.

2. **`flags` is `passages` under another name.** The reader is `kept!` over an instance
   variable `flags`; the writer is `set_passages` with that variable: `keep_table`, then
   `set_table`, which stores the shared table in `rgss2.passages` of the scene's tilemap
   and refreshes it. The field of the scene and of the plane op keeps its name
   `passages`: a rename would edit `vkxp-core` and `vkxp-proto` for no behaviour.

3. **Nil.** As measured and as RGSS2: `map_data = nil` and `flags = nil` are accepted
   while the property never held a table and read `nil`; once it holds one they raise
   `TypeError` with the message `can't convert NilClass into Table` and the table stays.
   `flash_data = nil` is always accepted. This is `keep_table` as it is.

4. **A slot of a disposed tilemap.** `bitmaps` goes on returning the array, a slot reads
   what it held, and a write is accepted and kept, as `set_sheet` does today for RGSS2.
   The original ends with a page fault at the write (four runs, both DLLs: a freed object
   is written). Known difference, recorded in `docs/compatibility.md`: the original ends,
   vkxp goes on and the slot reads the object back.

5. **What needs no code.** `visible=` refreshes when the value changes and the core hides
   a tilemap of the rule set of RGSS3; `ox=`, `oy=`, `flash_data=`, `update`,
   `viewport=`, `dispose`, `disposed?`, `clone` and `dup` are those of RGSS2. A map of
   fewer than four layers is drawn with the missing ones empty by the core. The tests
   hold these for a game of RGSS3 all the same, because nothing did so far.

6. **The method surface is tested in the test file**, against the list written out in it
   (the public instance methods of `Tilemap` and of `Tilemap::BitmapArray` with their
   arities, from the method list and tilemap report 10), not against a shared data file:
   several changes bind classes of RGSS3 at the same time, and a data file they all edit
   would be the one place where they meet.

7. **Tests.** `crates/vkxp-binding-rgss3/tests/tilemap_rgss3.rs`, a file of that package
   alone, starts the harness with `testing::start_as(Runtime::Rgss3)` and installs
   `scene_support` by its path in `vkxp-binding`, as `rgss3_script.rs` does. That is how
   a test of RGSS3 is kept from running as RGSS1 or RGSS2 over Ruby 1.8.7: the test of
   `xtask` asks for a counterpart of every file of `vkxp-binding`, not the reverse. The
   pixels are read with `pixel(x, y)` of the support module. A snippet disposes what it
   made. The sheets are bitmaps of one colour (`solid`); tile 1 is drawn from slot 5 at
   (32, 0), so that sheet is 64 by 32.
   - No entry of `ruby192.allow` concerns the tilemap; the list is not edited.

## Implementation notes

- A flow reads `../../ref/` and copies nothing but rules and numbers. No Wine and no
  original runtime is run.
- Before anything changes, keep under `local/` the lists of `cargo test --locked --
  --list` for `vkxp-binding` and `vkxp-binding-rgss3`. At the end every name is there
  again.
- The message `disposed tilemap`, the `TypeError` of `dup` and `clone` (`can't dup
  Tilemap`, `can't clone Tilemap`) and the conversions of `ox=` are what `element.rs`
  gives today; the scenarios state them so that a game of RGSS3 is held to them.
- The scenarios on RGSS1 and RGSS2 ("XP and VX") are assertions added to existing tests
  of `tilemap_rgss2.rs` and `scene.rs`; no existing assertion is edited.
- `docs/compatibility.md`, section "RPG Maker VX Ace" (created in the style of the section
  on VX if it is not there): the class `Tilemap` as RGSS3 has it, and the known
  differences: the write to a slot of a disposed tilemap, and, from "VX Ace" of
  `docs/decisions.md`, the tile numbers and the maps of fewer layers for which the
  original reads outside its arrays. `CHANGELOG.md` under "Unreleased" (created if
  absent).

## Risks

- A game calls `passages=` on a tilemap of RGSS3 (a script ported from VX). It raises
  `NoMethodError`, as on the original.

## Follow-ups

- `Viewport#dispose` in RGSS3 does not dispose the elements of the viewport: a tilemap
  whose viewport is disposed answers `disposed?` with `false`, takes `update` and `ox=`,
  is not drawn, and is disposed by its own `dispose` (tilemap report 5 and 10;
  `ace-v1-sprite-REPORT.md` says the same of sprites and planes). vkxp disposes them with
  the viewport in every runtime. For `bind-the-scene-classes-of-rgss3`, which binds
  `Viewport`.
- The count message of `Tilemap.new(viewport, 1)`: the original says `wrong number of
  arguments (1 for 2)`, the accepted count first (tilemap report 10; API report 0). It is
  the message of every constructor of an element and comes with
  `bind-the-values-of-rgss3`, which words the count messages.
- The message of a value that is no `Bitmap` in a slot (`can't convert Fixnum into
  Bitmap`) and of 2**31 for `ox=` (``bignum too big to convert into `long'``): tilemap
  report 10. The classes of the errors are held by this change.
- The private method `initialize_copy` that every element class of vkxp has is not in the
  method list of RGSS3 for `Tilemap`; it is private and refuses a copy as `clone` does.
- The names `bmp0` to `bmp8` interned at the first use of a slot (API report 4.1): for
  `intern-the-names-of-rgss3`, if a game shows a need.
- The field `passages` of the scene's tilemap and of the plane op holds the flags of RGSS3
  under its old name.
- A tilemap that nothing refers to is collected and no longer drawn (tilemap report 5,
  `api3`): the requirement "Unreachable elements are released" covers it for every
  runtime and is not tested again here.
- `ox` and `oy` near 2**30 are stored and were not drawn on the original (tilemap report,
  "Not measured", point 3).
