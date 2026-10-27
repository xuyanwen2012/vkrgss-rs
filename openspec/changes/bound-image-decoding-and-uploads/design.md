# Design

## Context

- Reviews: `target/review-2026-10-26/fable-review.md` (M7, L14, L15, section 4) and
  `codex-review.md` (defect 9). Not in the repository; the findings are repeated here.
- `docs/decisions.md` ("Largest bitmap of RGSS3") and `docs/compatibility.md`: the original
  created 16384 by 16384, so the limit of 268435456 pixels stays. Lowering it is not an option.
- `vkrgss-proto/src/framing.rs`: `MAX_MESSAGE_LEN = 1 << 30` includes the encoding.
- The change `end-the-session-without-the-guest` (merged before this one) bounds what the host
  queues from a guest. Bitmap changes ahead of a frame must fit the budget it set; if they do
  not, adjust that budget here and say so in this file.

## Decisions

- **JPEG memory.** `coefficients::read` allocates a component's coefficient plane at the first
  scan that covers the component, not at the frame header. A file whose transient memory would
  pass a fixed multiple of its RGBA output (state the multiple; 4:4:4 needs about 3.3) is not
  decoded by the custom path. The reason travels as a small enum (`GiveUp`) instead of `None`;
  `decode` logs it through `tracing` at debug level when it falls back. Outputs of every file
  the custom path decodes today stay byte-identical (the committed fixtures and the measured
  scenarios of "JPEG arithmetic of the original").
  Outcome: the stated multiple is 4, plus a fixed allowance of 1 MiB for tables and for the
  block grid of a small image, which covers whole MCUs (a 1 by 1 image of chroma halved in
  both directions has a grid of 16 by 16 samples). The custom path counts, from the frame
  header, the coefficients (two bytes) and samples (one byte) of every block grid and the
  output, and leaves a frame above the bound to `jpeg-decoder`; the coefficients of a
  component are dropped once its samples are made, so a 4:4:4 decode of 1024 by 1024 peaks
  at about 7 MiB (13.6 MiB before). The counting allocator of the tests implements the
  unsafe trait `GlobalAlloc` in test code of `vkrgss-image` (`tests/counting/mod.rs`),
  passing every call to `System`; the library keeps `forbid(unsafe_code)`.
- **Fallback memory.** `jpeg-decoder` is called only after the header check of `rgba_len`, as
  now; a file that declares a size without any scan fails in the custom path's header pass and
  is not handed to the library a second time if the library would only fail the same way (a
  missing SOS is a malformed error of both).
- **Bitmap changes ahead of a frame.** A new message `GuestToHost::Bitmaps(Vec<BitmapDelta>)`
  carries bitmap changes that belong to the next `Frame`. The guest sends it when the encoded
  frame packet would pass a threshold (say a quarter of `MAX_MESSAGE_LEN`), splitting an
  `Update` of a large bitmap into row bands so that each message stays under the threshold. The
  host keeps them in order and applies them with the next frame, before that frame's own
  deltas; it sends no tick for them. A `Frame` without preceding `Bitmaps` behaves as now. The
  tick handshake and the order of deltas are unchanged in meaning. Recordings store the new
  message like any other. Bump `PROTOCOL_VERSION`.
- **One limit.** `vkrgss_proto::MAX_BITMAP_PIXELS` (proto is the one crate core,
  compose-ref and vk all depend on). `vkrgss-image` depends on no crate of the workspace; it
  keeps its own `MAX_PIXELS` and a test in `vkrgss-core` (dev-dependency on `vkrgss-image`, a
  guest crate like core) asserts equality. Check `xtask/src/deps.rs` allows it.
- **Extraction.** `Scene::extract` returns the arena error to its caller (or panics with a
  message in debug and returns the error in release, if the signature cannot change cheaply):
  pick one and record it. The binding reports it as a session failure.
  Outcome: the signature changed, `Scene::extract` returns `Result<FramePacket,
  BitmapError>` and leaves the frame index and the table deltas as they were on an error.
  The binding ends the game as on a quit request, sends no packet, and the run returns
  `RunError::Extraction`, so the host gets `Outcome { ok: false }`. A test makes the arena
  fail through a field of `Bitmaps` that exists only in the tests of `vkrgss-core`.
- **Plane quads.** The binding never sets a logical size above 640 by 480 and a tile is at
  least one pixel, so a plane emits at most one quad per covered pixel. State that bound in
  the doc comment of the plane extraction and test it at the worst case (1 by 1 texel tile,
  640 by 480). Merging copies of an inexact stretch would change pixels; it is not done.
- **Stretch cut.** The three copies (`core/src/bitmap.rs` `transfer_axis`, `turned.rs`
  `texel`, `extract.rs` `cut_span`) answer different questions (pixel lists, one texel, a
  span) in different integer widths. Share a helper only where a proptest over the full input
  range shows the shared form gives the same answers as each copy; otherwise leave them and add
  a comment in each that names the other two. Either way `cargo xtask xarch check` stays
  identical.
  Outcome: no helper is shared. `transfer_axis` (i64) and `Span::texel` (i128) share the
  formulas of the first and the last pixel but walk by `stretch_index` and by the step of a
  quad, and `cut_span` leaves texels out without starting anew; each has a comment naming
  the other two.
- **Fuzz targets of the file readers.** Three targets, `jpeg`, `bmp` and `rgssad`, run by
  `cargo xtask fuzz <name>` on the pinned toolchain: coverage-guided with `libfuzzer-sys` if it
  builds on the pinned stable toolchain, otherwise a seeded random-mutation loop for the given
  seconds over the committed fixtures. They check that decoding never panics, never allocates
  beyond the stated bounds (a counting global allocator in the target) and returns a full
  image or an error. They are robustness tests of the decoders. `nightly.yml`'s `fuzz` matrix
  gains the three names; the artifact upload stays as it is.

## Risks

- Splitting an upload changes when the host sees pixels, not which: the host applies the
  changes before the frame that needs them, as now. xarch frames stay identical.
- A recording of the previous protocol version is unreadable, as with every bump (documented).

## Follow-ups

None yet.
