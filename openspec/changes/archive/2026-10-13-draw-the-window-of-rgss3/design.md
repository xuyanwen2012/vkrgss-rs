## Context

- The window of RGSS2 (change `draw-the-window-of-rgss2`, archived) is in
  `crates/vkxp-core/src/window/rgss2.rs` beside the window of RGSS1 in `window.rs`. A window
  of RGSS2 is told from one of RGSS1 by `Window::rgss2`, a `Some(State)` that
  `Scene::create_window_rgss2` sets. Background, frame and cursor are bitmaps of the arena
  that the window builds at certain calls (`Caches`); extraction draws them, the contents
  and the pieces of the skin as `Op::Quad`: in `BlendMode::Window` when open, as quads
  stretched over the shown rows in the normal blend while opening. There is no window op in
  the protocol and no window code in a compositor.
- What a script calls on a window reaches the scene as `Window::call(bitmaps, WindowCall)`;
  the state the original keeps between calls (contents shown, arrows, built bitmaps) lives
  there. The binding (`crates/vkxp-binding/src/window.rs`) only translates.
- The `Rules` of a scene have no value for RGSS3; `vkxp-binding` maps `Runtime::Rgss3` to
  `Rules::Rgss2`. The changes in flight `draw-the-tilemap-of-rgss3` and
  `draw-text-and-bitmaps-as-rgss3` both add a rule set to the element or the call and leave
  `Rules` alone.
- The measurement of RGSS3, seen from a clone under `wt/`:
  `../../ref/tools/oracle/measurements/ace-v1-window-REPORT.md`, its executable model
  `ace-v1-window-model.py`, the generator `ace-v1-window-gen.py` (over
  `vx_v1_window_gen.py`), the case files `ace-v1-window-a1-cases.py` to `a16` (there is no
  a10), the random scenes `ace-v1-window-ha-cases.py`, and the logs. The report is the
  authority for every rule below, and the model where the prose of the report can be read
  in two ways. The measurement of RGSS2 it builds on is `vx-v1-window-REPORT.md`; "as
  RGSS2" below means the rule of that report, which `window/rgss2.rs` implements.
- No script of RGSS3 can run yet: the interpreter and the guest of RGSS3 are other changes.

### Measured rules (RGSS3, versions 3.0.0.1 and 3.0.1.1 identical)

x, y, W, H are the window's position and size, pad its `padding`, padb its
`padding_bottom`; `wblend` is the window formula and `sblend` the normal formula of
`frame-composition`; a half is a division by 2 that rounds towards zero.

1. Values. A new window has z 100, back opacity 192, padding 8, bottom padding 8, arrows
   allowed, tone (0, 0, 0, 0), openness 255; all else as RGSS2. `padding =` sets both
   paddings; `padding_bottom =` the lower one alone; neither is clamped (-5 and 100000 are
   kept).
2. Skin layout as RGSS2, except the cursor: block (64, 64, 32, 32) with a border of 4, centre
   (68, 68, 24, 24).
3. Background: built as RGSS2 and at the same calls. When drawn: first the tone on every
   pixel of the built bitmap (components truncated towards zero; when grey is not 0,
   `l = (77 r + 150 g + 29 b) >> 8` and each channel `sblend(v, l, grey)`; then
   `min(255, max(0, v + t))`; alpha kept), then the alpha of RGSS2
   `(a * ((back_opacity * opacity) / 255) + 127) / 255`, then `sblend` for every pixel at any
   openness. The tone is read when the window is drawn; assigning it builds nothing.
4. Frame and openness as RGSS2: `wblend` with the odd-column rule at openness 255, `sblend`
   below; rows `n = trunc(H * s)`, `top = trunc(H * (1 - s) / 2)` in double precision.
   Nothing but background and frame is drawn below 255.
5. Contents: pixel (u, v) at (pad + u - ox, pad + v - oy), inside
   (pad .. W - pad, pad .. H - padb) cut by the window's rectangle. Alpha and blend as RGSS2.
6. Cursor bitmap: as RGSS2 with 4 for 2 and 24 for 28. Centre to (4, 4, cw - 8, ch - 8) when
   both are above 0: bilinear with numerator 23 when both are above 12 and the size is not
   24 by 24, else nearest; top and bottom edge when cw - 8 > 0, left and right edge when
   ch - 8 > 0, each stretched by the nearest rule; then the four corners of 4 by 4 with
   `blt`. Built when another skin is assigned and when the size of `cursor_rect` changes.
   Blink and opacity as RGSS2.
7. Cursor position: `px = pad + |cursor_rect.x - ox|`, `py = pad + |cursor_rect.y - oy|`.
   The cursor is drawn whole from (px, py).
8. Cursor placement is state. (px, py) and the visible part
   `(px, py, px + cw, py + ch)` cut by `(pad, pad, W - pad, H - padb)` cut by the window's
   rectangle are computed from the values of the moment at: `cursor_rect =` and every
   change through the methods of the `Rect`, `ox =`, `oy =`, `padding =`,
   `padding_bottom =` (each also with an equal value), and `windowskin =` with another
   object. At no other call (not `width =`, `height =`, `move`, `update`, `x =`, `y =`).
   When drawn, the kept part is cut by the window's rectangle of that time. The kept values
   are relative to the window.
9. Pause sign as RGSS2 at (half(W) - 8, H - 16), cut by the window's rectangle.
10. Scroll arrows at down (half(W) - 8, H - 12), left (4, half(H) - 8), right
    (W - 12, half(H) - 8), up (half(W) - 8, 4). Conditions with contents of cw by ch: down
    `ch - oy > H - pad - padb`, left `ox > 0`, right `cw - ox > W - 2 * pad`, up `oy > 0`;
    none while `arrows_visible` is false. Decided, and bound to the skin of that moment,
    only at: `width =`, `height =`, `ox =`, `oy =`, `arrows_visible =` (always), `move`,
    and `windowskin =`, `contents =` with another object; and only when the contents are a
    live bitmap at that moment, else the list and its skin stay. Not at `update`, `x =`,
    `y =`, `active =`, `pause =`, `cursor_rect =`, `contents_opacity =`, `openness =`,
    `padding =`, `padding_bottom =`.
11. Contents shown := `visible`, by `update` and by: `width`, `height`, `ox`, `oy`,
    `contents_opacity`, `openness`, `padding`, `padding_bottom` (always); `x`, `y`,
    `active`, `pause`, `windowskin`, `contents`, `cursor_rect` (when the value changes).
    As RGSS2 plus the two paddings.
12. Order inside a window: background, frame, then at openness 255: cursor, arrows (down,
    left, right, up), pause sign, contents.
13. `move(x, y, w, h)` is `x =`, `y =`, `width =`, `height =` in this order.
14. Width or height of 0 or less: no background, no frame. The rectangle that cuts cursor,
    arrows, pause sign and contents, and that is used when the cursor is placed, is then the
    one pixel at the window's origin. Fitted to 9 pixels, 5 more in the held-out sets.
15. Skins smaller than the layout. Pixels outside the skin are transparent for everything
    copied (frame pieces, cursor corners, pattern, arrows, pause sign). A stretched block
    (background, cursor centre, cursor edges) that leaves the skin is cut and its target
    shortened, each axis by itself, with S the side of the block, A what the skin has of it
    (A < S) and D the target: a copy (target size equal to block size on both axes) draws A
    columns; the nearest stretch the columns i with `(i + 1) * S / D < A`, that is
    `ceil(A * D / S) - 1`; the bilinear stretch those with `(i + 1) * (S - 1) / D < A - 1`,
    that is `ceil((A - 1) * D / (S - 1)) - 1`; exact arithmetic, clamped to 0 .. D. Which of
    the three applies is decided from the full sizes. The drawn columns take the pixels
    they would take from a full skin. A part of the skin drawn on the screen (arrow, pause
    sign) that the skin cuts counts its cut width in the odd-column rule.
16. A disposed skin or contents bitmap raises nothing and is drawn as nil: with a disposed
    skin nothing of the skin is drawn, the built background, frame and cursor included;
    disposed contents are not drawn and count as nil when the arrows are decided.
17. Page faults of the original: placing the cursor (`ox =`, `oy =`, `padding =`,
    `padding_bottom =`, a `cursor_rect` assignment that leaves the size 0 by 0) in a window
    on which `windowskin =` was never called and whose `cursor_rect` never changed its
    size; a window of 20000 by 20000 with a skin, a `cursor_rect` of 100000 by 100000, a
    window of 100000 by 100000.

Not measured (the report's list): sizes above 600 and origins beyond 1000; text in the
contents; `flash` of a viewport over a window; a reset with windows alive; whether `update`
or the paddings can take an arrow away.

### What the captures of RGSS2 decide of rule 15

Checked while this change was drafted, with the model of RGSS2 and the captures of RGSS2
under `vkxp/target/vx-v1/window/` (no Wine):

- With the rule of the stretched block applied to the **background** alone, the cases g8
  and g6 of RGSS2 go from 9525 wrong pixels to 0 (17 captures; the report has the same on
  the second DLL of RGSS2).
- The same rule applied to the cursor's centre and edges, and the cut width in the
  odd-column rule, change no pixel of those captures: RGSS2 was not measured with a skin
  that cuts the cursor block or an arrow at an odd width.
- Of the 147 frames in `window-rgss2-cases.txt` and `window-rgss2-random.txt`, the corrected
  model changes exactly two, and gives for both the digest of the capture of the original:
  `skin_sizes_t` of the scene `g6-skin_sizes_t` (now `51c81ac0bdc87e36`, measured
  `edbddf94f4992ce6`) and `smallskin_t_0` of `g8-smallskin_t_0` (now `6e2b1b9df573caf0`,
  measured `0263b5710a3a9da6`). The head of the file names these two as not measured.

## Goals / Non-Goals

**Goals:**

- A window of RGSS3 is drawn pixel for pixel as the model of the measurement draws it, by
  the CPU reference and by the Vulkan backend alike, for every case the model covers.
- What the original keeps between calls is state of `vkxp-core`, driven by `Window::call`,
  and tested here.
- The background of a window of RGSS2 from a skin smaller than the layout is drawn as the
  original of VX draws it.
- XP is unchanged; VX is unchanged but for the two pictures named above.

**Non-Goals:**

- The class `Window` of RGSS3 in the binding, the interpreter, and anything of
  `crates/vkxp-binding`.
- A value of `Rules` for RGSS3.
- Text in the contents; what the report lists as not measured.
- For RGSS2: the cursor from a skin that cuts the cursor block, the cut width in the
  odd-column rule, the list of calls that decide the arrows, disposed bitmaps. None of
  them was measured on RGSS2, and no frame of RGSS2 changes for them.
- A change to the protocol, to `vkxp-compose-ref` or to the shaders.

## Decisions

1. **A third rule set of the window, chosen at creation.** `Scene::create_window_rgss3`
   creates a window that follows the rules above; `create_window` and `create_window_rgss2`
   stay. The rule set is a property of the window, not of the scene: a window of RGSS3
   lives in a scene of `Rules::Rgss2`, which is what the binding gives a game of RGSS3
   today, and whose order of elements (z, then 0 for a window and y for a sprite, then
   creation) is the measured one for RGSS3 too. The code of RGSS3 extends that of RGSS2
   where the rules are the same (background and frame builders, openness, blink, pause
   sign, the refresh) and stands beside it where they differ; how the state is laid out
   (one `State` with a rule set, or a second one) is left to the implementation. A public
   reader tells the three kinds apart.
2. **The state machine lives in `vkxp-core`.** `WindowCall` gets `Padding(i32)`,
   `PaddingBottom(i32)`, `ArrowsVisible(bool)` and a call for `move` with its four numbers.
   On a window of RGSS3, `Window::call` does what rules 8, 10, 11, 13 and the building of
   rule 6 say for each call; a call that assigns an equal value is still a call. `tone`
   keeps nothing and is a plain setter like `opacity`; the window has readers for the
   paddings, `arrows_visible` and the tone. A window of RGSS1 or RGSS2 ignores the three
   new properties, as a window of RGSS1 ignores `Openness`, and takes `move` as its four
   assignments. Alternative considered: the lists of calls in the binding; rejected,
   because they could then not be tested before the interpreter exists and the binding
   would imitate the original instead of the scene.
3. **No protocol change, no compositor change.** Every rule is said with `Op::Quad`:
   - the background is a quad of the built bitmap in `BlendMode::Normal` with the window's
     tone in the quad's `tone` (`Tone::to_proto` truncates towards zero; `apply_tone` of
     `vkxp-pixel` is the formula of rule 3: grey by the normal formula, then the clamped
     offsets, alpha kept), stretched over the shown rows while opening as in RGSS2;
   - frame, cursor, contents, arrows and pause sign are quads in `BlendMode::Window` as in
     RGSS2, whose odd-column rule (`Quad::is_window_tail`) counts the columns of the quad's
     source inside its clip, which is the count of the model;
   - the kept part of the cursor, the padding rectangle and the one pixel of rule 14 are
     clip rectangles.
   `PROTOCOL_VERSION` is not touched (the change `draw-the-tilemap-of-rgss3` raises it;
   this one does not meet it). Should a rule turn out not to be sayable with the ops that
   exist, the case is recorded under "Follow-ups", the scenes that need it are left out of
   the fixtures and named there, and no op, field or shader is changed in this change.
4. **Rule 15 for both runtimes where it is measured for both.** The builder of the
   background is one function for RGSS2 and RGSS3; it gets the rule of the shortened
   target for both. This is the one place where a frame of RGSS2 changes, and only for a
   skin narrower or lower than 64 pixels. The cursor's centre and edges are shortened, and
   a cut arrow or pause sign counts its cut width, for RGSS3 alone; RGSS2 keeps what it
   does today for both (the skin padded to the layout for the cursor, and
   `Assembly::skin_part` for the parts), because the captures of RGSS2 do not decide them.
   RGSS1 is not touched: its layout is another and its small skins were never measured.
5. **The arrows of RGSS2 stay as they are.** The report infers that the list of calls of
   rule 10 would serve RGSS2 too; nothing forces the two runtimes to share it (the refresh
   of RGSS2 decides the arrows, the one of RGSS3 does not), so RGSS2 keeps deciding them in
   every refresh and no frame of RGSS2 can change by it.
6. **Where the original faults, the scene goes on** (rule 17). The calls take effect by the
   rules: placing the cursor in a window that never had a skin places it (nothing is drawn
   without a skin); a window or a cursor too large for a bitmap of the arena
   (`MAX_BITMAP_PIXELS`) has no built background, frame or cursor, as in RGSS2 today, and
   its other parts are drawn. Known difference: the original ends, vkxp does not.
7. **Disposed bitmaps** follow rule 16 for RGSS3. For RGSS2 nothing changes (not measured
   there).
8. **Expected values without data of the original.** Two sources, both independent of the
   code of this repository:
   - *Rules restated as expectations.* The scenarios of the delta spec give small scenes
     with the pixels or pixel counts they must show. Every number in them was produced by
     `ace-v1-window-model.py` (for the windows of RGSS2 by `vx-v1-window-model.py` with the
     stretch functions of the model of RGSS3) while this change was drafted, and can be
     followed by hand from the rules. The tests build these scenes with generated skins
     (below) and compare pixels through the CPU reference.
   - *Digests of the model.* Scenes of the case files and of the random generator are
     committed as text in the format of `vkxp_core::test_support::window_cases`, each frame
     with the FNV-1a digest of the picture the model draws, as was done for RGSS2:
     `crates/vkxp-core/tests/data/window-rgss3-cases.txt` with at least one scene of each
     case file a1 to a9 and a11 to a16, and `window-rgss3-random.txt` with thirty scenes of
     the set of seed 3 (`H_SEED=3`), which was held out when the model was fitted. The
     digests are made by `Scene.run` of the model, driven by a script of the flow's own
     that stays under `local/` and is not committed. Running that Python model is allowed;
     running Wine is not.
   A scene is left out of a fixture only when it uses a bitmap from a file or something no
   rule of this change covers (a tone or colour of a viewport), or when it needs an op that
   does not exist (decision 3); the random file takes the first thirty scenes of the set
   that are not left out. A case file without a scene that can be kept is named in the head
   of the fixture with the reason. The case `rtp` reads a skin of the run-time package and
   is not rebuilt. Only rules, numbers and digests of frames of generated bitmaps enter the
   repository.
9. **XP and VX stay.** No expected value of an existing test is edited, with one exception:
   the two `frame` lines of `window-rgss2-cases.txt` named above get the measured digests,
   and the head of that file no longer calls them not measured. `window-rgss2-random.txt`,
   `window-rgss2-samples.txt` and `window-samples.txt` stay byte for byte. The proving
   tests: all of `crates/vkxp-core/src/window/tests.rs` (RGSS1) and
   `crates/vkxp-core/src/window/rgss2/tests.rs`, the two tests of
   `crates/vkxp-vk/src/tests/windows.rs`, the tests of `crates/vkxp-compose-ref` with
   `tests/expected/` unchanged, and `crates/vkxp-binding/tests/scene_rgss2.rs`.
10. **Where the decisions are recorded.** `docs/decisions.md`, section "VX Ace": decision 6
    with its known difference, the few pixels behind rule 14, and what stays unmeasured for
    RGSS2 (decisions 4, 5, 7). `docs/compatibility.md`, section "The window of VX" only: the
    background from a small skin is measured now; the cursor from a skin that cuts its
    block is not. Nothing there says that a game of VX Ace runs.

### Scenes of the scenarios

The scenarios of `core-scene` use these words.

- *The readable skin*: 128 by 128, pixel (x, y) is (x, y, 100, 255), except the pattern
  quadrant (x < 64, y >= 64), which is transparent. A pixel on the screen then names the
  pixel of the skin it came from.
- *The piece skin*: 128 by 128, one opaque colour for each piece and transparent elsewhere:
  the background block (0, 0, 64, 64) is (0, 0, 200); the frame (64, 0, 64, 64) one colour
  but for its centre (80, 16, 32, 32), which is transparent but for the four arrows, each of
  a colour of its own (up (88, 16, 16, 8), down (88, 40, 16, 8), left (80, 24, 8, 16), right
  (104, 24, 8, 16)); the cursor block (64, 64, 32, 32) white; the four pause images
  (96, 64), (112, 64), (96, 80), (112, 80), 16 by 16, a colour each. The skin of
  `crates/vkxp-core/src/window/rgss2/tests.rs` is of this kind; the colour of the background
  block matters to the scenario "Opening" alone.
- *A flat bitmap*: every pixel one opaque colour. "Opaque contents" are a flat bitmap.
- Unless a scenario says otherwise: the window is of RGSS3, at (0, 0), with the defaults of
  rule 1, a contents bitmap of 1 by 1 that is transparent, nothing else in the scene, and
  the frame is black where nothing is drawn. Rectangles of pixels are written
  (left .. right, top .. bottom) with the right and bottom excluded. Calls are written as
  the assignments of a script.
- The arithmetic of the first background scenario, as an example of how the numbers
  follow: screen (30, 40) is background pixel (28, 38), a copy of skin pixel (28, 38) =
  (28, 38, 100); alpha `(255 * 192 + 127) / 255 = 192`; over (200, 200, 200):
  `200 + (((28 - 200) * 193) >> 8) = 70`, 77 and 124. (`wblend` would give 71.)

## Implementation notes

- The refresh of RGSS2 (`Window::latch`) sets "contents shown" and decides the arrows in
  one step. For RGSS3 these are two steps with two lists of calls (rules 10 and 11), and a
  third for the placement (rule 8). `update` only does the first, and advances blink and
  pause sign as in RGSS2.
- Order within one call, as `Scene.set` of the model does it: store the value (`padding`
  stores both paddings), the refresh if the call is on its list, the arrows if on theirs,
  the building of background and frame or of the cursor, the placement of the cursor.
  "Another object" compares bitmap ids, whether the bitmap is alive or not.
- `windowskin = nil` makes the arrows vanish because they are decided anew and bound to no
  skin. When the contents are nil or disposed at that moment they are not decided, and the
  arrows go on being drawn from the skin they were bound to (`d_arrows` of the model reads
  `arrow_skin`, not the window's skin). The prose of the report does not say this; the
  model decides.
- The cursor's border and block side become parameters of `cursor_pieces`; RGSS1 and RGSS2
  pass what they have today.
- Rule 15 in the builders: the shortened target can be had by building the stretch as
  today and keeping its first columns and rows, since "the drawn columns take the pixels
  they would take from a full skin". The background's pattern is laid afterwards over the
  whole target as today (it changes only pixels whose background alpha is above 0).
- In RGSS3 a part of the skin on the screen is a plain `Assembly::quad`: it cuts the source
  to the bitmap, and the tail rule then counts the cut width, which is rule 15.
  `Assembly::skin_part` stays for RGSS2.
- The halves of rules 9 and 10 round towards zero: `/` of Rust, not `div_euclid`.
- A window of RGSS3 without area is not skipped by extraction: its front parts are drawn
  with the one-pixel rectangle. The placement of the cursor uses the same rectangle.
- A disposed skin is noticed when the frame is extracted (`Bitmaps::is_disposed`), not
  only at a call.
- The format of `test_support::window_cases` gets what the scenes of RGSS3 need: a window
  of RGSS3 with or without its four numbers, `padding`, `padding_bottom`, `arrows_visible`,
  `tone`, `move`, the change of `cursor_rect.x` alone, the disposal of a bitmap, and any
  pixel function a case file uses that the format lacks (written from the case file's
  formula). Scenes of RGSS3 run in a scene of `Rules::Rgss2`; how the files of RGSS2 are
  read does not change. A window of the runner gets contents of 1 by 1, as the binding
  will give it.
- The exporter of the RGSS2 fixtures may still lie at
  `../draw-the-window-of-rgss2/local/export_window_cases.py` (outside the repository); it
  shows the digest (width and height as little-endian 32-bit numbers, then the pixels as
  red, green, blue, 255) and how a scene is cut at `clear`. It is a starting point, not a
  dependency: it knows neither the model of RGSS3 nor its statements, and it leaves out
  scenes for the y key of sprites, which the scenes of RGSS3 need not.
- `crates/vkxp-vk/src/tests/windows.rs` gets one test per new file, through the `compare`
  that is there. No test needs the AMD device: no shader and no record changes.
- Edits expected to meet the changes in flight, each a few lines:
  `crates/vkxp-core/src/lib.rs` (exports and the module table; both siblings edit it),
  `crates/vkxp-core/src/scene.rs` and `element.rs` (the tilemap change adds
  `create_tilemap_rgss3` and a field of `TilemapRgss2` near the window's code),
  `docs/decisions.md` (all three append to "VX Ace"). The protocol version is not among
  them.

## As built

- One `State` (`window/rgss2.rs`) serves both rule sets; what RGSS3 keeps beyond it
  (paddings, `arrows_visible`, tone, the placed cursor) is an optional part of it in
  `window/rgss3.rs`. No file outside the scope was touched: extraction (`extract.rs`) asks
  `Window::is_rgss2` whether a window is one element, so that reader stays true for a window
  of RGSS3 as well, and `Window::is_rgss3` tells the two apart.
- Rule 15 needed a new cut for the bilinear stretch alone. `Bitmaps::stretch_blt` already
  cuts a source rectangle that leaves its bitmap by the same count (measured for RGSS1 and
  RGSS2 bitmaps), and a block transfer draws what the source has; the builders that follow
  rule 15 therefore stretch from the skin itself and not from its copy padded to the layout.
- The fixtures hold the first and the last scene of each case file (28 scenes) and the first
  thirty scenes of the set of seed 3. While this change was built, all 125 scenes of the
  case files and all 60 of that set were composed by the CPU reference and had the digests
  of the model; no scene had to be left out and no rule needed an op that does not exist.

## Follow-ups

None of these leaves a requirement of this change unmet.

- The binding of RGSS3 (with the binding over Ruby 1.9.2), each with its measured rule:
  - `Window.new(x = 0, y = 0, width = 0, height = 0)`: 0 to 4 Integers, truncated, negative
    values kept; a fifth argument raises `ArgumentError`, a Viewport `TypeError`; there is
    no viewport argument. It calls `Scene::create_window_rgss3` and then the call of
    `move`: with no skin yet nothing is built, placed or shown by it. The window gets a
    contents bitmap of 1 by 1 of its own. The defaults are those of the scene.
  - `viewport=` as in RGSS2 (`Scene::set_window_viewport`); it keeps nothing.
  - `padding`, `padding=` (sets both), `padding_bottom`, `padding_bottom=`: truncated, not
    clamped; `arrows_visible`, `arrows_visible=` (Ruby truth); `move(x, y, w, h)` with
    exactly four arguments, returning the window; `open?` is `openness == 255`, `close?`
    is `openness == 0`.
  - `tone`, `tone=`: a Tone, anything else raises `TypeError`; the values are copied into
    the window's own Tone, every read returns that object, and a change through its
    methods is an assignment (an owner of the value object, as `cursor_rect` has one).
  - Every assignment reaches `Window::call`, also of an equal value, and every change of
    `cursor_rect` through the methods of its `Rect`, also to the same values: rules 8, 10
    and 11 count calls, not changes.
  - The calls of rule 17 are accepted; `dispose` and "disposed window" as RGSS2; the method
    list of RGSS3.
- A value of `Rules` for RGSS3 is still not needed: the window carries its own rule set.
- RGSS2, each needing a measurement on the original of VX first: the cursor and the cut
  width in the odd-column rule with a skin 65 to 95 pixels wide or high (rule 15 beyond the
  background); whether the arrows of RGSS2 are decided by the list of rule 10 (the report
  infers it from 373 captures that do not tell the two apart); disposed skin and contents.
- Rule 14 rests on 14 pixels and was not examined with readable bitmaps.
- What the report lists as not measured: sizes above 600, origins beyond 1000, a viewport's
  `flash` over a window, a reset with windows alive.
