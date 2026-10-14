# Design

## Context

`draw-the-window-of-rgss3` built the window of RGSS3 in `vkxp-core`: a third rule set,
chosen when the window is created (`Scene::create_window_rgss3`), whose properties are
assigned through `Window::call` with a `WindowCall`, so that everything the original keeps
between calls (whether the contents show, which arrows are drawn, where the cursor was
placed, the built bitmaps) is state of the core. Its Follow-ups list what the binding has to
do, and the table "the changes that follow" of the design of
`run-rgss3-games-in-a-guest-of-their-own` names this change second, after
`bind-the-values-of-rgss3`.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{window,element,wrap,rect,color}.rs`, `crates/vkxp-core/src/
  window/{rgss2,rgss3}.rs` (the table of the module of `rgss3.rs`: which call recomputes
  what), the requirements "... of a window of RGSS3" of `openspec/specs/core-scene`.
- The designs of `draw-the-window-of-rgss3` (Follow-ups), `draw-the-window-of-rgss2` and
  `bind-the-values-of-rgss3`; `docs/decisions.md`, "The window of RGSS3".
- The measurement, outside the repository: `../../ref/tools/oracle/measurements/
  ace-v1-window-REPORT.md` ("the window report": "The class", rules 7, 9, 10, 14, 15), its
  facts log `ace-v1-window-facts-samples.txt` ("the facts log", 211 lines, the same on both
  DLL versions), `ace-v1-api-REPORT.md`, section 7 ("the API report"), and
  `../../ref/RGSS301_api.md` ("the method list").

What `window.rs` does today:

| | RGSS1 | RGSS2, and RGSS3 today |
|---|---|---|
| `initialize` | `(viewport = nil)`, `Scene::create_window` | `(viewport = nil)`, `Scene::create_window_rgss2`, a contents bitmap of 1 by 1 |
| `x=`, `y=`, `width=`, `height=`, `ox=`, `oy=`, `contents_opacity=`, `active=`, `pause=`, `windowskin=`, `contents=`, `update` | `Window::call` | `Window::call` |
| `cursor_rect=`, and every change of its `Rect` | `WindowCall::CursorRect`, through the owner of the object | the same |
| `z=`, `opacity=`, `back_opacity=`, `visible=` | setters and a field | the same |
| `openness`, `viewport=` | not defined | defined |
| `stretch` | defined | not defined |

Two facts:

1. **The binding already observes a `Rect` that is changed in place.** A `Color`, `Tone` or
   `Rect` object that belongs to an element is an `Owned<T>` of `wrap.rs` with an `Owner`,
   here `Owner::WindowCursorRect(id)`. Every method of the value class that changes the
   object ends in `Owned::set`, which stores the value and hands it to `Property::store`,
   and for this owner that is `Window::call` with `WindowCall::CursorRect`, also when the
   values are the ones the object had. This is how RGSS2's cursor is latched today. The
   forms of `set` that `bind-the-values-of-rgss3` adds go the same way. The window's tone
   needs one more owner and nothing else.
2. **Nearly every assignment is a call already.** Of the measured lists of rules 7, 9 and
   10 the binding reaches the core for each entry that RGSS2 has; `z`, `opacity`,
   `back_opacity`, `visible` and `viewport` are on none of the lists, and the core's
   setters latch nothing. What is new is the calls of the four new properties and `move`.

## Goals / Non-Goals

**Goals:**

- In a game of RGSS3 `Window` has the methods of the method list, exactly, with the
  measured arguments, defaults, return values and errors.
- Each assignment reaches the core as the call the core's table expects, so that the
  measured state machine runs for a script as it runs in the tests of the core.
- RGSS1 and RGSS2 do not change.

**Non-Goals:**

- Any rule of drawing or of state: they are `vkxp-core`'s and are tested there. A
  disagreement between a rule of the core and this design is a follow-up, not a fix here.
- `Viewport.new` without arguments, what becomes of a window when its viewport is disposed,
  the scene of RGSS3: `bind-the-scene-classes-of-rgss3`.
- `Graphics.__reset__` with windows alive (`run-rgss3-games-in-a-guest-of-their-own`).
- Text in the contents (`bind-font-and-text-of-rgss3`).
- The messages of a wrong class given to `cursor_rect=`, `windowskin=` and `contents=`,
  which every runtime shares.
- Comparing with the original: `run-the-oracle-on-vx-ace-games`.

## Decisions

1. **Defined by the runtime of the game.** `window::define` gets a third branch. Shared by
   all runtimes: what is registered for all today. RGSS1 keeps `initialize` and `stretch`,
   RGSS2 `initialize_rgss2` and `openness`. RGSS3 gets `openness` with its writer, as
   RGSS2, and:

   | Method | Arity | Function |
   |---|---|---|
   | `initialize` | -1 | decision 2 |
   | `move` | 4 | four Integers, `WindowCall::Move`, returns the receiver |
   | `open?`, `close?` | 0 | `openness() == 255`, `openness() == 0` |
   | `padding`, `padding=` | 0, 1 | `Window::padding`, `WindowCall::Padding` |
   | `padding_bottom`, `padding_bottom=` | 0, 1 | `Window::padding_bottom`, `WindowCall::PaddingBottom` |
   | `arrows_visible`, `arrows_visible=` | 0, 1 | `Window::arrows_visible`, `WindowCall::ArrowsVisible` |
   | `tone`, `tone=` | 0, 1 | decision 4 |

   `viewport` and `viewport=` come from `element::define_in_viewport` as for RGSS2. The
   writers are made with the macros `called!`, `reader!` and `writer!` of today: a
   disposed window raises before the argument is looked at, an Integer is `rb::int` (a
   Float truncated, `nil` a `TypeError`), a flag is Ruby's truth.

2. **`Window.new(x = 0, y = 0, width = 0, height = 0)`.**
   - More than four arguments: `rb::check_count_rgss3(ruby, args, 0, 4)`, the message of
     the original (`wrong number of arguments (4 for 5)`), before any argument is looked
     at, so that a viewport among five arguments is a count error as measured.
   - The arguments given are converted with `rb::int` in order, before a window exists. A
     `Viewport` is then the `TypeError` of Ruby 1.9 (`can't convert Viewport into
     Integer`), `nil` `no implicit conversion from nil to integer`.
   - Then, as the Follow-ups of `draw-the-window-of-rgss3` say: `element::create` with
     `Scene::create_window_rgss3(None)` and no viewport object; `Window::call` with
     `WindowCall::Move(x, y, width, height)`; the window's `Rect` and `Tone` objects in
     their instance variables; a new bitmap of 1 by 1 through `set_contents`, as
     `initialize_rgss2` does. The defaults are the scene's.

3. **Assignments and calls.** The whole table, for the reviewer; the rows without "new"
   exist today and are registered unchanged.

   | Script | Core | |
   |---|---|---|
   | `x=`, `y=`, `width=`, `height=`, `ox=`, `oy=` | `WindowCall::X` .. `Oy` | |
   | `contents_opacity=`, `openness=` | `WindowCall::ContentsOpacity`, `Openness` | |
   | `active=`, `pause=` | `WindowCall::Active`, `Pause` | |
   | `windowskin=`, `contents=` | `WindowCall::Windowskin`, `Contents`; the object is kept | |
   | `cursor_rect=`, a method of its `Rect` | `WindowCall::CursorRect` | |
   | `update` | `WindowCall::Update` | |
   | `padding=`, `padding_bottom=`, `arrows_visible=` | `WindowCall::Padding`, `PaddingBottom`, `ArrowsVisible` | new |
   | `move` | `WindowCall::Move` | new |
   | `tone=`, a method of its `Tone` | `Window::set_tone` | new |
   | `z=` | `Scene::set_window_z` | |
   | `opacity=`, `back_opacity=`, `visible=` | the setters and the field | |
   | `viewport=` | `Scene::set_window_viewport` | |

   The binding never compares a value with the one the window has: the core decides what
   an equal value does (rules 7 and 10 count some calls always and some only when the
   value changes).

4. **`tone`.** `wrap.rs` gets `Owner::WindowTone(WindowId)`, and `Property for Tone` hands
   it to `Window::set_tone`. The window's `Tone` object is made in `initialize` with
   `tone_of` and kept in an instance variable without `@`, as the cursor rectangle is.
   `tone` returns it; `tone=` copies the values of its argument into it with `tone_set`.
   The argument must be a `Tone`: anything else raises `TypeError` with the measured
   message `can't convert <class> into Tone` (`NilClass`, `Fixnum`, `Color`), which this
   writer words itself, as `element::set_viewport` words its own. The core reads the tone
   when the window is drawn, so a change shows in the next frame and latches nothing.

5. **Disposal.** Every method registered for RGSS3 but `dispose` and `disposed?` goes
   through `element::live` or `element::id` and raises `RGSSError`, `disposed window`.
   `dispose` returns `nil` each time. The window's bitmaps are not disposed with it.

6. **Disposed bitmaps and the faults of the original.** Nothing is coded: the core draws a
   disposed skin or contents as none and accepts the calls at which the original ends
   (`docs/decisions.md`). The class must not stand in the way: it raises for neither, and
   `windowskin` goes on returning the disposed object. The API report (section 7) names
   one more fault, `windowskin = Bitmap.new(8, 8)` followed by `contents = Bitmap.new(8,
   8)` on a new window; in vkxp both are assignments like any other.

7. **`viewport=`** is that of RGSS2, unchanged: `nil` or a `Viewport`, the object returned
   by the reader, another class `can't convert Fixnum into Viewport` as measured for
   RGSS3 too. A disposed viewport raises `RGSSError`, as in RGSS2; RGSS3 was not asked.

8. **What RGSS3 lacks.** `stretch` and `stretch=`. `crates/vkxp-binding-rgss3/tests/
   arity_rgss3.rs` gets `"Window"` in its list of bound classes: the 53 entries exist with
   their arities and the class has no public instance method beside them.

9. **Tests.** `crates/vkxp-binding-rgss3/tests/window_rgss3.rs`, a file of the second
   package alone, in the form of `rgss3_script.rs`: `testing::start_as(Runtime::Rgss3)`,
   the scene support of `vkxp-binding` through `#[path]` (`pixel`, `solid`), one evaluation
   per scenario, which disposes what it made. The scenarios "XP and VX" are one new test
   in `crates/vkxp-binding/tests/scene.rs` (RGSS1) and one in `tests/scene_rgss2.rs`
   (RGSS2), which run over both interpreters. No entry of `ruby192.allow` is expected to
   start passing; if one does, the entry and its marker are removed.

   The bitmaps of the pixel scenarios, all made with `fill_rect`:
   - *grey skin*: 128 by 128, opaque (100, 100, 100);
   - *cursor skin*: the grey skin with the cursor block (64, 64, 32, 32) opaque red;
   - *arrow skin*: the grey skin with (80, 16, 32, 32), which holds the four arrows,
     opaque (255, 0, 255);
   - *green contents*: 50 by 50, opaque (0, 255, 0).

10. **Documents.** `docs/compatibility.md` gets "The window of VX Ace" as a subsection of
    "RPG Maker VX Ace", after the subsection of the value classes: what the class has, that
    the rules are those measured (with the numbers of the window report), the page faults
    vkxp does not have, and what is not measured. `docs/decisions.md` has the decisions
    already. `CHANGELOG.md` is not touched.

11. **Delta specifications add and do not modify.** The requirement "Construction" of
    `rgss-scene-classes` is also what the changes for `Viewport` and `Tilemap` of RGSS3
    will qualify, and a modified requirement replaces the whole text when it is archived.
    The added requirement says in its own words that "Construction" holds for `Window.new`
    in RGSS1 and RGSS2 only.

## Implementation notes

- A flow reads `../../ref/` and copies nothing from it but rules and numbers. No Wine and
  no original runtime is run.
- Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding` and of
  `vkxp-binding-rgss3` under `local/`: every name must be there at the end.
- `Owned::set` must not be called inside `Guest::with`; `tone_set` and `rect_set` are
  called from the writer as `object!` does it.
- The pixel positions of the scenarios follow from the rules of `core-scene`: contents at
  (padding, padding); the down arrow at (width / 2 - 8, height - 12), 16 by 8; the cursor
  at (padding + |x - ox|, padding + |y - oy|); the background toned and then blended. If a
  position or colour stated in a scenario disagrees with what the scene draws for the
  same calls, the scene is right: assert the same property where the scene shows it, and
  write the disagreement under Follow-ups. Do not change `vkxp-core`.
- A scenario that compares two pixels of one position ("differs", "is the same") asks for
  that comparison, not for a colour.
- The interpreter of a test binary is shared: a test disposes its windows and bitmaps.
- `Window#initialize` on an object that already has a window disposes the old one, as
  `element::create` does for every class.

## Follow-ups

Left out, with the measured source. Rules are those of the window report.

- A window whose viewport is disposed: the facts log shows it alive after `w.viewport = v;
  v.dispose` (`disposed?` false, `x=` and `Graphics.update` work), and so does the facts
  log of RGSS2 (`vx-v1-window-facts-samples.txt`, line 123); vkxp disposes the elements of
  a disposed viewport in every runtime (requirement "Viewport disposal").
  `bind-the-scene-classes-of-rgss3`.
- `viewport=` with a disposed viewport: not measured for RGSS3; `RGSSError` as in RGSS2,
  where the original faults.
- The messages `can't convert NilClass into Rect` of `cursor_rect=` and `can't convert
  Fixnum into Bitmap` of `windowskin=` and `contents=` (facts log): the writers shared
  with RGSS1 and RGSS2 say `wrong argument type NilClass (expected Rect)` in every
  runtime.
- The message of a `RangeError` (`bignum too big to convert into `long'`, facts log): the
  word size, as in `bind-the-values-of-rgss3`.
- `move` with an argument that cannot be converted: which of the four values were
  assigned before the error was not measured; vkxp converts all four first.
- Whether the contents bitmap a window was made with is disposed with it: the facts log
  has it for an assigned bitmap only (it is not).
- Which call of the constructor decides the arrows first (the move, then the contents):
  not observable on the original.
- Rule 15, the sizes: a window of 20000 by 20000 with a skin and a cursor of 100000 by
  100000 fault on the original at another address; the core draws them without the parts
  that would have to be built, and no test of this change builds such a window with a
  skin.
- API report 7: the fault of `windowskin = Bitmap.new(8, 8); contents = Bitmap.new(8, 8)`
  is not among the cases of rule 15, which names windows on which `windowskin=` was never
  called. Not examined by either report.
- What the window report lists as not measured: sizes above 600, origins beyond 1000,
  `flash` of a viewport over a window, a reset with windows alive, text in the contents.
