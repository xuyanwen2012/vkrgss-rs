## Context

- A scene follows the rules of one runtime, `Rules { Rgss1, Rgss2 }` of `vkxp-core`
  (`scene.rs`), given when it is made; a sprite copies them when it is created and an
  `InputState` is made with them. `Rules` is matched in `input.rs` (repeat timing),
  `plane.rs` (tile size), `extract.rs` (the unrotated sprite, the viewport's colour and
  flash, the y in the draw order, a flash without a colour), `element.rs` (`bitmap=` with
  the same object, the wave phase) and `turned.rs` (the picture of an unrotated sprite, the
  bush rows of a turned one). `vkxp-binding` maps `Runtime::Rgss3` to `Rules::Rgss2`. The
  window and the tilemap of RGSS3 carry rule sets of their own and left `Rules` alone
  (changes `draw-the-window-of-rgss3`, `draw-the-tilemap-of-rgss3`).
- A turned sprite is rendered on the CPU by `vkxp-core` into a bitmap of the scene's own and
  drawn as an unscaled quad (`turned.rs`). `Sprite::turned_placement` is where the shift of
  RGSS1 and RGSS2 lives: the box comes from a `Geometry` at `x - viewport.ox` and is put at
  the viewport's corner, the rows are walked from a `Geometry` at the raw `x`. Neither
  compositor knows about turned sprites.
- `Scene::set_sprite_viewport`, `set_window_viewport`, `set_plane_viewport` and
  `set_tilemap_viewport` move an element and keep its creation stamp; a disposed viewport
  fails with `SceneError::Disposed` and moves nothing. Their documentation says the order
  after a move was not measured.
- `Scene::set_logical_size` changes the size of the screen; `level_of` clips a viewport to
  `rect ∩ screen` for every frame. `Scene::snapshot` gives a packet that stands alone, with
  the brightness of the scene as it is now.
- `Graphics.snap_to_bitmap` of RGSS2 is in the binding (`graphics.rs`): by the owner's
  decision of 2026-10-12 the guest composes the snapshot packet itself with the CPU
  reference compositor, which applies the brightness (`(v * b) >> 8`, unchanged at 255,
  requirement "Brightness of the frame"); the binding then sets every alpha to 255.
- The frozen picture is the host's: `GuestToHost::Freeze` makes the session ask its display
  to hold (`Display::hold`), which in `vkxp-vk` copies what is shown into a held image of
  the frame's own size (`compositor/present/held.rs`). `present.frag` mixes held and newest
  frame texel by texel in integers when both have one size, and otherwise scales each into
  its own letterbox rectangle and mixes per target pixel. The session steps a transition by
  `TransitionRule`. The reference compositor has no freeze or transition.
- Input: the host sends `InputSnapshot`s, a scheduled or replayed one ahead of the tick of
  its frame, a windowed one whenever the state changes. The guest takes messages while it
  waits for a tick (`frame` in `graphics.rs`) and, without waiting, in `Input.update`;
  `InputState::update` latches the most recent snapshot received ("Update latches a
  snapshot" of `core-input`).
- The measurement, seen from a clone under `wt/`:
  `../../ref/tools/oracle/measurements/ace-v1-sprite-REPORT.md`, its model
  `ace-v1-sprite-model.py` (the model of VX, `vx-v1-sprite-model.py`, with the one option
  `tv=fix`), the topics `ace-v1-sprite-topics.py` over `vx-v1-sprite-topics.py`, the frames
  `ace-v1-sprite-frames/<topic>/NNNNNN.png` (frames of generated bitmaps, which
  `ace-v1-sprite-model.py check <topic> ace-v1-sprite-frames/<topic>` re-checks without
  Wine), `ace-v1-sprite-model-check.txt`, `ace-v1-sprite-vx-compare.txt`, and the input
  samples `ace-v1-sprite-input-samples.txt.gz` with `ace-v1-sprite-input-model.py`. The
  report is the authority for every rule below.
- No script of RGSS3 can run yet: the interpreter and the guest of RGSS3 are other changes.

### Measured rules (RGSS3, versions 3.0.0.1 and 3.0.1.1 identical)

1. A turned sprite in a viewport (report section 1). With `X = x + rect.x - ox` and
   `Y = y + rect.y - oy` of its viewport, the sprite is drawn by the rotation rule of RGSS1
   and RGSS2 as a sprite without a viewport at (X, Y), clipped to the viewport's rectangle.
   Box and walk use the same position. 359 frames, 81243136 pixels, 0 wrong, the held-out
   set `aheldx` (20 frames) among them; the rule of RGSS2 leaves 213035 wrong.
2. Everything else of sprites, planes and viewports is RGSS2 (24 of 32 topics byte-identical
   in every frame): pixel arithmetic, zoom, rotation without a viewport origin, `src_rect`
   leaving the bitmap, bush, wave, flash, the plane's tile and tiling, the viewport's clip,
   tone, colour and flash, and the order (z, y of a sprite, creation).
3. `viewport=` of a plane and of a sprite: the element is drawn in the other viewport (its
   clip, its origin) or on the screen, and keeps its creation rank among the elements of
   equal z of its new container (`aplanevp`, 6 frames, 0 wrong).
4. `Viewport.new()` without arguments: rect (0, 0, `Graphics.width`, `Graphics.height`) as
   they are at that moment (measured at 544x416, 640x480, 320x240, 200x300, 1x1). The rect
   of a viewport never changes by itself.
5. `resize_screen` shows and counts no frame and keeps brightness, `frame_count`,
   `frame_rate` and a freeze. No viewport is changed: the clip is `rect ∩ new screen`. An
   element without a viewport covers the whole new screen (49 bounding boxes).
6. The frozen picture is one buffer of 640 by 480 pixels, black at the start of the
   process. `Graphics.freeze` copies the screen, width by height as they are then, into its
   top left corner and leaves the rest. A transition blends buffer pixel (x, y) with the
   new scene for every pixel of the screen as it is now, by the step formula of RGSS2;
   nothing is scaled or centred; frames merely shown do not enter the buffer (13 screenshots
   of 640 by 480, 0 wrong; plain transitions only).
7. `snap_to_bitmap`: with `s` the composed pixel of the scene as the objects are now (a
   freeze is ignored) and `b` the brightness at the call: `(s.r, s.g, s.b, 255)` for b 255;
   `((s.r * b) >> 8, (s.g * b) >> 8, (s.b * b) >> 8, 255)` for 0 < b < 255; `(0, 0, 0, 0)`
   for the whole bitmap at b 0. The alpha of the scene plays no part (256 snapshots, 0
   wrong). RGSS2 was measured with alpha 255 and not at brightness 0.
8. Brightness on the screen, fades, plain and masked transitions, their steps and the mask
   level are RGSS2's.
9. `Input`: the constants are Symbols of their names; `press?`, `trigger?` and `repeat?`
   never raise for the class of their argument and can be true for the Symbol and for the
   Integer of RGSS2 of the same button; every other argument gives false. The key table is
   that of RGSS2.
10. `trigger?`, `repeat?` (delay 24, interval 6), `dir4`, `dir8`: the model of RGSS2, 12084
    updates, 0 mismatches when the model is fed the held set the original reported.
11. When a key is seen: `Input.update` takes no window message and reads the key state of
    its thread; calls that show no frame take none either; every call that shows frames
    takes messages per frame shown: exactly one for a frame without time to wait, more, up
    to all that are queued, for a frame that waits. How many in between is not a rule.
12. Faults of the original: a disposed viewport given to `Sprite#viewport=` or
    `Plane#viewport=` ends the process; one sprite (bitmap 20 by 90, `src_rect`
    (8, 38, 9, 26), zoom 2.5 by 0.7, angle 180, `bush_depth` 20, at (434, 160)) ends it
    inside `Graphics.update`, condition not found; a negative `wave_length` reads outside
    the sine table (pictures differ between the two DLL versions).

## Goals / Non-Goals

**Goals:**

- Rules 1, 3, 4, 5, 6, 7, 10 and the vkxp side of 11 in `vkxp-core`, `vkxp-compose-ref`,
  `vkxp-proto`, `vkxp-host` and the presenter of `vkxp-vk`, each with tests that state the
  measured rule, so that the binding of RGSS3 only hands data over.
- The CPU reference and the Vulkan compositor agree byte for byte on the new scenes.
- Frames, display and input of XP and VX do not change.

**Non-Goals:**

- The classes `Sprite`, `Plane`, `Viewport` and the modules `Graphics` and `Input` of RGSS3
  in `vkxp-binding`: method lists, argument forms, errors, Symbols. Nothing of an
  interpreter. The measured rules they need are kept under "Follow-ups".
- `Graphics.play_movie`.
- What the report lists as open or not measured: a wave on a turned sprite, a wave with a
  stretched cut source rectangle, what an element made in a disposed viewport draws,
  `Graphics.visible=`, a masked transition after a resize, the message taken during
  `Kernel#sleep`, gamepads.
- Whether RGSS2 keeps its frozen picture as RGSS3 does. It was not measured, so a game of
  RGSS2 keeps the display it has.
- `docs/compatibility.md` and any statement that VX Ace runs.

## Decisions

Where each of the six measured differences goes:

| Point | Below the binding (this change) | Interpreter-bound (Follow-ups) |
|---|---|---|
| Turned sprite in a viewport | all of it: `vkxp-core` | nothing |
| `snap_to_bitmap` | the pixels: `vkxp-compose-ref`; the brightness at the call and the scene as it is: `Scene::snapshot`, as today | the method of RGSS3 calling it with the rule of RGSS3 |
| Frozen picture, `resize_screen`, `Viewport.new()` | the buffer: protocol, session, presenter; the viewport of the screen and the size: `vkxp-core` | `Graphics.freeze` sending the rule; the argument forms |
| `Plane#viewport=` | the move and the rank: `vkxp-core`, there already | reader, writer and their arities |
| Input | timing and the frame at which a snapshot is seen: `vkxp-core` | Symbols, Integers, false for other arguments; the call that marks a frame |
| Faults | what the scene does: `vkxp-core`, tested | what `viewport=` raises |

1. **`Rules::Rgss3` now.** The scene's `Rules` get the value `Rgss3`, and
   `InputState::with_rules` takes it. Reasons: the difference of rule 1 is how a sprite
   stands in its viewport, which the sprite reads from the rules it copied from its scene
   at creation, through the one `Scene::create_sprite`; a rule set of its own would need a
   second constructor and a second field that says the same as `Rules` for every sprite of
   a game. The input state is made from `Rules` and now differs too. The window and the
   tilemap did not need the value because each has a constructor of its own; they keep
   their rule sets, and a window or tilemap of RGSS3 lives in a scene of any rules.
   Alternatives considered: a rule set per sprite (rejected, above); one value per scene
   beside `Rules` (rejected: it would be `Rules` under another name).
   With the value, every place that reads `Rules` is decided, and RGSS3 equals RGSS2
   unless measured otherwise:

   | Place | RGSS3 | Basis |
   |---|---|---|
   | repeat timing (`input.rs`) | 24 and 6, as RGSS2 | rule 10 |
   | when a snapshot is seen (`input.rs`) | new, decision 5 | rule 11 |
   | tile size of a plane (`plane.rs`) | doubles, as RGSS2 | topics `plane`, `size`, `size2` identical |
   | unrotated sprite, its picture, wave, bush rows (`extract.rs`, `extract/rgss2.rs`, `turned.rs`) | as RGSS2 | topics `zoom`, `clip*`, `srcrect*`, `wave1` to `wave6`, `bush*`, `edge` identical |
   | viewport colour then flash; nothing during `flash(nil)` (`extract.rs`) | as RGSS2 | topics `viewport`, `flash` identical |
   | y of a sprite in the order (`extract.rs`) | as RGSS2 | topic `order` identical, 160 pairs |
   | `bitmap=` with the same object keeps `src_rect`; `update` adds to the wave phase (`element.rs`) | as RGSS2 | report section 1, exploration log |
   | bush rows of a turned sprite (`turned.rs`) | as RGSS2 | topics `xbush`, `geom2` with rule 1 |
   | turned sprite in a viewport (`turned.rs`) | new, decision 2 | rule 1 |

   Places that compare with `Rules::Rgss2` (`==`) are written so that RGSS3 takes the side
   of RGSS2; a `match` without a wildcard is preferred, so that a fourth value could not be
   forgotten. `Rules::default()` stays `Rgss1`.
2. **Turned sprites.** With `Rules::Rgss3`, `Sprite::turned_placement` builds one
   `Geometry` at `(x + level.dx, y + level.dy)` (`dx = rect.x - ox`), takes the box from it
   on the screen and walks the rows from it; the corner is (0, 0). The position is added
   before the rotation is computed, as the model does, not afterwards. The key of the
   rendered bitmap (`TurnedKey`) stays valid: with no shift the picture depends on the
   position only through the part of the box inside the clip. Bush rows, opacity, blend,
   tone and colour are those of a turned sprite of RGSS2. A turned sprite whose source
   rectangle leaves its bitmap is not drawn, as before.
3. **The viewport of the screen.** `vkxp-core` gets a way to create a viewport without a
   rectangle (a constructor beside `Scene::create_viewport`): its rectangle is
   (0, 0, width, height) of the logical size at that moment and is an ordinary rectangle
   afterwards. `Scene::set_logical_size` already leaves every element and rectangle alone
   and `level_of` already clips to the screen of the frame, so rule 5 needs no code, only
   tests that state it.
4. **Moved elements.** The scene already does what rule 3 measured. The remark "not
   measured" at `set_sprite_viewport` and its siblings is replaced by the provenance (for
   RGSS2: the order key of `vx-v1-sprite-REPORT.md`, which `viewport=` does not renew; for
   RGSS3: topic `aplanevp`). A disposed viewport still fails with `SceneError::Disposed` and
   moves nothing (decision 10).
5. **When a snapshot is seen (rule 11).** vkxp has no message queue of keys: the host sends
   whole input states, and a scheduled or replayed run delivers the state of frame n ahead
   of that frame's tick. The rule vkxp follows for RGSS3: **an update latches the most
   recent snapshot that had arrived when a frame was last shown; a snapshot that arrives
   afterwards waits for the next shown frame.** `InputState` gets a call that marks a shown
   frame (`frame_shown`, or a name of that meaning): with `Rules::Rgss3` it makes the
   snapshots received so far visible to the next update; with the other rules it changes
   nothing and `update` latches the most recent snapshot received, as today. The binding of
   RGSS3 will call it once per frame it shows (`Graphics.update`, and each frame of `wait`,
   `fadeout`, `fadein` and a `transition` after a freeze) and from nowhere else.
   This reproduces: `Input.update` alone never sees a new key; a key change before a
   `Graphics.update` is seen by the `Input.update` after it (240 of 240 on the original);
   `trigger?` is computed at `Input.update` from what is seen then. A scheduled run gives
   the same answers at the same updates as it would under the rules of RGSS2, because the
   snapshot of a frame arrives ahead of that frame's tick.
   Known difference: the original takes exactly one key message in a frame that has no
   time to wait and more, up to all, in a frame that waits; how many is not a rule
   (rule 11), so vkxp does not imitate a count. Every shown frame makes the whole state as
   it arrived visible. Consequences: several key changes between two frames are seen
   together, where a frame of the original without slack spreads them over the following
   frames; a key pressed and released between two frames is never seen, where the original
   saw it for one update when the frame handled only the key-down (2 of 5 in one
   measurement, 0 of 16 in clean loops). The message the original takes now and then
   during `Kernel#sleep` is not imitated either.
6. **Arguments of `Input`** (rule 9) are the binding's: they are about the class of a Ruby
   object. Nothing of them is built here; the rule is kept under "Follow-ups".
7. **Snapshots.** `vkxp-compose-ref` gets a function that takes a packet that stands alone
   and a rule (an enum with the values of RGSS2 and RGSS3, defined there, because the crate
   does not depend on `vkxp-core`) and returns the pixels of the snapshot: the composed
   frame, brightness applied by the compositor as for every frame, with alpha 255
   everywhere; by the rule of RGSS3 every byte is 0 when the packet's brightness is 0. It is
   in the reference compositor because that is what composes a snapshot, in the guest; the
   Vulkan compositor draws no snapshot. `snap_to_bitmap` of the binding calls it with the
   rule of RGSS2 instead of its own loop, so the rule has one definition; its tests
   (`crates/vkxp-binding/tests/graphics_rgss2.rs`) stay as they are. "Ignores a freeze" and
   "the brightness at the call" hold already: the freeze is the host's and
   `Scene::snapshot` reads the scene now.
8. **The frozen picture.** The guest says by which rule a freeze keeps the picture, as it
   says the rule of a transition: `GuestToHost::Freeze` gets a field `rule` of a new enum
   with two values, the frame (what every freeze does today) and the buffer of RGSS3. The
   host does not know runtimes, and the rule belongs to the freeze, which is why it is not
   an option of the session. Alternatives considered: a session option set by the program
   (no protocol step, but the host would hold knowledge of the runtime that the transition
   rule already gets from the guest); the buffer for every runtime (rejected: it would
   change what tests of the presenter expect of a held frame of another size, and RGSS2
   was not measured).
   - The session passes the rule to `Display::hold`. Everything else of freeze and
     transition is unchanged: a freeze while frozen does nothing, the steps are those of
     the transition's rule (the binding of RGSS3 sends `TransitionRule::Rgss2`, rule 8), a
     packet of another size does not end a freeze.
   - The presenter keeps, beside the held frame it has, a buffer image of 640 by 480
     texels, made opaque black when first needed and kept for the life of the compositor.
     A hold by the buffer rule writes what the display shows of the screen, at the logical
     size of the newest frame, into the top left corner and leaves the other texels; the
     buffer is then the held picture until the next hold. While the buffer is the held
     picture and the newest frame fits into it, held texel (x, y) is mixed with newest
     texel (x, y) in integers before scaling. This is the path of `present.frag` for two
     frames of one grid, with the newest frame's size as the grid, so no shader change is
     expected.
   - Frames of RGSS3 never exceed the buffer (the binding limits `resize_screen` to 640 by
     480, as measured). The host does not rely on that: a hold by the buffer rule of a
     frame wider than 640 or higher than 480 is a hold by the frame rule, and a frame that
     large shown while the buffer is the held picture is shown as the presenter shows a
     held frame of another size today. Nothing is read outside an image.
   - Not measured, and decided here: while frozen after a resize the display shows the
     buffer's texels for the screen as it is now (the original's window shows stale
     contents until the next frame); a masked transition after a resize tiles the mask over
     the screen as it is now and takes the old picture from the buffer.
9. **The protocol version goes up by one** (13 to 14 when this was drafted; one above what
   the branch has if another change raised it first), by the rule of `vkxp_proto::codec`.
   An input recording of an earlier build is refused, as after every step; save states
   never meet another version. `README.md` and `docs/architecture.md` name the new version
   and what it added.
10. **Where the original faults, vkxp goes on.** Recorded in `docs/decisions.md` as known
    differences:
    - A disposed viewport given to a move: the scene refuses it and moves nothing
      (`SceneError::Disposed`), for every runtime, as today. What the binding of RGSS3
      raises from it is the binding's (the binding of RGSS2 raises `RGSSError`).
    - The sprite of rule 12 is drawn by the rules of a turned sprite; there is no picture of
      the original to compare with.
    - A negative `wave_length` takes the angle modulo 360 into the sine table, as the
      sprite of RGSS2 does in vkxp today; the original shows memory contents.
11. **Expected values without data of the original.** Two sources:
    - *Rules restated as expectations.* The scenarios give small scenes with every expected
      number written out. The numbers of the turned sprite, the moved plane, the resized
      screen, the sprite of rule 12 and the wave offset were computed with the model of the
      measurement and with the scene of vkxp as it is (for RGSS1, RGSS2 and a sprite
      without a viewport) and agree.
    - *The model's pictures as digests.* The topics `atvp`, `turnedvp`, `aplanevp` and
      `aheldx` are rebuilt as text scenes in the format of
      `vkxp_core::test_support::sprite_cases`, with the digest of the picture
      `ace-v1-sprite-model.py` draws of each frame, which is the frame of the original (0
      wrong on all four; `ace-v1-sprite-model-check.txt`). The input samples are the
      numbers of `ace-v1-sprite-input-samples.txt.gz` in the format of
      `rgss2-input-samples.txt`. The scripts that drive the model and convert the samples
      are not committed (they run files under `ref/`); they are kept under `local/`.
12. **XP and VX do not change.** No existing file under a `tests/data/` directory is
    edited and no expected value of an existing test. Proving tests: the scenes of RGSS2
    (`extract/rgss2/tests.rs` over `sprite-rgss2-*.txt`, `extract/scene_cases.rs` over
    `rgss2-scene-cases.txt`), the turned sprites of RGSS1 (`extract/tests.rs` over
    `sprite-rotation-samples.txt`, `sprite-angle-frames.txt` and
    `extract/samples.rs` over `sprite-geometry-samples.txt`), the input sequences
    (`input/tests.rs` over `rgss2-input-samples.txt`), `crates/vkxp-vk/src/tests/sprites.rs`
    and `presentation.rs`, the session tests and `crates/vkxp-host/tests/headless.rs`, and
    `crates/vkxp-binding/tests/graphics_rgss2.rs`. In addition the scenes of RGSS2 that the
    measurement found identical on RGSS3 are run with `Rules::Rgss3` against their
    committed digests, which proves the table of decision 1.

## Implementation notes

- The files of scenes that RGSS3 shares with RGSS2, by the measurement: all of
  `sprite-rgss2-place.txt`, `sprite-rgss2-bush.txt`, `sprite-rgss2-wave.txt` (topics
  `clip3`, `size2`, `bushbig`, `bushop`, `wave1` to `wave6`, byte-identical on RGSS3) and
  the topics `order`, `flash`, `plane`, `size2` and the nine committed frames of `viewport`
  of `rgss2-scene-cases.txt` (the one frame of `viewport` that differs on RGSS3 is frame 9,
  which that file leaves out). `sprite-rgss2-random.txt` is not among them: its scenes have
  turned sprites in viewports with origins.
- `test_support::sprite_cases` runs its scenes with `Rules::Rgss2` on 544 by 416. It gets a
  way to run a file with `Rules::Rgss3`, and what `aplanevp` needs and it lacks today (the
  viewport of a plane as a property that can be set).
- New files of `crates/vkxp-core/tests/data/`: `sprite-rgss3-turned.txt` (`atvp`, 8 frames;
  `turnedvp`, 4 frames), `sprite-rgss3-moved.txt` (`aplanevp`, 6 frames),
  `sprite-rgss3-random.txt` (`aheldx`, 20 scenes) and `rgss3-input-samples.txt`. The head of
  each says which topic and model made it and on which DLL versions the topic was measured.
  If a scene of `aheldx` shows a pixel that only a case of "Non-Goals" decides, it is left
  out and the head of the file names it; none is expected, since the model has no wrong
  pixel on these scenes. In `aheldx` sprite 581 of scene 17 is invisible, as in the
  measurement.
- The input samples: one line per update of the 3.0.1.1 run. The held set of an update is
  the logged `press?` (`p=`), not the key events, because of rule 11; the test receives it,
  marks a shown frame, updates, and compares `press?`, `trigger?`, `repeat?`, `dir4` and
  `dir8`.
- `Compositor::render` already gives the brightness of a packet; the snapshot function
  composes with a compositor of its own, as `snap_to_bitmap` does today.
- The freeze message is a literal in about twenty places of tests and in
  `crates/vkxp-host/src/demo.rs`; each gets the frame rule. In `crates/vkxp-binding` the
  only edits are the literal in `freeze` and the body of `snap_to_bitmap`; `Runtime::Rgss3`
  stays mapped to `Rules::Rgss2` there until the binding of RGSS3 (see "Follow-ups").
- `snap_to_bitmap` no longer names `Compositor`, so the `use` line of
  `crates/vkxp-binding/src/graphics.rs` that imported it imports the rule of the snapshot
  function instead, and the `use` line of `vkxp_proto` there names `FreezeRule` for the
  literal in `freeze`: two lines outside the two places named above, which the build needs
  (an unused import is an error of the gate). The tests of the binding that hold a literal
  of the freeze message import `FreezeRule` likewise.
- The test of `vkxp_proto::codec` that states the protocol version and lists the versions
  that are refused states 14 and lists 13 too, as after every step of the version.
- `RecordingDisplay` of `vkxp-host` keeps the rules of its holds in a list of their own
  (`holds`), so that the events the existing tests compare stay as they are.
- One test outside the scope of the change is corrected, because the gate failed on it
  in the default schedule of the tests (twice in the review):
  `matches_at_the_end_of_the_subject_are_as_before` of
  `crates/vkxp-ruby-sys/tests/backports.rs`. Its regular expression literals were compiled
  when the source was parsed, before its `$KCODE = 'U'` ran. The tests of that file share
  one interpreter, and when `Regexp.new('[\\777]', nil, 'n')` of the test beside it had
  raised just before, the engine was still set to the kcode of that pattern, so the
  literals were compiled for it and `'aSs' =~ /[\303\237]/i` gave nil instead of 1. The
  test now compiles its literals in an `eval` of their own after the assignment; every
  assertion and expected value is as it was, and no code of the interpreter changed.
- A second test outside the scope is corrected, because the gate failed on it twice
  (once for the author, once in the review): `a_reset_request_starts_the_scripts_over` of
  `crates/vkxp-binding/tests/run_game.rs` found one live bitmap where it expects none. The
  tests of that file share one interpreter. The harness ran the game in one call of the
  interpreter (`run_game` of `crates/vkxp-binding/src/testing/host.rs`), and the test
  counted the bitmaps in a second call (`testing::bitmap_count`), so the game of another
  test could run in between and leave its bitmap. `run_game` now counts the bitmaps in
  the call that ran the game and gives the number in `Played::bitmaps`, which the test
  compares with 0 as before. These are the test harness and one test of the binding; no
  class or module of the binding changed.
- Presenter: a hold in the middle of a cross-fade writes the mix as it is shown at the
  logical size of the newest frame, which the existing off-screen capture gives. After a
  state was loaded the buffer keeps its texels; the freeze has ended, as today.
  `forget_resources` leaves the buffer as it leaves the held frame.
- If a shader does change (none is expected: decisions 2 and 8), the Vulkan tests are run
  once more on the hardware device,
  `VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV cargo test -p vkxp-vk --locked`.
  Every other command that reaches Vulkan restricts the loader to llvmpipe.

## Follow-ups

None of these leaves a requirement of this change unmet. The first group is the binding of
RGSS3 (with the binding over Ruby 1.9.2), with the measured rule each item needs
(`ace-v1-sprite-REPORT.md`, 2026-10-13):

- The scene and the input state of a game of RGSS3 are made with `Rules::Rgss3`
  (`runtime.rs`), and every frame shown marks the input state (decision 5).
- `Input`: 20 constants, each the Symbol of its name. `press?`, `trigger?`, `repeat?` take
  exactly one argument (`ArgumentError` otherwise) and never raise for its class. True is
  possible for the Symbol and for the Integers 2, 4, 6, 8 (DOWN, LEFT, RIGHT, UP), 11 to 18
  (A, B, C, X, Y, Z, L, R), 21, 22, 23 (SHIFT, CTRL, ALT), 25 to 29 (F5 to F9). False for
  everything else: another Symbol (`:down`, `:Down`), a String, a Float (2.0, 2.9), 258,
  2**31, nil, true, false, an Array, an Object. The key map is that of VX.
- `Graphics.snap_to_bitmap` calls the snapshot function with the rule of RGSS3; an argument
  raises `ArgumentError`. `Graphics.freeze` sends the buffer rule. `resize_screen` as RGSS2
  (two Floats truncated, limits 1 to 640 and 1 to 480).
- `transition` without a freeze: 0 frames and no argument check. After a freeze: `()` 10
  frames; `(0)` and `(-3)` 0 frames and the freeze is over; `(nil)`, `(3, nil)`, `(3, 5)`
  raise `TypeError` and four arguments `ArgumentError`, and the freeze is kept;
  `(3, "missing")` raises `Errno::ENOENT` and the freeze is lost. `wait(2.9)`,
  `fadeout(2.9)`, `fadein(2.9)`, `transition(2.9)`: 2 frames.
- `Viewport.new()` creates the viewport of the screen; `(Rect)` copies; `(x, y, w, h)`
  truncates Floats and keeps negative sizes; other counts raise `ArgumentError`. No
  `disposed?`.
- `Plane#viewport` is a reader without an argument and `Plane#viewport=` a writer
  (`plane.viewport(nil)` raises `ArgumentError`); both take a viewport or nil, 5 raises
  `TypeError`. A disposed viewport given to `Sprite#viewport=` or `Plane#viewport=`: the
  original ends; what vkxp raises is decided there (RGSS2: `RGSSError: disposed viewport`).
  `Sprite.new(disposed viewport)` does not fail.
- The other argument forms and errors of sections 1, 3 and 5 of the report.

Not the binding:

- A masked transition after a resize, and what the window shows while frozen after a
  resize, are decided in decision 8 without a measurement.
- Found by the gate of this change, in tests of the interpreters and not caused by it:
  - Ruby 1.8.7 leaves the encoding of its regular expression engine at the fixed kcode of
    a pattern whose compilation raises (`rb_reg_initialize` in `re.c` raises between
    `rb_kcode_set_option` and `rb_kcode_reset_option`), until `$KCODE` is assigned again.
    Whether the original runtime does the same, and whether a game can meet it, was not
    examined; the test that depended on it was corrected (see "Implementation notes").
  - `crates/vkxp-ruby192-sys/tests/hash_seed.rs` fails when the tests run with
    `RUST_TEST_THREADS=1`: the child then prints its values behind the name of the test
    on one line, and the parent looks for the marker at the start of a line. The gate
    does not run the tests that way, so the test is left as it is.
  - `testing::bitmap_count` of `vkxp-binding` counts in a call of the interpreter of its
    own, so a test that runs beside others of its binary can see their bitmaps. The one
    test the gate failed on no longer uses it (see "Implementation notes"); its other
    callers (`tests/modules.rs`, `tests/ruby.rs`, `tests/stress.rs`) were not examined
    and did not fail in any gate run of this change.
- Whether RGSS2 keeps its frozen picture in such a buffer is not measured; a game of RGSS2
  that freezes and then resizes shows the held frame in its own letterbox rectangle.
- A wave on a turned sprite and a wave with a stretched cut source rectangle have no rule on
  RGSS3 either (same pixels as RGSS2 in the one frame measured).
- The fault of the sprite of rule 12 was not tried on RGSS2 or on Windows.
- The window keeps the size it has when the game resizes its screen; the frame is scaled
  into it (follow-up of `resize-the-screen-in-rgss2`).
