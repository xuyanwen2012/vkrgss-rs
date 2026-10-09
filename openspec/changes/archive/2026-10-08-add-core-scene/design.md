# Design

## Context

See `proposal.md` for motivation. What this change starts from:

- `vkxp-core` has the value types (`Color`, `Tone`, `Rect`, `Table`, `Font`) and the `Bitmaps`
  arena: bitmaps are named by protocol `BitmapId`s, drawn into with `blt`, `stretch_blt`,
  `fill_rect` and friends, and `Bitmaps::deltas(&[BitmapId])` returns the `BitmapDelta`s for the
  bitmaps a frame refers to.
- Protocol version 2: a `FramePacket` has `frame_index`, `logical_size`, `frame_rate`,
  `bitmaps` and `ops`. `Op::Quad` draws a source rectangle through a fixed-point `Transform`
  that maps the centre of a screen pixel to a position in the source rectangle, with `tone`,
  `color`, `opacity` and `blend`; the source rectangle must lie inside the bitmap.
  `Op::RectEffect` applies a tone and a colour to what is already inside a rectangle. Every op
  has a clip rectangle. `Op::Plane` and `Op::TilemapLayer` are reserved and rejected by the
  compositors. `InputSnapshot` is a bit set of the twenty RGSS1 buttons.
- `vkxp-compose-ref` is a development-only dependency of `vkxp-core` and renders packets on
  the CPU.
- `vkxp-core` forbids `unsafe`, does no I/O and may only reach crates on its allow-list.

## Goals / Non-Goals

**Goals:**

- Everything the binding needs to implement `Sprite`, `Viewport`, `Window`, `Graphics` counters
  and `Input` as thin wrappers, enough to show and operate a title screen.
- Extraction that is deterministic on every platform.
- Tests that assert rendered pixels, so the internal op structure stays free to change.

**Non-Goals:**

- Drawing planes and tilemaps, and bush depth. They need protocol additions.
- Talking to the host: sending packets, waiting for ticks, freeze and transition messages.
- Matching the original runtime bit for bit. The placeholder list at the end names what must be
  checked against it later.

## Decisions

### A scene arena with typed handles

`Scene` owns all elements in arenas, one per kind, addressed by small copyable handles
(`SpriteId`, `ViewportId`, `WindowId`, `PlaneId`, `TilemapId`) that carry a generation, like
bitmap ids. Properties are read and written through the scene. The binding wraps each handle in
a Ruby object. A stale handle is a disposed element.

`Scene::extract(&mut self, bitmaps: &mut Bitmaps) -> FramePacket` is the only place a frame is
assembled.

### Draw order with stamps

Each element has `z` and a stamp from a counter that the scene increments when an element is
created and when its z is set to a different value. Top-level elements and viewports are sorted
by `(z, stamp)`; so are the elements inside each viewport. A window takes part twice: its frame
and background at `z`, its contents, cursor, pause sign and arrows at `z + 2`, both with the
window's stamp.

### Geometry is resolved to fixed point once, in core

For a sprite at screen position `(x, y)` (after adding the viewport's rectangle position and
subtracting its origin), origin `(ox, oy)`, zoom `(zx, zy)` and angle `a` in degrees, the map
from a screen point `(X, Y)` to a position in the source rectangle is

```text
u = ( cos a * (X - x) - sin a * (Y - y)) / zx + ox
v = ( sin a * (X - x) + cos a * (Y - y)) / zy + oy
```

and mirroring replaces `u` by `source width - u`. Core computes the six coefficients in `f64`
and rounds each to the nearest multiple of 2^-16 for the protocol's `Transform`. Sines and
cosines come from the pure-Rust `libm` crate so every platform gets the same bits; angles that
are multiples of 90 degrees use exact 0, 1 and -1. `libm` joins the allow-list of `vkxp-core`
in `xtask/src/deps.rs`: it has no dependencies, no I/O and no FFI.

The protocol requires the source rectangle to lie inside the bitmap, so extraction intersects
the sprite's source rectangle with the bitmap and moves the translation by the amount cut off
at the left and top (at the right, when mirrored). An empty result emits no op.

### Sprite and viewport effects

A sprite becomes one `Quad` with its opacity, blend mode, tone and effective colour. The
effective colour is the flash colour with its current alpha when a flash is running and that
alpha exceeds the alpha of the sprite's own colour, otherwise the sprite's colour. Flash alpha
after `k` updates of a flash of duration `d` and alpha `A` is `A * (d - k) / d`, rounded down.

A viewport emits its elements with the viewport rectangle (intersected with the screen) as
clip, then one `RectEffect` over that rectangle if its tone is not neutral or its effective
colour has a non-zero alpha.

### Windows are composed from cached bitmaps

Rather than emitting dozens of quads per window every frame, the scene keeps per window up to
three bitmaps of its own in the bitmap arena and redraws them with the arena's operations only
when what they depend on changes:

- background: the skin's (0, 0, 128, 128) pattern stretched or tiled to `width - 4` by
  `height - 4`; depends on skin, size and the stretch flag;
- frame: `width` by `height`, built from the skin's frame area at (128, 0, 64, 64): its four
  16 by 16 corners, and its edges from the 32-pixel-long middle pieces repeated along each
  side; depends on skin and size;
- cursor: the skin's (128, 64, 32, 32) image stretched to the cursor rectangle's size keeping a
  2-pixel border unscaled; depends on skin and cursor size.

Per frame a window then emits: background quad (opacity `opacity * back_opacity / 255`) and
frame quad (`opacity`) at z; at z + 2 the cursor quad (cursor alpha times contents opacity over
255), the contents quad (clip: the area 16 pixels inside the window, intersected with the
window's viewport clip), the pause sign quad (one of the four 16 by 16 images at (160, 64),
advancing every 16 updates) and arrow quads from the frame area's centre: up (152, 16, 16, 8),
down (152, 40, 16, 8), left (144, 24, 8, 16), right (168, 24, 8, 16), centred on the matching
edge. Skin changes are noticed through the arena's change tracking, so a script that draws into
its skin bitmap gets fresh caches. Cached bitmaps are disposed with the window.

Alternative considered: one quad per frame piece and per background tile. Rejected: many more
ops per frame, and the stretch and tile logic would have to exist a second time next to the
bitmap operations that already implement it.

### Input state

`InputState::receive(snapshot)` stores the newest snapshot; `update()` latches it and advances,
per button, a counter of consecutive updates held. Triggered is counter 1; repeated is counter
1, or counter at least 16 with `(counter - 16) % 4 == 0`. For the direction queries the state
keeps the update number at which each direction was last triggered.

### Bookkeeping

`Scene` holds the frame count, the frame rate and the logical size given at construction.
`extract` increments the frame count and a separate, never-settable frame index for the packet.

### Tests look at pixels

Scenario tests build a scene, extract, feed the packets to `vkxp_compose_ref::Compositor` and
assert on pixels of the returned frame. The property test does the same with random operation
sequences and only requires that no packet is rejected.

## Risks / Trade-offs

- [A placeholder below differs from the original runtime] → Each is confined to one function
  and marked `PLACEHOLDER` in the code, as in the rest of the crate.
- [Rounding the transform to 16 fractional bits misplaces pixels of rotated or zoomed sprites]
  → Exactness is required only for multiples of 90 degrees and whole zooms here; the format can
  be widened with a protocol version when the original's loop is known.
- [Window caches go stale] → They are keyed on the arena's change tracking for the skin and on
  the window's size, stretch flag and cursor size; tests change each and compare pixels.
- [`libm` on the allow-list] → Pure Rust, no dependencies; recorded here as the reviewed
  decision the allow-list asks for.

Behaviour to confirm against the original runtime later (placeholders today):

1. The order among elements with equal z, in particular after a z change.
2. Rounding of rotated and zoomed sprite geometry.
3. Which of a sprite's colour and flash colour wins, and the flash decay.
4. Whether a disposed viewport hides or releases its elements.
5. The window skin layout details: frame edge repetition, the 2-pixel cursor border, the pause
   sign's animation speed, arrow positions and when arrows appear.
6. The cursor blink values and what an inactive window shows.
7. Sampling when the background is stretched.
8. The repeat timing and the direction priority of `Input`.
