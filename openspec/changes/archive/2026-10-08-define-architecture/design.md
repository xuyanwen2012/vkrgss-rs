# Design

## Context

See `proposal.md` for motivation and `docs/decisions.md` for the settled architecture. This
document only covers how the foundation is built. No code exists yet; Rust 1.99 stable is the
installed toolchain.

## Goals / Non-Goals

**Goals:**

- A workspace where each later OpenSpec change touches one crate and cannot break the
  guest/host boundary by accident.
- Protocol types complete enough that core, the CPU reference and the Vulkan backend can be
  developed in parallel against them.
- A runnable proof: packet bytes in, PNG out, with tests.

**Non-Goals:**

- Final op set. Plane, tilemap, window-part and transition-mask ops are declared as reserved
  variants but not specified or implemented here.
- Bit exactness against the original runtime. Formulas here are textbook and marked as such.
- Performance work of any kind.

## Decisions

### Workspace layout

`crates/<name>` for all thirteen crates, one `Cargo.toml` workspace with shared
`[workspace.package]`, `[workspace.lints]` and `[workspace.dependencies]`. Edition 2024,
`rust-toolchain.toml` pinned to the installed stable. Stub crates contain only a crate-level doc
comment. Alternative considered: creating crates lazily as each module starts; rejected because
dependency-direction checks need all names to exist from day one.

### Enforcing dependency direction

A test in an `xtask` crate reads `cargo metadata` and fails if a guest crate
(`vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-binding`) depends,
directly or transitively, on a host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`), or if
`vkxp-core` depends on anything outside `vkxp-proto`, `vkxp-pixel` and an allow-list of pure
crates. `vkxp-core`, `vkxp-proto`, `vkxp-pixel` and `vkxp-compose-ref` carry
`#![forbid(unsafe_code)]`. Alternative considered: `cargo-deny` bans; rejected because it cannot
express "transitively, between workspace members" as directly.

### Gate

`cargo xtask gate` runs fmt check, clippy with `-D warnings`, workspace tests and
`openspec validate --all`. One command so flows and humans run the same thing.

### Protocol shape (`vkxp-proto`)

- `GuestToHost`: `Frame(FramePacket)`, `Freeze`, `Transition { duration, mask, vague }`,
  `Audio(AudioCommand)`, `Modal { text }`, `SetTitle`, `Shutdown`.
- `HostToGuest`: `Input(InputSnapshot)`, `ModalDismissed`, `Tick`, `Quit`.
- `FramePacket { frame_index, logical_size, bitmaps: Vec<BitmapDelta>, ops: Vec<Op> }`.
- `BitmapDelta`: `Create { id, width, height }`, `Update { id, rect, rgba }`, `Destroy { id }`.
  Bitmap ids are `(index, generation)` so a stale reference is detectable.
- `Op` is an enum; each variant carries a clip rectangle. Implemented here: `Clear`, `Quad`,
  `RectEffect`. Reserved (declared, rejected by backends with a typed error): `Plane`,
  `TilemapLayer`.
- Coordinates and transforms are fixed point (`i32` with 16 fractional bits); colours and
  opacities are integers. No floats cross the boundary, so two backends cannot disagree through
  float rounding.
- Draw order is the order of `ops`. Sorting by z and creation order is the guest's job; backends
  never reorder.

Alternative considered: a retained scene graph shared between sides; rejected because it
couples the sides and cannot cross a process boundary.

### Encoding

`serde` with `postcard` for a compact, deterministic binary form, prefixed by a magic and a
protocol version. Used for input recordings, test fixtures and the later process split; inside
one process the types move through channels without encoding. Alternative considered: `bincode`
(encoding stability across versions is weaker) and a hand-rolled format (unneeded cost now).

### Pixel arithmetic (`vkxp-pixel`)

Pure `fn`s on integer channels: normal, additive and subtractive blend with opacity, colour
blend, tone. Each function documents its formula and carries a `PROVENANCE` note: `Textbook` now,
later a reference to the reverse-engineering spec that replaced it. Both `vkxp-compose-ref` and
later `vkxp-core` bitmap operations call these, and the GLSL in `vkxp-vk` mirrors them.

### CPU reference compositor (`vkxp-compose-ref`)

Owns a bitmap store fed by `BitmapDelta`s and evaluates ops per pixel in order over a
`logical_size` RGBA8 buffer. Per-pixel evaluation (not per-op blitting) is deliberate: it is the
same structure the compute shader will have, which keeps the two easy to compare. No tile
binning here; binning is a backend optimisation with no visible effect.

### Headless proof

`vkxp-compose-ref` ships an example binary that reads encoded `GuestToHost` messages from a file
and writes the resulting frame as PNG. Test fixtures are built in code from generated patterns
(checkerboard, gradients, alpha shapes); expected outputs are small committed PNGs produced by
that same code path and reviewed once.

## Risks / Trade-offs

- [Op set frozen too early] → Only three op kinds are specified; the rest are reserved names.
  The protocol version makes later additions explicit.
- [Fixed-point format turns out too coarse for rotation] → 16 fractional bits is a starting
  point; changing it is a protocol version bump while no backend beyond the reference exists.
- [Textbook arithmetic mistaken for final] → Provenance markers and a spec requirement that
  provenance is queryable.
- [Stub crates rot] → They contain no code; the dependency test keeps their names meaningful.

## Open Questions

- Whether `postcard` remains the encoding once the guest becomes a separate process and shared
  memory is used for pixel data. Does not affect the types or this change's tasks.
