# Design

## Context

See `proposal.md` for motivation. What this change starts from:

- `vkxp-core` contains only its crate documentation and `#![forbid(unsafe_code)]`. It may
  depend on `vkxp-proto`, `vkxp-pixel` and the pure crates on `CORE_ALLOW_LIST`
  (`xtask/src/deps.rs`); `thiserror` is on that list.
- `vkxp-proto` defines `BitmapId { index, generation }`, `BitmapDelta` (`Create`, `Update`,
  `Destroy`), the integer `Color`, `Tone` and `Rect`. Bitmap pixels cross the boundary as red,
  green, blue, straight alpha, four bytes per pixel in row order.
- `vkxp-pixel::blend_normal(dst, src, opacity)` is straight-alpha source-over with a correct
  result alpha, and `effective_alpha(alpha, opacity)` scales an alpha. Both are what a
  bitmap-to-bitmap transfer needs. `vkxp-pixel::provenance()` lists the compositing functions
  and a test in `vkxp-vk` requires a GLSL function of the same name for every entry.
- The reference compositor's `BitmapStore` applies `BitmapDelta`s and rejects a create into an
  occupied slot, an update outside the bitmap, and bitmaps above `MAX_BITMAP_PIXELS` (2^28).
- The serialized layouts in `specs/core-values` were checked on 2026-10-08 against the data
  files of a real RPG Maker XP project: tables of one, two and three dimensions (header plus
  16-bit elements), a colour and a tone (32 bytes each). No serialized rectangle occurs in that
  project; its layout is the commonly documented one.

## Goals / Non-Goals

**Goals:**

- Everything the Ruby binding needs to implement `Color`, `Tone`, `Rect`, `Table`, `Font` and
  `Bitmap` as thin wrappers.
- Bitmap pixels reach the host only as the minimal deltas for the bitmaps a frame uses.
- No dependency that would stop `vkxp-core` from being tested without Ruby, files or a GPU.

**Non-Goals:**

- Sprites, viewports, windows, planes, tilemaps and frame extraction.
- Decoding image files, finding fonts, rasterising glyphs.
- Matching the original runtime bit for bit. Formulas here are textbook and marked as such; the
  list at the end names what must later be checked against the original.

## Decisions

### Value types are plain Rust values

`Color` and `Tone` store `f64` components and clamp in their setters; `Rect` stores four `i32`;
`Table` stores dimensions and a `Vec<i16>`; `Font` stores names, size, flags and a `Color`.
None of them has identity: the binding wraps each in a Ruby object. Conversion to the integer
protocol types (`vkxp_proto::Color`, `Tone`, `Rect`) truncates toward zero; a core rectangle
with a non-positive width or height converts to an empty protocol rectangle.

`FontDefaults` is an ordinary value owned by whoever owns the runtime state, not a global, so
tests and a later second interpreter instance do not share it. The default family names are a
constructor argument because they depend on the game's language variant.

Serialized forms are `to_bytes` / `from_bytes` on each type, returning a typed error for
malformed input. The binding exposes them as `_dump` and `_load`.

### One arena owns all bitmaps; the handle is the protocol id

`Bitmaps` is an arena of slots. A slot holds the pixels (`Vec<u8>`, the protocol's layout, so
an upload is a copy), the bitmap's `Font`, the bounding rectangle changed since the last update
that was sent, and what the host knows about the slot. `BitmapId` from `vkxp-proto` is the
handle: `index` is the slot and `generation` increases every time a slot gets a new occupant,
so an id is never reused. An id whose slot is empty or holds another generation is a disposed
bitmap, which is exactly the error scripts must see.

Operations that involve two bitmaps (`blt`, `stretch_blt`) are methods of the arena because
they borrow two slots; a transfer within one bitmap reads from a temporary copy of the source
rectangle.

Alternative considered: reference-counted bitmap objects with interior mutability. Rejected
because the arena gives the delta logic one place that sees every bitmap, and avoids shared
mutable state in a crate that forbids `unsafe`.

### Deltas

`Bitmaps::deltas(referenced)` returns, in this order:

1. `Destroy` for every disposed bitmap the host still knows.
2. For each referenced bitmap the host does not know: `Create`, then `Update` of the whole
   bitmap.
3. For each referenced bitmap the host knows and that has a changed rectangle: `Update` of that
   rectangle.

Destroys come first so a create into a reused slot is never rejected. The changed region is one
bounding rectangle per bitmap: simple, and a frame that touches two distant corners of a large
bitmap is rare enough not to matter yet. Bitmaps that are not referenced are not uploaded and
keep their pending rectangle.

The creation limit is the reference compositor's `MAX_BITMAP_PIXELS`. `vkxp-core` cannot depend
on `vkxp-compose-ref`, so it defines the same number and a test (with the development-only
dependency) asserts the two are equal.

### Drawing arithmetic

- `set_pixel` and `fill_rect` replace all four channels.
- `blt` and `stretch_blt` call `vkxp_pixel::blend_normal(dst, src, opacity)` per pixel.
- `stretch_blt` maps destination column `i` to source column `floor(i * sw / w)` and rows
  likewise, through a new function `stretch_index` in `vkxp-pixel`.
- `hue_change` calls a new function `rotate_hue` in `vkxp-pixel`. It is the usual hue rotation
  in hue-saturation-value terms, done in exact integer arithmetic. With `M` and `m` the largest
  and smallest of the three channels and `d = M - m` (a pixel with `d = 0` is unchanged), the
  hue position `p` lies in `0..6d`:

  ```text
  M is red:    p = g - b            if g >= b, else 6d - (b - g)
  M is green:  p = 2d + (b - r)
  M is blue:   p = 4d + (r - g)     (ties resolved red, then green, then blue)

  p' = (p + floor((h * d + 30) / 60)) mod 6d      with h the angle reduced to 0..359
  s  = p' / d,   f = p' - s * d

  s = 0: (M, m+f, m)   s = 1: (M-f, M, m)   s = 2: (m, M, m+f)
  s = 3: (m, M-f, M)   s = 4: (m+f, m, M)   s = 5: (M, m, M-f)
  ```

  An angle of 0 reproduces every pixel exactly, and multiples of 60 degrees permute channels
  exactly.

Both new functions go into a new module of `vkxp-pixel` with its own provenance list and a
`bitmap_provenance()` accessor. They are deliberately not added to `provenance()`: that list is
the set the shaders mirror, and these run only on the CPU.

### Text through a rasteriser interface

`vkxp-core` defines

```text
trait TextRasterizer
    measure(font, text)   -> (width, height)
    rasterize(font, text) -> coverage mask of that size, one byte per pixel
```

and `draw_text` does the layout: the natural width `n` comes from `measure`; the drawn width is
`n` if it fits the rectangle of width `w`, otherwise `max(w, ceil(0.6 * n))`, reached by
resampling mask columns with `stretch_index`; the text is placed by alignment when it fits and
from the rectangle's left edge when it still does not; it is centred vertically with
`(h - text_height) / 2` rounded down and clipped to the rectangle. Each mask value `c` draws the
font colour with alpha `effective_alpha(c, colour alpha)` through `blend_normal`.

The implementation of the trait for real fonts belongs to `vkxp-text` and is wired up by the
binding. Tests use a deterministic stand-in (fixed-size cells with a recognisable pattern).

### Errors

One error enum for bitmaps (`Disposed`, `CreateFailed` with the reason) and one for malformed
serialized data. The binding turns them into the original's `RGSSError` messages.

## Risks / Trade-offs

- [A formula differs from the original runtime] → Every item below is isolated in one function
  and, where it is arithmetic, registered with a provenance marker.
- [Bounding-rectangle updates upload more than needed] → Acceptable at 640 by 480; the arena is
  the single place to refine it.
- [A bitmap accepted here exceeds a GPU's image size limit] → The Vulkan backend already reports
  a typed limit error; splitting oversized bitmaps is left for when a game needs it.
- [Text behaviour is specified against a stand-in rasteriser] → The interface is small, and the
  text change tests the real rasteriser against the same scenarios.

Behaviour to confirm against the original runtime later (placeholders today):

1. Discarding the fraction when a colour or tone component becomes an integer.
2. A table keeping the low 16 bits of a written value (rather than clamping).
3. The alpha arithmetic of `blt`, in particular onto partly transparent destinations.
4. The sampling rule of `stretch_blt`.
5. The hue rotation formula and its rounding.
6. The 60 percent squeeze limit of `draw_text`, how squeezed text is resampled, and where text
   that still does not fit is placed.
7. The result of a transfer within one bitmap when source and destination overlap.
8. The serialized layout of a rectangle.

## Open Questions

- Whether one bounding rectangle per bitmap is enough once tile animations and message windows
  redraw every frame. It affects performance only, not the specs or the tasks.
