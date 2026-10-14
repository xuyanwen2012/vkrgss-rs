# Design

## Context

A game of RGSS3 sees the scene classes and `Graphics` of RGSS2 today, over a scene made
with `Rules::Rgss2` (`run-rgss3-games-in-a-guest-of-their-own`, table "What is there
today"). The archived change `draw-sprites-and-snapshots-as-rgss3` built what RGSS3 does
differently below the binding and left the binding to its Follow-ups. This change is row 3
of the table "the changes that follow" of `run-rgss3-games-in-a-guest-of-their-own`:
`Viewport.new` without arguments, `Plane#viewport` and `viewport=`, the scene of RGSS3
(turned sprites in viewports, viewports across `resize_screen`, the frozen picture,
`snap_to_bitmap` at brightness 0), over `Rules::Rgss3` and the scene of RGSS3. `Input` is
row 5, `bind-input-of-rgss3`; `Graphics.play_movie` with a picture is row 13,
`play-movies-of-rgss3`.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{runtime,guest,element,viewport,plane,sprite,tilemap,graphics}.rs`,
  `testing.rs` (`start_as`), `testing/host.rs` (`play`, `Script`, `Record`, `run_game`),
  `crates/vkxp-binding-rgss3/tests/{rgss3_script,tilemap_rgss3}.rs` (test files of the
  second package alone), `crates/vkxp-binding/tests/{scene_rgss2,graphics_rgss2}.rs` (the
  models of the tests of RGSS2), `crates/vkxp-core/src/scene.rs` (`dispose_viewport`,
  `create_screen_viewport`, `set_*_viewport`, `Rules`).
- The design of the archived `draw-sprites-and-snapshots-as-rgss3` (decision 1: every place
  that reads `Rules`; its Follow-ups: what the binding needs) and of
  `bind-the-values-of-rgss3` (decision 5: `rb::check_count_rgss3`; decision 12: the method
  list and `BOUND`).
- The measurements, outside the repository: `../../ref/tools/oracle/measurements/
  ace-v1-sprite-REPORT.md` ("the sprite report", sections 1 to 5, "Not measured / open")
  with `ace-v1-sprite-explore-log.txt` ("the log"; part 2 is the arguments and errors),
  `ace-v1-window-REPORT.md` and `ace-v1-window-facts-samples.txt` lines 196 to 198,
  `ace-v1-tilemap-REPORT.md` section 5 and `ace-v1-tilemap-api-samples.txt` lines 64 to
  68 (an element of a disposed viewport), `ace-v0-REPORT.md` section 6 (limits of
  `Graphics`), `vx-v1-api-REPORT.md` lines 337 and 338 with `vx-v1-api-behaviour-samples.txt`
  lines 496 to 512 (the same on the original of RGSS2), and `../../ref/RGSS301_api.md` ("the
  method list").

What the binding does today in a game of RGSS3, and what was measured:

| | Today | RGSS3, measured |
|---|---|---|
| scene rules | `Rules::Rgss2` | RGSS2 except a turned sprite in a viewport (`Rules::Rgss3`) |
| `Viewport.new` | 1 or 4 arguments | also none: the screen at that moment |
| `Viewport#dispose` | disposes its elements | disposes none; they stay usable, `viewport` returns the disposed object |
| `Plane#viewport` | arity 1, moves the plane | reader, arity 0; `viewport=` moves |
| counts and conversions of constructors and writers | Ruby's order, `wrong argument type` | `(1 for 2)`, `(4 for 1)`, `can't convert X into Viewport` |
| `Graphics.freeze` | frame rule | the buffer of 640 by 480 |
| `snap_to_bitmap` at brightness 0 | alpha 255 | (0, 0, 0, 0) |
| `transition` | RGSS2's | no argument check without a freeze; a `nil` name raises; `""` no mask; a missing picture ends the freeze |
| `play_movie` | not defined | arity 1; errors measured; no video was played |
| `_reset` | public | not in the method list |

Four facts:

1. **The original of RGSS2 keeps the elements of a disposed viewport alive too.**
   `vx-v1-api-REPORT.md` says so for sprites, planes, windows and tilemaps
   (`[false, false, false, false]` for `disposed?`, `viewport` returning the disposed
   viewport, `Graphics.update` running), and `vx-v1-window-facts-samples.txt` line 123
   says it of the window. Only RGSS1 disposes them (the scenario "Viewport takes its
   elements along, measured on the original" of `rgss-scene-classes`). vkxp disposes them
   in every runtime: `Scene::dispose_viewport` and `element::dispose_viewport`.
2. **What such an element draws was not measured** in any runtime. The sprite report lists
   "what a sprite or plane made in an already disposed viewport draws" as open; the
   window, tilemap and sprite reports state only that the element answers and that frames
   go on. The core already has a rule for it: "An invisible or disposed viewport SHALL hide
   its elements" (`core-scene`, "Viewport"), which today applies to an element created in a
   disposed viewport.
3. **The core moves an element out of a disposed viewport**: `set_*_viewport` checks only
   the viewport it moves to.
4. **`__reset__` of RGSS3 calls `_reset`** through `funcall`, which ignores visibility
   (`run-rgss3-games-in-a-guest-of-their-own`), and the harness evaluates
   `Graphics._reset` in `testing::host::run_game`.

## Goals / Non-Goals

**Goals:**

- A game of RGSS3 can make every form of `Viewport`, `Sprite` and `Plane` the standard
  scripts make, and its scene follows the rules of RGSS3.
- `Sprite`, `Plane`, `Viewport` and `Graphics` of a game of RGSS3 match the method list of
  RGSS3 exactly, with the measured arguments, values and errors.
- Where the original faults or plays a movie, or was not measured, vkxp does something
  defined and the documents say what.
- RGSS1 and RGSS2 do not change.

**Non-Goals:**

- `Input` and the marking of shown frames: `bind-input-of-rgss3`. The input state of a game
  of RGSS3 keeps `Rules::Rgss2` here (decision 2).
- Playing a movie: `play-movies-of-rgss3`.
- `Window` (`bind-the-window-of-rgss3`), `Tilemap` beyond the wording of its constructor,
  `Bitmap` and `Font` (`bind-font-and-text-of-rgss3`), `Audio` (`bind-audio-of-rgss3`).
- Changing RGSS1 or RGSS2, also where RGSS2 differs from the original as RGSS3 does
  (fact 1, the wording of conversions): Follow-ups.
- The word size: `frame_count` wrapping at 2^31, the `RangeError` of 2^31 for an Integer
  writer (decision "Word size" of `docs/decisions.md`).
- Any change of `vkxp-proto`, `vkxp-compose-ref`, `vkxp-host`, `vkxp-vk`.

## Decisions

1. **Selected by the runtime of the game.** As in the changes of VX and in
   `bind-the-values-of-rgss3`: `define` of each class and module registers, for
   `Runtime::Rgss3`, the functions of RGSS3 named below, and the functions and arities of
   RGSS1 and RGSS2 stay exactly as they are. `rb::RUBY_19` is not asked. The tests of
   RGSS3 are files of `crates/vkxp-binding-rgss3/tests/` alone that call
   `testing::start_as(Runtime::Rgss3)` (decision 13 of `bind-the-values-of-rgss3`).

2. **The rules of the scene.** `runtime::rules(Runtime::Rgss3)` returns `Rules::Rgss3`, so
   `new_scene` makes the scene of a game of RGSS3 with them and every sprite copies them.
   The input state is made from a new function `runtime::input_rules`, which returns
   `Rules::Rgss1` for RGSS1 and `Rules::Rgss2` for RGSS2 and RGSS3; `guest.rs` calls it
   where it calls `rules` today, and `connect` and `reset` keep taking the rules of the
   state they replace. Reason: with `Rules::Rgss3` an input state shows a snapshot only
   after a frame is marked as shown, and nothing marks one before `bind-input-of-rgss3`;
   that change removes `input_rules`. Every place of `vkxp-core` that reads `Rules` and
   what a game of RGSS3 gets there (decision 1 of the archived change):

   | Place | RGSS3 | Held by |
   |---|---|---|
   | turned sprite in a viewport (`turned.rs`) | the rule of RGSS3 | the tests of `vkxp-core` and the scenario "A turned sprite in a viewport" here |
   | unrotated sprite, wave, bush rows, flash, colour of a viewport, y in the order, `bitmap=` with the same object, tile size of a plane | as RGSS2 | the tests of `vkxp-core` ("Scene of RGSS3") and the scenario "The order of RGSS2" here |
   | repeat timing, when a snapshot is seen (`input.rs`) | as RGSS2 until `bind-input-of-rgss3` | that change |

   The window and the tilemap carry rule sets of their own and do not read `Rules`.

3. **`Viewport` of RGSS3** (sprite report 3; the log, part 2). `initialize` (-1):
   - no argument: `Scene::create_screen_viewport`, a rectangle (0, 0, width, height) of the
     screen as it is; the rectangle is an ordinary one afterwards;
   - one `Rect`: copied, as today;
   - four arguments: converted by `rb::int` (Floats truncated, negative sizes kept, `nil`
     raises the `TypeError` of the interpreter), as today;
   - any other count, or one argument that is no `Rect`: `ArgumentError` worded by
     `rb::check_count_rgss3` with the bounds (1, 1) when the first argument is a `Rect` and
     (4, 4) otherwise. That gives every measured message: `(4 for 1)` for `1`, `nil`, `"a"`
     and a `Viewport`, `(4 for 2)`, `(4 for 3)`, `(4 for 5)`, and `(1 for 2)` for
     `(Rect, 1)`.
   Everything else of `Viewport` is RGSS2's and held by tests: no `disposed?`, the disposed
   errors, `rect`, `color`, `tone` copied by their writers.

4. **Disposing a viewport in RGSS3** (fact 1, fact 2). `vkxp-core` gets
   `Scene::dispose_viewport_alone(id)`: it removes the viewport and nothing else, and
   returns nothing; an unknown or disposed handle does nothing. `element::dispose_viewport`
   calls it for `Runtime::Rgss3` and `Scene::dispose_viewport` for the other runtimes. It
   is the function behind `Viewport#dispose` and behind a collected `Viewport` object alike
   (`Kind::remove`), so a viewport the collector frees takes no element along either. In a
   game of RGSS3, then:
   - every sprite, plane, window and tilemap of the viewport stays alive: `disposed?` is
     false, its readers and writers work, `update` works, its `viewport` returns the
     disposed `Viewport` object (kept in its instance variable), `Graphics.update` goes on,
     and its own `dispose` disposes it;
   - it is drawn nowhere while it is in that viewport, by the existing rule of the core;
     it is not clipped to the old rectangle and not shown on the screen. Not measured;
     recorded in `docs/decisions.md`;
   - `viewport = nil` moves it to the screen, where it is drawn (fact 3); `viewport =`
     another live viewport moves it there. Not measured either: on the original only the
     move to a disposed viewport was tried.
   `Graphics._reset` and `__reset__` dispose every object by itself, so a reset still
   disposes everything. The scene's test counters (`scene_testing::count`) count the
   viewport alone. RGSS1 and RGSS2 keep `Scene::dispose_viewport`; that RGSS2 differs from
   its original is a Follow-up and one sentence of the section on VX of
   `docs/compatibility.md`.
   Alternatives considered: making `Scene::dispose_viewport` follow `Rules` (rejected: the
   value of RGSS2 would then encode a rule its original contradicts, or this change would
   change RGSS2); drawing such elements on the screen or in the last rectangle (rejected:
   nothing measured supports either, and the core has a rule).

5. **`Plane#viewport` and `viewport=` of RGSS3** (sprite report 2). For `Runtime::Rgss3`
   `element::define_plane_viewport` registers the reader `viewport` (0) and the writer
   `viewport=` (1) that sprites have (`viewport_of`, `set_viewport`). `plane.viewport(nil)`
   then raises the interpreter's `wrong number of arguments(1 for 0)`, as measured. The
   writer moves the plane by `Scene::set_plane_viewport`, which keeps its creation rank.

6. **Constructors and object arguments of RGSS3** (the log, part 2; tilemap report 10).
   - `element::viewport_arg_rgss3`, used by `initialize` of `Sprite`, `Plane` and `Tilemap`
     in RGSS3: `rb::check_count_rgss3(args, 0, 1)` (`Sprite.new(vp, 1)`: `wrong number of
     arguments (1 for 2)`), `nil` for none, a `Viewport` (also a disposed one, as today),
     anything else `TypeError` `can't convert <class> into Viewport`. `Window.new` is not
     touched (`bind-the-window-of-rgss3`).
   - In RGSS3 an argument of `Sprite`, `Plane` or `Viewport` that must be an object of an
     RGSS class raises, when it is not, `TypeError` worded `can't convert <class of the
     argument> into <expected class>`, the class of `nil` being `NilClass`: the writers
     `color=`, `tone=`, `src_rect=`, `rect=`, `bitmap=` and the colour of `flash`. The
     measured lines: `can't convert NilClass into Color`, `can't convert Fixnum into
     Color`, `can't convert NilClass into Tone`, `can't convert NilClass into Rect`, `can't
     convert Fixnum into Rect`, `can't convert Fixnum into Bitmap`. The wording is chosen by
     the runtime of the game in one helper of `element.rs`; `wrap::expect` and the
     wording of RGSS1 and RGSS2 stay.
   - `viewport =` with a disposed viewport (sprite and plane): `RGSSError` `disposed
     viewport`, as the binding of RGSS2 raises; the original ends with a page fault. Known
     difference.
   - Integer, Float and truth conversions are the interpreter's; their measured messages
     (`no implicit conversion from nil to integer`, `can't convert String into Integer`,
     `no implicit conversion to float from nil`) need no code and are held by tests.

7. **`Graphics` of RGSS3, method surface.** The method list has `__reset__` (added by
   `run-rgss3-games-in-a-guest-of-their-own`) and `play_movie`, and not `_reset`. In RGSS3
   the Ruby source that defines `Graphics._reset` is followed by `class << Graphics;
   private :_reset; end`: `funcall` still reaches it (fact 4), `singleton_methods(false)`
   and `respond_to?` do not. `testing::host::run_game` evaluates
   `Graphics.send(:_reset)` instead of `Graphics._reset`, which is the same call in every
   runtime.

8. **Freeze, snapshot, transition of RGSS3** (sprite report 5).
   - `freeze` sends `GuestToHost::Freeze { rule: FreezeRule::Buffer }`; RGSS1 and RGSS2
     keep `FreezeRule::Frame`.
   - `snap_to_bitmap` calls `vkxp_compose_ref::snapshot` with `SnapshotRule::Rgss3`.
   - `transition` of RGSS3, a function of its own: with nothing frozen it returns `nil` at
     once, before counting or reading any argument. After a freeze, in this order: the
     count by `rb::check_count_rgss3(args, 0, 3)` (four arguments: `wrong number of
     arguments (3 for 4)`); the duration by `rb::int`; a name given (also `nil`) is
     converted as a String with the interpreter's wording `can't convert nil into String`,
     `can't convert Fixnum into String` (the binding words `NilClass` for `nil`, so the
     function words `nil`, `true` and `false` itself); the vagueness by `rb::int`. Each of
     these errors leaves the freeze. An empty name is no mask. A name whose picture is not
     found raises `Errno::ENOENT` (`No such file or directory - <name>`) and ends the
     freeze: the guest clears `held` and sends a transition of 0 frames, as `__reset__`
     does. Otherwise as RGSS2: defaults 10 and 40, the brightness 255, the rule
     `TransitionRule::Rgss2`, `duration` frames.
   - Not followed: `(3, "")` showed 3 frames on the original, whose pixels were not looked
     at; vkxp shows a plain transition.

9. **`Graphics.play_movie(name)` of RGSS3** (sprite report 5, "`play_movie`"). Arity 1. The
   argument is converted as in decision 8 (`(nil)`: `can't convert nil into String`,
   `(5)`: `can't convert Fixnum into String`). The name is looked up in the game directory
   by `GameFiles::resolve(FileKind::Data, name)`, then with `.ogv` appended. Not found:
   `Errno::ENOENT` `No such file or directory - <name as given>`, no frame, as measured for
   `"Movies/missing"`, `""` and `"m/ramp"`. Found: `nil`, no frame, nothing played: no
   video was played on the original, and how it looks a file up was not measured (the
   extension `.ogv` is the one the editor of VX Ace writes; the run-time packages are not
   searched). Known difference; `play-movies-of-rgss3` replaces it.

10. **What stays RGSS2's in `Graphics` and is held by tests** (sprite report 5, the log,
    V0 report 6): `update`, `wait`, `fadeout`, `fadein` (counts truncated, none below 1,
    the brightness at the end also without a frame), `brightness` (0 to 255, truncated),
    `frame_rate` (10 to 120, truncated), `frame_count`, `frame_reset`, `width`, `height`,
    `resize_screen` (1 to 640 by 1 to 480, truncated, no frame, keeps brightness, frame
    count and rate, viewports keep their rectangles), `visible`, and `snap_to_bitmap` but
    for its rule. No code changes for them.

11. **Method surface.** `Sprite`, `Plane`, `Viewport` and `Graphics` are added to `BOUND` of
    `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs`, and `Tilemap` and
    `Tilemap::BitmapArray`, which `bind-the-tilemap-of-rgss3` bound before the list
    existed; nothing else of that file or of `rgss3_api.txt` is edited. Because that test
    compares `public_instance_methods(false)` of classes, `graphics_rgss3.rs` compares
    `Graphics.singleton_methods(false)` with the list itself.

12. **Tests.**
    - `crates/vkxp-binding-rgss3/tests/scene_rgss3.rs`: `Viewport`, `Plane`, `Sprite`, the
      disposal, the pixels of the scene of RGSS3, through `testing::eval` and
      `scene_support` (loaded by `#[path]`, as `rgss3_script.rs` does).
    - `crates/vkxp-binding-rgss3/tests/graphics_rgss3.rs`: `Graphics`, through
      `testing::host::play` and its `Record`, as `tests/graphics_rgss2.rs` does; a movie
      file is written into `testing::game_dir()` by the test.
    - The scenarios "XP and VX": one new test in `crates/vkxp-binding/tests/scene.rs` (a
      game of RGSS1) and one in `tests/scene_rgss2.rs` (RGSS2); both run over both
      interpreters. No existing test is edited.
    - `crates/vkxp-core/src/scene/tests.rs`: the new function.
    - No entry of `ruby192.allow` is expected to start passing; if the mechanism reports
      one, the entry and its marker are removed.

13. **Documents.** `docs/compatibility.md`, under "RPG Maker VX Ace", a subsection "Sprites,
    planes, viewports and Graphics of VX Ace" in the style of the sections of VX: what is
    as measured, and the known differences (a disposed viewport given to `viewport=`, the
    elements of a disposed viewport drawn nowhere, `play_movie`, the word size of
    `frame_count`). The bullet of "The tilemap of VX Ace" that says `Viewport#dispose`
    disposes the tilemaps of the viewport is replaced: in VX Ace the tilemap stays alive,
    as on the original, and vkxp draws it nowhere (that the original does not draw it
    either, as the bullet says today, is not in the measurement).
    Under "RPG Maker VX", in "Order, viewports, planes and input of VX", one sentence:
    `Viewport#dispose` disposes the elements of the viewport, as in XP, where the original
    of VX keeps them alive. `docs/decisions.md`, section "VX Ace": decisions 4 and 9 as
    decided without a measurement. `CHANGELOG.md` is not touched: no game of VX Ace shows
    anything new to a user by this change alone.

14. **Delta specifications.** "Viewport disposal" of `rgss-scene-classes` is MODIFIED: it
    states RGSS1, and RGSS2 by what vkxp does, and gets the scenarios of RGSS3. Everything
    else is ADDED, in the form "In a game of RGSS3 ...".

## Implementation notes

- A flow reads `../../ref/` and copies nothing from it but names, arities, rules and
  numbers. No Wine and no original runtime is run.
- Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding`, of
  `vkxp-binding-rgss3` and of `vkxp-core` under `local/`: every name must be there at the
  end.
- `Scene::dispose_viewport_alone` sits beside `dispose_viewport`; the documentation of the
  arena of elements in `scene.rs` ("disposing the viewport disposes it") says which
  function does what. Its test in `scene/tests.rs` extracts a frame with a sprite, a plane,
  a window of RGSS3 and a tilemap of RGSS3 in a disposed viewport and finds none of them
  drawn and no error, as the existing test of an element created in a disposed viewport
  does.
- A tilemap of RGSS3 notices the origin of its viewport (`notice_viewport_origins`); with
  its viewport gone it must not fail. The test above covers it.
- The scenario pixels use `pixel(x, y)` of `scene_support`, which renders the guest's scene
  with the reference compositor; "nothing" is the background (0, 0, 0). The snapshot
  scenarios read `get_pixel` of the bitmap `snap_to_bitmap` returns.
- The bitmap of the turned sprite and of the moved plane: texel column c is
  (c, 255 - c, 128), filled by `fill_rect` column by column, as the core's scenarios
  describe it.
- `GameFiles` caches directory listings: the test writes the movie file before the first
  lookup in a directory of its own (`Movies/`), or the function calls `GameFiles::refresh`
  first if the lookup fails; the harness shares one game directory between the tests of a
  binary.
- `rb::check_count_rgss3` comes with `bind-the-values-of-rgss3`. If its signature on `main`
  differs from decision 5 of that change, the message text of the scenarios is what
  counts.
- A scenario that names a message asks for that message; one that names only a class asks
  for the class.
- Edit outside the files of the Impact: at the start of this change the test
  `crates/vkxp-binding-rgss3/tests/reset_rgss3.rs` did not compile, because it compared a
  message with `GuestToHost::Freeze`, which is a struct variant since protocol version 13.
  Its line 212 now matches `GuestToHost::Freeze { .. }`; without it neither the test list of
  task 1.1 nor the gate can be had.

## Follow-ups

- **VX: the elements of a disposed viewport** (fact 1): the original of RGSS2 keeps
  sprites, planes, windows and tilemaps of a disposed viewport alive, `viewport` returning
  the disposed object (`vx-v1-api-REPORT.md` lines 337 and 338; samples lines 496 to 512;
  `vx-v1-window-facts-samples.txt` line 123). vkxp disposes them, and its test
  `a_moved_element_is_disposed_with_its_new_viewport` of `tests/scene_rgss2.rs` holds that.
  A change for VX calls `Scene::dispose_viewport_alone` for RGSS2 too and replaces that
  test. XP is measured to dispose them and stays.
- **VX: the wording of conversions**: the original of RGSS2 says `can't convert NilClass
  into Color` and `can't convert Fixnum into Color` (`vx-v1-api-behaviour-samples.txt`
  lines 451, 1660), where vkxp says `wrong argument type ...`.
- What an element made in, or left in, a disposed viewport draws: not measured in any
  runtime (sprite report, "Not measured / open", point 3).
- `Graphics.play_movie` with a file that exists, the lookup of its name and its extension:
  `play-movies-of-rgss3` (sprite report 5).
- `Graphics.visible=`: sprite report "Not measured", point 3.
- `transition(3, "")`: its pixels were not looked at (sprite report 5).
- The word size: `frame_count = 2**31` raises `RangeError` and `2**31 - 1` plus one update
  is -2147483648 on the original; vkxp keeps the value (sprite report 5; the log). An
  Integer writer with 2147483648 raises `RangeError` in both, with other messages.
- `Plane#visible` after `visible = false`: the default `true` was measured on RGSS3, the
  reading after a change was not; vkxp keeps the reader of RGSS2 (always `true`).
- `Sprite#bitmap` of a disposed sprite: the log has `nil` for a sprite without a bitmap;
  whether one with a bitmap returns it, as on RGSS2, was not tried.
- A wave on a turned sprite, a negative `wave_length`, the sprite that ends the original:
  decided in `draw-sprites-and-snapshots-as-rgss3`; nothing of the binding.
- The window keeps the size it has when the game resizes its screen (follow-up of
  `resize-the-screen-in-rgss2`).
