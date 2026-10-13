# Architecture

[decisions.md](decisions.md) records the settled design; this file describes the layout that
follows from it.

## Layout

The guest side (script thread) and the host side (window, GPU, audio) exchange only plain data
defined in `vkxp-proto`.

| Crate | Responsibility | Side |
|---|---|---|
| `vkxp-proto` | Plain data shared by both sides: frame packet, op list, audio commands, input snapshot | shared |
| `vkxp-pixel` | Bit-exact pixel arithmetic with its provenance | shared |
| `vkxp-core` | Value types, bitmaps, scene objects, frame extraction | guest |
| `vkxp-image` | PNG, JPEG, BMP decoding | guest |
| `vkxp-text` | Font lookup, substitution table, glyph rasterisation | guest |
| `vkxp-vfs` | Path resolution, RTP lookup, encrypted archive | guest |
| `vkxp-ruby-sys` | Build of the embedded Ruby and raw FFI | guest |
| `vkxp-binding` | RGSS Ruby API on top of core | guest |
| `vkxp-guest` | The guest role: the hidden option, the guest record and the settings in it, the subscriber of the diagnostics, the attachment, the copies that are states, the sandbox and the signal settings of a guest process | guest |
| `vkxp-guest-main` | The main of a guest: what a guest is set up with, the text backend over the font engine, the guest on a thread, and the function that makes an executable a guest process | guest |
| `vkxp-compose-ref` | CPU reference compositor | host and tests |
| `vkxp-vk` | Vulkan backend | host |
| `vkxp-audio` | Mixer, decoding, MIDI synthesis | host |
| `vkxp-host` | Event loop, input mapping, thread orchestration, configuration | host |
| `vkxp` | Executable | |

`xtask` holds repository automation and the tool that extracts the built-in scripts; it is not
part of the runtime.

The protocol is at version 13; an encoded stream of another version is rejected. A frame packet
carries the changes to bitmaps and tables and an ordered list of ops of five kinds: `Clear`,
`Quad`, `RectEffect`, `TilemapLayer` and `TilemapPlane`. Sprites, planes and the parts of a
window are quads.
Both compositors evaluate every kind. Version 8 added the messages of the save states: the
requests to save and to load a state, the order to a parked copy of the guest process to
resume, the attachment of a guest process, the outcome of a game, and the answers saved,
loaded and failed. Version 9 added to the failed answer whether a save or a load failed.
Version 10 added two messages from the guest: a key that is pressed or released for the
player, which the session treats as the physical key of that Windows virtual-key code, and
whether the pointer is to be seen over the picture. Version 11 added `FramePacket::brightness`
and the field `rule` of `GuestToHost::Transition`. Version 12 added the op `TilemapPlane`.
Version 13 added the rule set of the tilemap plane op, `TilemapPlane::rules`: the tilemap of
RGSS2 or that of RGSS3.

`FramePacket::brightness` is a value from 0 to 255 that the scene keeps and writes into every
packet. Both compositors apply it after the last op: each colour channel `v` of the frame
becomes `(v * brightness) >> 8`, the alpha stays, and 255 leaves the frame as composed. A
frame that the display holds keeps the brightness it was composed with.

`GuestToHost::Transition` carries a `TransitionRule` in `rule`. With `TransitionRule::Rgss1`
the frames of a transition of N are shown at the steps 0 to N-1, the first of which is the
held frame alone, and the frame after them is shown alone. With `TransitionRule::Rgss2` they
are shown at the steps 1 to N, and the N-th is the newest frame alone and ends the
transition. The binding sends `TransitionRule::Rgss1` and sets no brightness.

`Op::TilemapPlane` draws one of the two planes of a tilemap of RGSS2, as `Op::TilemapLayer`
draws the tiles of one priority of one layer of a tilemap of RGSS1: nothing of a map is
composed in the guest, and both compositors evaluate the op per pixel from the tables and
the sheets. A `TilemapPlane` carries `clip`, `plane` (`Plane::Lower` or `Plane::Upper`),
`corner` (the screen pixel of the plane's top-left pixel), `first_cell` (the cell of the map
in the plane's first column and row), `animation_step`, `flash_level`, the table ids `map`,
`passages` and `flash`, of which the last two may be absent, and `bitmaps`, up to nine
bitmap ids. A plane is 21 by 16 cells of 32 by 32 pixels and the map repeats in it. The
lower plane draws the tiles whose entry of the passages table lacks the bit 0x10, the
shadows beside walls and the edges under tables; the upper plane draws the tiles of the
third layer that have the bit; both add the flash of a cell. A texel is blended as a window
layer is. `vkxp_proto::tiles_rgss2` is the one definition of the tile numbers, of the shapes
of the three kinds of autotiles and of the animation, which the reference compositor uses
and of whose tables the compute shader holds copies that a test compares. The scene emits
the lower plane at z 0 and the upper at z 200 among the elements of the tilemap's viewport.

## Headless proof

The contract between the two sides is testable without a window, Ruby or a GPU. The example
below reads an encoded stream of `GuestToHost` messages, runs it through the CPU reference
compositor and writes the last frame as PNG:

```sh
cargo run -p vkxp-compose-ref --example render_frame -- <messages> <output.png>
```

`vkxp_compose_ref::fixtures` generates the test streams from procedural patterns. Their expected
frames are committed under `crates/vkxp-compose-ref/tests/expected/` and compared byte for byte;
`VKXP_BLESS=1 cargo test -p vkxp-compose-ref --test fixtures` rewrites them after an intended
change, to be reviewed before committing.
`vkxp_compose_ref::fixtures::tilemap_rgss2` rebuilds by formula the scenes with which the
tilemap of VX was measured on the original runtime. Their expected frames are committed as
digests in `crates/vkxp-compose-ref/tests/data/tilemap-rgss2-digests.txt`; they come from
the model of that measurement and are not rewritten from a compositor of this repository.

## Dependency rules

Four rules keep the guest/host boundary intact. A test in `xtask` checks them against
`cargo metadata` as part of the `test` step, and `cargo xtask deps` runs the check alone. A
violation is reported with the full dependency path that causes it.

1. **Guest crates do not reach host crates.** None of `vkxp-core`, `vkxp-image`, `vkxp-text`,
   `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-binding`, `vkxp-guest`, `vkxp-guest-main` may depend,
   directly or transitively, on `vkxp-vk`, `vkxp-audio` or `vkxp-host`.
2. **Host crates do not reach guest crates.** None of `vkxp-vk`, `vkxp-audio`, `vkxp-host`, and
   not the reference compositor `vkxp-compose-ref` either, may depend, directly or transitively,
   on a guest crate, `vkxp-guest` and `vkxp-guest-main` included. The `vkxp` binary is exempt from both directions: it is the one crate that
   depends on both sides and joins them.
3. **`vkxp-core` stays pure.** Everything `vkxp-core` reaches, directly or transitively, must be
   on its allow-list: `vkxp-proto`, `vkxp-pixel` and the pure crates those need. The list is
   `CORE_ALLOW_LIST` in `xtask/src/deps.rs`; adding to it is a reviewed decision, and nothing
   that does file I/O or binds Ruby, Vulkan, windowing or audio belongs there.
4. **The guest role links no interpreter.** `vkxp-guest` may not depend, directly or
   transitively, on `vkxp-ruby-sys` or `vkxp-binding`: an executable of another interpreter
   links it too, and gives it the function that runs the game.

The check runs with every feature of every workspace crate enabled, so an optional dependency
or an off-by-default feature cannot hide a forbidden path. Only normal and build dependencies
count; dev-dependencies are not linked into either side.

In addition, `vkxp-proto`, `vkxp-pixel`, `vkxp-core` and `vkxp-compose-ref` carry
`#![forbid(unsafe_code)]`, so `unsafe` in them does not compile, and so does `vkxp-guest-main`.
`vkxp-guest` denies `unsafe`
at its root and allows it in the modules of the copier and of the signals only.
