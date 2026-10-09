# Tasks

## 1. Game configuration (`vkxp-vfs`)

- [x] 1.1 Add the dependencies (`encoding_rs`; `tempfile` for tests) and an error type for the crate. Verify: `cargo build -p vkxp-vfs` has no warnings and `cargo xtask deps` reports no violation.
- [x] 1.2 Implement reading `Game.ini`: case-insensitive file, section and key names, both line endings, trimming, the six values, errors for a missing file and a missing `Scripts`. Verify: tests for the three scenarios of the configuration-file requirement in `specs/game-files/spec.md`.
- [x] 1.3 Implement the decoding rule (UTF-8 with optional byte-order mark, then GBK, Shift-JIS, Big5, with a caller-named override). Verify: tests for the three encoding scenarios, including the bytes `B9 A4 B3 CC`.
- [x] 1.4 Implement the resolution of RTP names through a caller-supplied table. Verify: tests for the two run-time-package scenarios and for skipping empty names.

## 2. Path resolution (`vkxp-vfs`)

- [x] 2.1 Implement component-wise, case-insensitive matching with both separators, the exact-case preference, the rejection of paths that leave their root, and the per-directory cache with `refresh`. Verify: tests for the three path-matching scenarios, built in a temporary directory.
- [x] 2.2 Implement the extension lists per kind and the rule that a name matching as given wins. Verify: tests for the three extension-search scenarios.
- [x] 2.3 Implement the search order over the game directory and RTP directories, data files in the game directory only, the not-found error carrying the requested path, and `read`. Verify: tests for the search-order and lookup-failure scenarios.
- [x] 2.4 Document the crate in `crates/vkxp-vfs/src/lib.rs` with a usage example that compiles as a doctest. Verify: `cargo test -p vkxp-vfs --doc` passes.

## 3. Image decoding (`vkxp-image`)

- [x] 3.1 Add the dependencies (`png`, `jpeg-decoder` in place of `zune-jpeg`, see `design.md`; a development-only JPEG encoder), the `DecodedImage` type, the error enum and signature sniffing. Verify: tests for the two format-recognition scenarios in `specs/image-decoding/spec.md`.
- [x] 3.2 Implement PNG decoding. Verify: tests for the three PNG scenarios plus one test per colour type and for an interlaced image, with inputs encoded in the test.
- [x] 3.3 Implement JPEG decoding. Verify: tests for the two JPEG scenarios, a progressive image, and an unsupported component layout returning the unsupported-format error.
- [x] 3.4 Implement BMP decoding for 1, 4, 8, 24 and 32 bits, bottom-up and top-down, with the alpha rule for 32 bits and an error for compressed data. Verify: tests for the three BMP scenarios and one per bit depth, with inputs built byte by byte.
- [x] 3.5 Enforce the pixel limit before allocation and make every failure a typed error. Verify: tests for the three malformed-input scenarios: truncation at every length for one input per format, a property test over random bytes behind each signature, and the 65535 by 65535 header.
- [x] 3.6 Document the crate in `crates/vkxp-image/src/lib.rs`. Verify: `cargo doc -p vkxp-image --no-deps` builds without warnings.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-game-files-and-image-decoding --strict` passes.
