# Design

## Context

See `proposal.md` for motivation. What this change starts from:

- `vkxp-binding` has the `Guest` state (reached through `Guest::with`, never held across a
  call into Ruby), object wrappers with a deferred-free queue, argument and error helpers,
  registration of methods with the original arities, `Color`, `Tone`, `Rect` (wrappers already
  carry an optional owner, unused so far), `Table`, `Font`, `Bitmap`, the arity test over a
  growing set of classes, and a Ruby test harness. Ruby objects owned by another object are kept
  in instance variables whose names have no `@`, which scripts cannot see. The guest already
  holds a `Scene`.
- `vkxp-core` has the `Scene` with typed handles for sprites, viewports, windows, planes and
  tilemaps (`create_*`, `dispose_*`, element access by handle, `set_*_z`), elements whose
  properties are plain fields, flash and update, extraction to a `FramePacket`, and
  `SharedTable` for tilemap tables.
- The original runtime's messages for disposed elements are `disposed sprite`,
  `disposed viewport`, `disposed window`, `disposed plane` and `disposed tilemap`, and copying
  an element fails with `can't clone <class>` or `can't dup <class>`.
- `Viewport` has no `disposed?` in the original's method list.
- The default scripts write `self.color.set(...)`, `sprite.src_rect.set(...)` and
  `window.cursor_rect.set(...)` and expect the element to change; they assign
  `tilemap.map_data = $game_map.data` once.
- A parallel change adds `Graphics`, `Input`, `Audio` and `run_game` to the same crate.

## Goals / Non-Goals

**Goals:**

- The five classes behave as scripts expect, including the reference semantics the default
  scripts depend on.
- No element or bitmap is freed while a script can still reach it, and none leaks when a script
  forgets it.

**Non-Goals:**

- Drawing behaviour itself: that is `core-scene`. Tests here check the wiring, not the pixels of
  every case.
- `Graphics.update`; tests render the scene through a helper instead.

## Decisions

### Element objects

Each class is a data class whose data is the scene handle. Its instance variables hold the Ruby
objects it refers to (viewport, bitmap, window skin, contents, tileset, the autotile list, the
tables) and the Ruby objects of its object properties. Instance variables are what keeps those
alive for the collector and what readers return, which gives identity for free.

### Owned value objects

The wrapper of `Color`, `Tone` and `Rect` has an owner field: a scene handle and a property
tag. A property object is created with the element, with the owner set. Every mutation of a
wrapper updates its own value and, if it has an owner, writes the new value to the scene
through the matching setter; a stale handle there is ignored, so an object that outlives its
disposed element behaves like a plain value. A writer such as `sprite.color = c` type-checks,
copies `c`'s value into the property object and thereby into the scene.

Alternative considered: reading the value from the scene on every access instead of keeping a
copy. Rejected because the object must keep working after its element is disposed.

### Tables and tilemaps

The `Table` wrapper stores a `SharedTable`. Assigning a table to a tilemap stores the Ruby
object in an instance variable and the shared handle in the scene. Element writes through Ruby
go to the same storage, so the tilemap sees them at the next extraction.

`TilemapAutotiles` is a data object holding the tilemap's handle and an array of seven Ruby
bitmap objects; `[]=` type-checks and forwards to the scene; an index outside 0 to 6 reads `nil`
and writes nothing.

### Bitmap properties

Assigning a bitmap stores the Ruby object in an instance variable and its `BitmapId` in the
scene; `nil` clears both. For a sprite the source rectangle property object is then set to the
bitmap's rectangle. A disposed bitmap may be assigned and simply draws nothing, as in core.

### Disposal, collection, copies

`dispose` disposes the scene element; later calls map core's disposed error to the class's
message. The free function of an element object queues its handle like bitmaps do, and the
queue's drain disposes the element. Elements are reached by the collector only through Ruby
references, so one that a script still holds is never queued. A viewport's children keep it
alive through their instance variable.

`initialize_copy` raises `TypeError` with the original's message.

### Registration and the parallel change

All classes are defined from one `install` function per module, called from `Guest::install`.
The parallel change adds its own calls next to them; the overlap is that list and the fields of
`Guest`, and is merged by hand.

### Tests

The harness gains a Ruby-visible helper, available only in tests, that extracts a frame from
the scene, renders it with `vkxp-compose-ref` (a development-only dependency, as in
`vkxp-core`) and returns the colour of a pixel. Scenario tests
are Ruby snippets using it. The collection scenarios force collections and count scene
elements through another test-only helper.

## Risks / Trade-offs

- [An owner write happens while the guest is borrowed] → Mutations of value wrappers take the
  guest borrow themselves and call nothing in Ruby while holding it.
- [Instance variables holding owned objects] → They use the foundation's hidden names (no `@`),
  which `instance_variables` does not list.
- [Disposal order during collection: a viewport freed before its sprites] → Core treats elements
  of a disposed viewport as not drawn; their own frees follow.

## Follow-ups

- `vkxp-core`'s `Scene` has no public count of its elements. `scene_count(kind)` therefore
  counts in the binding: the element classes record every element they create and dispose,
  behind the `testing` feature only. A count in core would let the helper ask the scene
  itself.
- `Guest` gives no public mutable access to the scene and the bitmap arena, and
  `vkxp-compose-ref` can only be a development dependency. `pixel(x, y)` is therefore defined
  by the integration tests (`tests/scene_support`) on top of `scene_testing::with_scene`. It
  renders a copy of the scene and uploads the referenced bitmaps whole, so that it neither
  counts a frame nor takes bitmap deltas away from `Graphics.update` once the parallel change
  is merged.
- The arity test checks the six classes of this change in a test function of its own
  (`SCENE_CLASSES`) to keep the merge with `add-rgss-runtime` free of conflicts. The scenario
  "Whole list covered" holds once that change has added `Graphics`, `Input` and `Audio`; the
  two lists can then become one.
- Not checked against the original runtime: that a disposed element raises before a writer
  looks at its argument; that assigning an autotile of a disposed tilemap is ignored; that a
  `TilemapAutotiles` object keeps its tilemap alive; the message of a copy that reaches
  `initialize_copy` without `clone` or `dup` (`can't clone <class>` here).
