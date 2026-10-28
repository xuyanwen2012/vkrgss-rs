# Architecture

[decisions.md](decisions.md) records the settled design; this file describes the layout that
follows from it.

## Layout

The guest side (script thread) and the host side (window, GPU, audio) exchange only plain data
defined in `vkrgss-proto`.

| Crate | Responsibility | Side |
|---|---|---|
| `vkrgss-proto` | Plain data shared by both sides: frame packet, op list, audio commands, input snapshot | shared |
| `vkrgss-pixel` | Bit-exact pixel arithmetic with its provenance | shared |
| `vkrgss-core` | Value types, bitmaps, scene objects, frame extraction | guest |
| `vkrgss-image` | PNG, JPEG, BMP decoding | guest |
| `vkrgss-text` | Font lookup, substitution table, glyph rasterisation | guest |
| `vkrgss-vfs` | Path resolution, RTP lookup, encrypted archive | guest |
| `vkrgss-ruby-glue` | The bridge between Rust and Ruby that both interpreter crates share: host functions and their trampolines, rooting, the protected call and the calls built on it, generic over a trait of the C API; no C code and no unmangled symbol | guest |
| `vkrgss-ruby-sys` | Build of the embedded Ruby 1.8.7 and raw FFI | guest |
| `vkrgss-ruby192-sys` | Build of the embedded Ruby 1.9.2, the interpreter of RGSS3, and raw FFI under the names of `vkrgss-ruby-sys`; empty on every target but `x86_64-unknown-linux-gnu` | guest |
| `vkrgss-binding` | RGSS Ruby API on top of core, over Ruby 1.8.7 | guest |
| `vkrgss-binding-rgss3` | The source of `vkrgss-binding` compiled over Ruby 1.9.2: no source of its own, and the tests of `vkrgss-binding` a second time | guest |
| `vkrgss-guest` | The guest role: the hidden option, the guest record and the settings in it, the subscriber of the diagnostics, the attachment, the copies that are states, the sandbox and the signal settings of a guest process | guest |
| `vkrgss-guest-main` | The main of a guest: what a guest is set up with, the text backend over the font engine, the guest on a thread, and the function that makes an executable a guest process | guest |
| `vkrgss-guest-main-rgss3` | The source of `vkrgss-guest-main` compiled over `vkrgss-binding-rgss3` and Ruby 1.9.2: no source of its own | guest |
| `vkrgss-guest-rgss12` | The guest executable of games of RGSS1 and RGSS2: the guest role and `vkrgss-guest-main`, nothing of the host; `vkrgss` starts it from beside itself | guest |
| `vkrgss-guest-rgss3` | The guest executable of games of RGSS3: the guest role and `vkrgss-guest-main-rgss3`, nothing of the host; `vkrgss` starts it from beside itself | guest |
| `vkrgss-compose-ref` | CPU reference compositor | host and tests |
| `vkrgss-vk` | Vulkan backend | host |
| `vkrgss-audio` | Mixer, decoding, MIDI synthesis | host |
| `vkrgss-host` | Event loop, input mapping, thread orchestration, configuration | host |
| `vkrgss` | Executable: the program, and the guest of games of RGSS1 and RGSS2 on a thread | |

`xtask` holds repository automation and the tool that extracts the built-in scripts; it is not
part of the runtime.

The RGSS classes have one source, `crates/vkrgss-binding/src`, and two packages. What differs
between the interpreters is behind the surface that both interpreter crates offer under the
same names (which Ruby it is, the options for a runtime, the main object, the accessors of
strings and arrays, a constructor for bytes and one for text, the mark of the game's scripts)
or is asked at run time in one place, `rb::RUBY_19`; no `cfg` and no cargo feature tells the
interpreters apart. The Rust side of that surface exists once, in `vkrgss-ruby-glue`: its
`Ruby`, `Root` and `protect` are generic over the trait `Interpreter`, which each interpreter
crate implements on its unit type `CApi` with its own declarations, and each offers them for
its interpreter under its own paths (`vkrgss_ruby_sys::Ruby` is `Ruby<CApi>`). What differs
between the interpreters in that code is an item of the trait. The glue has no C code and no
unmangled symbol, so both interpreter crates depend on it although no program links the two. A string the classes create is bytes (ASCII-8BIT over Ruby 1.9.2, as on
the original runtime of RGSS3) unless it is script source or the path of the script archive.
The main of a guest has one source as well, `crates/vkrgss-guest-main/src`, compiled a second
time by `vkrgss-guest-main-rgss3` over `vkrgss-binding-rgss3`. The guest executable
`vkrgss-guest-rgss3` links it, and nothing else of the workspace: the program starts it for a
game of RGSS3, always as a guest process, and finds it in the directory of its own file. The
guest executable `vkrgss-guest-rgss12` links `vkrgss-guest-main` in the same way, and the
program starts it for the guest process of a game of RGSS1 or RGSS2; the program never
starts itself as a guest, so the process that a state copies with `fork` links nothing of
the host. Each executable refuses a game of a runtime its interpreter does not run.

The host knows nothing of a guest process. It is given, as its guest, the relay of `vkrgss`
(`crates/vkrgss/src/process.rs`, which `vkrgss-android` compiles too): it holds the guest end
of the in-process link and hands each message on, between the link and the connection of the
running guest process, and keeps the copies that are states. The relay never waits on a
guest. It reads every connection without blocking, through the incremental reader of the
framing (`vkrgss_proto::framing::MessageReader`), and keeps what it is to write to a guest in a
buffer that it writes as the connection takes it, so its deadline is looked at on every turn:
once the host has gone, a guest is ended five seconds after it was asked to quit, by a quit
request or by the host's going. A new connection attaches as its statement of the version and
its `Attach` arrive, five seconds at most; the running guest's messages wait for it, so that a
copy is known before what its maker sent after making it. Between two ticks the relay hands
the host at most 256 MiB of messages from a guest process that are not frame packets, and apart
from them 2 GiB of bitmap changes ahead of a frame, and ends a guest that sends more. The session of the host gives up on a guest that has not shut down
five seconds after it first asked it to quit, whether a thread or a process, and the windowed
runner then returns without waiting for a guest thread.

The protocol is at version 16; an encoded stream of another version is rejected. A frame packet
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
RGSS2 or that of RGSS3. Version 14 added the rule of the freeze message,
`GuestToHost::Freeze::rule`: the frozen picture is kept as the frame at its own size, or in
the buffer of RGSS3, 640 by 480 pixels whose top left corner a freeze writes. Version 15
added to `AudioCommand::Play` the sample frame at which its sound starts, `start`, and the
time over which the music returns after a music effect, `music_return_ms`, let its pitch run
from 1 to 65535, and added `AudioCommand::Volume`, which sets the volume of what a channel
plays without starting it again. Version 16 added `GuestToHost::Bitmaps`, bitmap changes of
the next frame sent ahead of it.

One framed message is at most 1 GiB (`MAX_MESSAGE_LEN`), and the largest bitmap,
`vkrgss_proto::MAX_BITMAP_PIXELS` (16384 by 16384, the one definition that the core and both
compositors use), is 1 GiB of pixels alone. When the pixels of a frame's bitmap changes pass a
quarter of the bound (`framing::BITMAPS_AHEAD_LEN`), the guest sends all of them ahead of the
frame in `Bitmaps` messages of at most that many bytes, an update of a large bitmap cut into
bands of whole rows. The host keeps them in order and applies them with the next frame, before
that frame's own changes; they get no tick, and the frame gets its one tick as before. A
loaded state drops what was kept for the guest that is gone. Every bitmap the core accepts so
reaches the host, in a process as on a thread.

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
layer is. `vkrgss_proto::tiles_rgss2` is the one definition of the tile numbers, of the shapes
of the three kinds of autotiles and of the animation, which the reference compositor uses
and of whose tables the compute shader holds copies that a test compares. The scene emits
the lower plane at z 0 and the upper at z 200 among the elements of the tilemap's viewport.

## Headless proof

The contract between the two sides is testable without a window, Ruby or a GPU. The example
below reads an encoded stream of `GuestToHost` messages, runs it through the CPU reference
compositor and writes the last frame as PNG:

```sh
cargo run -p vkrgss-compose-ref --example render_frame -- <messages> <output.png>
```

`vkrgss_compose_ref::fixtures` generates the test streams from procedural patterns. Their expected
frames are committed under `crates/vkrgss-compose-ref/tests/expected/` and compared byte for byte;
`VKRGSS_BLESS=1 cargo test -p vkrgss-compose-ref --test fixtures` rewrites them after an intended
change, to be reviewed before committing.
`vkrgss_compose_ref::fixtures::tilemap_rgss2` rebuilds by formula the scenes with which the
tilemap of VX was measured on the original runtime. Their expected frames are committed as
digests in `crates/vkrgss-compose-ref/tests/data/tilemap-rgss2-digests.txt`; they come from
the model of that measurement and are not rewritten from a compositor of this repository.

## Dependency rules

Four rules keep the guest/host boundary intact. A test in `xtask` checks them against
`cargo metadata` as part of the `test` step, and `cargo xtask deps` runs the check alone. A
violation is reported with the full dependency path that causes it.

1. **Guest crates do not reach host crates.** None of `vkrgss-core`, `vkrgss-image`, `vkrgss-text`,
   `vkrgss-vfs`, `vkrgss-ruby-glue`, `vkrgss-ruby-sys`, `vkrgss-ruby192-sys`, `vkrgss-binding`, `vkrgss-binding-rgss3`,
   `vkrgss-guest`, `vkrgss-guest-main`, `vkrgss-guest-main-rgss3`, `vkrgss-guest-rgss12`,
   `vkrgss-guest-rgss3` may depend,
   directly or transitively, on `vkrgss-vk`, `vkrgss-audio` or `vkrgss-host`.
2. **Host crates do not reach guest crates.** None of `vkrgss-vk`, `vkrgss-audio`, `vkrgss-host`, and
   not the reference compositor `vkrgss-compose-ref` either, may depend, directly or transitively,
   on a guest crate, `vkrgss-guest`, the two packages of the main of a guest and the guest
   executables included. The `vkrgss` binary is exempt from both directions: it is the one crate that
   depends on both sides and joins them.
3. **`vkrgss-core` stays pure.** Everything `vkrgss-core` reaches, directly or transitively, must be
   on its allow-list: `vkrgss-proto`, `vkrgss-pixel` and the pure crates those need. The list is
   `CORE_ALLOW_LIST` in `xtask/src/deps.rs`; adding to it is a reviewed decision, and nothing
   that does file I/O or binds Ruby, Vulkan, windowing or audio belongs there.
4. **The guest role links no interpreter.** `vkrgss-guest` may not depend, directly or
   transitively, on `vkrgss-ruby-sys`, `vkrgss-ruby192-sys`, `vkrgss-binding` or
   `vkrgss-binding-rgss3`: an executable of another interpreter links it too, and gives it the
   function that runs the game.

The check runs with every feature of every workspace crate enabled, so an optional dependency
or an off-by-default feature cannot hide a forbidden path. Only normal and build dependencies
count; dev-dependencies are not linked into either side.

Beside the four, the same check holds the two interpreters apart: no package reaches both
`vkrgss-ruby-sys` and `vkrgss-ruby192-sys`, which define the same symbols and cannot be linked
into one program. Here development dependencies count too, because a test binary links
them. Tests of `xtask` also hold the twin packages together, those of the binding and those of
the main of a guest: they declare the same dependencies apart from the interpreter crate and
the binding package, and every test file of `vkrgss-binding` or `vkrgss-guest-main` has a file
of the same name in the package over Ruby 1.9.2. The program and the guest executables have
one version, which a test of `xtask` holds as well.

In addition, `vkrgss-proto`, `vkrgss-pixel`, `vkrgss-core` and `vkrgss-compose-ref` carry
`#![forbid(unsafe_code)]`, so `unsafe` in them does not compile, and so does `vkrgss-guest-main`;
`vkrgss-guest-rgss12` and `vkrgss-guest-rgss3` have no `unsafe`.
`vkrgss-ruby-glue` allows `unsafe` at its root, as the two interpreter crates do: it holds
the calls into Ruby and the trampolines that they share, each block with its `SAFETY`
comment.
`vkrgss-guest` denies `unsafe`
at its root and allows it in the modules of the copier and of the signals only.
