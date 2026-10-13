## Context

- The element classes are in `crates/vkxp-binding/src/` (`sprite.rs`, `window.rs`,
  `plane.rs`, `tilemap.rs`, `viewport.rs`, `element.rs`); `prelude.rb` defines `Hangup`.
  What `Graphics` defines already depends on the runtime of the run; the element classes
  do not yet.
- The requirement "Method surface of the scene classes" checks a committed method list of
  RGSS1.
- Measured on the original runtime of VX (2026-10-12, both DLL versions, identical):
  - `Sprite#viewport=`, `Window#viewport=`, `Tilemap#viewport=` take `nil` or a `Viewport`
    and the reader returns the object given; another object raises `TypeError: can't
    convert Fixnum into Viewport` (the class of the argument in the message). On a disposed
    element they raise `RGSSError` as every other writer.
  - `Plane#viewport` takes exactly one argument and sets the viewport: `plane.viewport`
    raises `ArgumentError: wrong number of arguments(0 for 1)`, `plane.viewport(v)` returns
    `v`, `plane.viewport(nil)` returns `nil`, and `respond_to?(:viewport=)` is `false`.
    With a red plane and a viewport of 50 by 50, the pixel at (10, 10) is red and the one
    at (100, 100) black; after `viewport(nil)` both are red.
  - `Plane#visible` reads `true` also after `visible = false`, which does hide the plane.
  - A disposed viewport given to one of the four ends the original with a page fault, while
    the constructors accept one.
  - `Sprite#width` and `Sprite#height` are the width and height of `src_rect`, whatever
    they are: 0 and 0 on a new sprite, 100 and -12 after
    `src_rect = Rect.new(1, 2, 100, -12)`.
  - `Window.new` gives `contents` a `Bitmap` of 1 by 1, the same object on every read;
    disposing the window does not dispose it. `Window#stretch` does not exist.
  - No constant `Hangup` exists.
  - The arities of every method: `ref/RGSS202E_api.md`.

## Goals / Non-Goals

**Goals:** the names above in a game of RGSS2 as the original has them; XP unchanged.

**Non-Goals:** `Window#openness`, the `wave_*` and `bush_opacity` properties of `Sprite`,
`Tilemap#bitmaps` and `passages`, and hiding `Tilemap#tileset`, `autotiles`, `priorities`
and `TilemapAutotiles` (each comes with the change that implements its drawing);
`require` and `load`; `Graphics._reset`.

## Decisions

- The element classes are defined by the runtime of the run, as `Graphics` is: one place
  per class says what RGSS2 adds and what it leaves out.
- Moving: `vkxp-core` lets a sprite, a window, a plane and a tilemap change the viewport
  they belong to, to another or to none. The element keeps everything else, including its
  place in the creation order that breaks ties of z. How the original orders a moved
  element among elements of equal z was not measured; the code marks this.
- A disposed viewport given to one of the four writers raises `RGSSError: disposed
  viewport`. The original ends with a page fault there; vkxp does not imitate a crash.
- `Plane#viewport` in RGSS2 has arity 1 and no reader exists; the binding keeps the Ruby
  object for the collector as the other classes do.
- `Plane#visible` in RGSS2 returns `true` whatever was set.
- The contents bitmap of a new window in RGSS2 is an ordinary bitmap of the arena, made
  with the window.
- `Hangup` is defined by the prelude only in RGSS1.
- The method list of RGSS2 for `Sprite`, `Viewport`, `Window`, `Plane`, `Tilemap` and
  `Tilemap::BitmapArray` is committed beside that of RGSS1, taken from
  `ref/RGSS202E_api.md` (names and arities only). The test checks every entry except those
  of a list of pending names written in the test itself: `Window#openness(=)`,
  `Sprite#bush_opacity(=)` and `wave_*(=)`, `Tilemap#bitmaps`, `passages(=)` and
  `Tilemap::BitmapArray`. The test fails when a pending name exists, so the list shrinks
  with the changes that add them. Names that RGSS2 lacks and vkxp still defines there
  (`Tilemap#tileset`, `autotiles`, `priorities`, `TilemapAutotiles`) are a second such
  list.

## Notes of the implementation

- Edits in `crates/vkxp-binding` beside the element classes, each a few lines that the
  requirements need: `guest.rs` passes the runtime of the run to the definition of the
  scene classes; `runtime.rs` evaluates `prelude_rgss1.rb`, which now holds `Hangup`, in a
  game of RGSS1 only; `bitmap.rs` has `new_blank`, which makes the contents bitmap of a new
  window; `testing.rs` has `start_as`, with which a test binary makes the harness a game of
  RGSS2 (`tests/scene_rgss2.rs`, `tests/arity_rgss2.rs`).
- The parser of the method lists moved from `tests/arity.rs` to `tests/method_list/`, so
  that the test of RGSS2 shares it; the tests of `arity.rs` are as they were.
- The test of the list of RGSS2 also fails when one of the six classes has a public method
  that is neither in the list nor in the second list, which is what keeps that list exact.

## Follow-ups

- How the original orders a moved element among elements of equal z is not measured (see
  "Decisions"); `Scene::set_sprite_viewport` and its siblings mark it.
- `Plane#visible` on a disposed plane in RGSS2 was not measured; it raises `RGSSError` as
  the other readers do.
- `tests/data/rgss2_api.txt` holds the scene classes only. The other classes of RGSS2
  (`Bitmap`, `Font`, `Graphics` and so on) are added by the changes that bind them.
