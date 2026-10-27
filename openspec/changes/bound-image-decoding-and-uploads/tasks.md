# Tasks

## 1. JPEG memory and reasons

- [x] 1.1 Allocate a component's coefficients at the first scan that codes it; refuse in the custom path a file whose transient memory would pass the stated multiple of its output; replace the `None` of `coefficients::read` and `decode_like_the_original` by a `GiveUp` reason and log it at debug level on fallback. Verify: the scenarios of "Memory of a JPEG decode" as unit tests of `vkrgss-image` with a counting allocator; `jpeg/{coefficients,idct,samples}.rs` gain unit tests of their own (prefix and flipped-byte loops as in `vkrgss-core/src/sound.rs`); the measured JPEG tests pass unchanged.

## 2. One limit, extraction, plane quads

- [ ] 2.1 Move the largest bitmap to `vkrgss_proto::MAX_BITMAP_PIXELS` and use it in core, compose-ref and vk; a core test ties `vkrgss_image::MAX_PIXELS` to it. Verify: the test "Limits agree"; `cargo xtask deps` (or the gate's deps part) passes.
- [ ] 2.2 `Scene::extract` reports an arena error; the binding reports it as a failure of the session. Verify: the scenario "Arena error".
- [ ] 2.3 State and test the bound of plane quads. Verify: the scenario "Smallest tile".
- [ ] 2.4 Stretch cut: share a helper only where a proptest shows identical answers, otherwise cross-reference the three copies in comments; record the outcome in design.md. Verify: the proptest if any; core tests pass.

## 3. Bitmap changes ahead of a frame

- [ ] 3.1 `GuestToHost::Bitmaps`, codec, recording, protocol version bump; the binding sends changes ahead and splits large updates into row bands under the threshold; the session holds them until the next frame and applies them first; the relay passes them; both compositors accept them through the existing delta path. Verify: the three scenarios of "Bitmap changes ahead of a frame" (the first as a process-mode test of the program at lvp, `VK_LOADER_DRIVERS_SELECT='*lvp*'`), the framing round-trip tests, the session tests of the tick handshake.

## 4. Fuzz targets of the file readers

- [ ] 4.1 Targets `jpeg`, `bmp`, `rgssad` in `xtask fuzz` with seeds from the committed fixtures, `nightly.yml` matrix, `xtask/tests/workflows.rs`, the usage text and `docs/development.md`. Verify: `cargo xtask fuzz jpeg --seconds 30`, same for `bmp` and `rgssad`, end with status 0; the workflow tests pass; the unknown-target message names all nine.

## 5. Documents and checks

- [ ] 5.1 `docs/architecture.md` (protocol), `docs/development.md` (fuzz), `vkrgss-image/src/lib.rs` crate doc (memory bound and fallback), `vkrgss-core/src/bitmap.rs` doc of the limit, `CHANGELOG.md`. Verify: the document tests pass.
- [ ] 5.2 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical`, `openspec validate bound-image-decoding-and-uploads --strict` passes.
