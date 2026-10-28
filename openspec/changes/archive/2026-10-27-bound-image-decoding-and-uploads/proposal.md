# Bound image decoding and uploads

## Why

Two whole-project reviews of 2026-10-26 (`target/review-2026-10-26/`, findings M7, L14, L15
and C3, and the test gaps of both) found limits that are stated but not kept:

- The hand-written JPEG decoder allocates its coefficient planes (about 3.3 times the output
  for a 4:4:4 file) from the frame header, before any scan is read. A 200-byte file that
  declares 16384 by 16384 and has no scan allocates 1.5 GiB and then gives up; every give-up
  is a silent `None` that decodes the whole file again through `jpeg-decoder`.
- The core accepts bitmaps up to 268435456 pixels, measured as created by the original
  (16384 by 16384), and says every such bitmap can be uploaded. Its pixels alone are 1 GiB,
  the bound of one framed message, so the first frame that uses it ends a process-mode
  session.
- The largest bitmap is defined four times (`vkrgss-core`, `vkrgss-compose-ref`,
  `vkrgss-image`, `vkrgss-vk`); a test ties only two of them.
- `Scene::extract` turns an error of `Bitmaps::deltas` into an empty list, and a plane whose
  stretch step is inexact emits one quad per copy with no stated bound.
- No coverage-guided run reaches the file decoders (JPEG, BMP, the RGSSAD reader); the long
  runs exist only for the interpreters.

## What Changes

- The JPEG decoder takes its coefficient memory when a scan needs it, keeps transient memory
  within a stated bound of the output, and says why it gave up; the fallback to
  `jpeg-decoder` is logged with that reason.
- The bitmap changes of one frame may travel in several messages: a guest that would exceed
  the bound of a message sends the bitmap changes ahead in messages of their own, applied
  with the next frame. Every bitmap the core accepts reaches the host. Protocol version bump.
- One definition of the largest bitmap, used by core, the compositors and the decoders.
- Extraction no longer swallows an arena error; the quads of a plane have a tested bound.
- `cargo xtask fuzz` gains targets for the JPEG decoder, BMP and the RGSSAD reader, run by
  `nightly.yml` beside the interpreter targets.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `image-decoding`: memory of a JPEG decode and the reason of a fallback.
- `guest-host-protocol`: bitmap changes ahead of a frame.
- `core-scene`: extraction errors and the bound of plane quads.
- `workspace-structure`: fuzz targets of the file readers.
- `embedded-ruby`, `embedded-ruby-1-9-2`: the list of targets that an unknown target names.

## Impact

`crates/vkrgss-image` (JPEG), `crates/vkrgss-proto` (message, codec, protocol version,
recording), `crates/vkrgss-core` (limit, extract, plane), `crates/vkrgss-compose-ref` and
`crates/vkrgss-vk` (the shared limit, and applying bitmap changes that come ahead of a frame),
`crates/vkrgss-binding` (sending them), `crates/vkrgss-host` (session: holding them until the
frame), `crates/vkrgss` relay if it inspects kinds, `crates/vkrgss-vfs` (fuzz entry point),
`xtask` (fuzz targets), `.github/workflows/nightly.yml`, `docs/development.md`,
`docs/architecture.md` (protocol section), `CHANGELOG.md`. No picture changes.
