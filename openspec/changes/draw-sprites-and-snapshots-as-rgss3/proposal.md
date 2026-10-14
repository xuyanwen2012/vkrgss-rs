## Why

The title screen, the menu and the map of a game of VX Ace are made of sprites, planes and
viewports, and its scenes change through freezes, transitions and snapshots. These classes
and `Graphics` and `Input` of RGSS3 were measured on the original runtime on 2026-10-13
(`ace-v1-sprite-REPORT.md` with its model and frames under `ref/tools/oracle/measurements/`,
outside the repository), on both DLL versions (3.0.0.1 and 3.0.1.1). Almost everything is
RGSS2: 275 of the 327 frames of the probes of VX are byte-identical on RGSS3, and so are
transitions, fades, the draw order and the model of `Input`. A few rules differ or are new.
The interpreter of RGSS3 (Ruby 1.9.2) and its binding are being built by other changes, so no
script of RGSS3 runs yet. This change builds those few rules in the parts that do not need
the interpreter, so that the binding later only hands the data over.

## What Changes

- The `Rules` of a scene and of an input state get a third value, RGSS3. It equals RGSS2
  everywhere except in the rules below.
- A turned sprite in a viewport: with the rules of RGSS3 the viewport's rectangle position
  less its origin is added to the sprite's position first, and bounding box and texel walk
  both use that position. RGSS1 and RGSS2 keep the shift between the two.
- A viewport can be made "of the screen": it takes the size of the screen at that moment.
  Tests state what was measured of `resize_screen`: no viewport's rectangle changes and
  elements without a viewport use the new size.
- A plane moved to another viewport: stated and tested as measured (the other clip and
  origin, the creation rank kept). The code does this today; its mark "not measured" goes.
- Input with the rules of RGSS3: the timing of RGSS2, and a snapshot is seen by an update
  only after a frame was shown since it arrived, as the original takes key messages only in
  the calls of `Graphics` that show frames.
- Snapshots: the reference compositor gets a function that makes the pixels of
  `Graphics.snap_to_bitmap` from a packet by the rule of RGSS2 (alpha 255) or of RGSS3
  (alpha 255, and all of (0, 0, 0, 0) at brightness 0). The binding of RGSS2 uses it.
- The frozen picture of RGSS3: a freeze message says by which rule the picture is kept. By
  the rule of RGSS3 the display keeps one buffer of 640 by 480 pixels, black at the start;
  a freeze writes the screen into its top left corner and leaves the rest; a transition
  mixes buffer pixel (x, y) with pixel (x, y) of the screen as it is now.
- **BREAKING** for encoded streams: the protocol version goes up by one, so an input
  recording written by an earlier build is refused, as after every such step.
- Where the original faults or reads outside a table (a disposed viewport given to
  `viewport=`, one sprite that faults inside `Graphics.update`, a negative `wave_length`),
  vkxp goes on; what it does is tested and recorded as known differences, and so is what
  cannot be reproduced of the timing of key messages.
- XP and VX are drawn, shown and read exactly as before: no expected value of an existing
  test changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: the rules of RGSS3 in a scene; turned sprites in viewports; the viewport of
  the screen and a change of the screen's size; elements moved between viewports; whole
  scenes of the measurement; the cases at which the original faults.
- `core-input`: the rules of RGSS3; when a snapshot is seen.
- `frame-composition`: the pixels of a snapshot.
- `guest-host-protocol`: the rule of a freeze message; the version.
- `host-session`: a freeze by the rule of RGSS3.
- `vulkan-presentation`: the frozen picture in the buffer of RGSS3.
- `supported-runtimes`: the recorded decisions and known differences.

## Impact

- `crates/vkxp-core`: `scene.rs` (`Rules`, the viewport of the screen, the documentation of
  moving), `turned.rs`, `extract.rs`, `element.rs`, `plane.rs`, `input.rs`, `lib.rs`,
  `test_support/sprite_cases.rs`, tests, and new files under `tests/data/`.
- `crates/vkxp-compose-ref`: a function for snapshots and its tests. No change to how a
  packet is composed.
- `crates/vkxp-proto`: `message.rs`, `codec.rs`, `lib.rs`, and the literals of the freeze
  message in its tests.
- `crates/vkxp-host`: `display.rs`, `session.rs`, `headless.rs`, `windowed.rs`,
  `testing.rs`, `demo.rs`, tests.
- `crates/vkxp-vk`: the presenter (`src/compositor/present/`), `src/tests/presentation.rs`
  and `src/tests/sprites.rs`. No change to the compute shader is expected, and none to the
  fragment shader of the presenter.
- `crates/vkxp-binding`: `graphics.rs` only, two places: the literal of the freeze message
  and the body of `snap_to_bitmap` of RGSS2. `crates/vkxp` and other crates: literals of the
  freeze message in tests.
- `README.md` and `docs/architecture.md` where they name the protocol version;
  `docs/decisions.md`, section "VX Ace".
- Not touched: the classes and modules of RGSS3 in the binding and everything of an
  interpreter; the tilemap and the window; text; `docs/compatibility.md`, which says nothing
  of VX Ace until a game of it runs.
- Nothing of the original is committed: the bitmaps of the tests are made by formula, and
  from the measurement only rules, numbers and digests of frames of generated bitmaps are
  taken.
