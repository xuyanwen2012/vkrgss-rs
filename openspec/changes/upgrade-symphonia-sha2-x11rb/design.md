# Design

## Context

- `symphonia` 0.5.5: `crates/vkxp-audio` decodes Ogg Vorbis, WAV/PCM and MP3 with it; tests
  pin decoded lengths and samples of generated files.
- `sha2` 0.10.9: `crates/vkxp-ruby-sys/tests/manifest.rs` hashes the vendored tree against a
  committed manifest.
- `x11rb` 0.13.2: `xtask/src/oracle/original.rs` talks to the X server that the original
  runtime draws to under Wine.
- The gate runs with `--locked`; `cargo xtask audit` checks licences when `cargo-deny` is
  installed.

## Decisions

- One dependency per commit, in the order `sha2`, `x11rb`, `symphonia` (smallest first), each
  with the crate's tests passing.
- The decoding tests of `vkxp-audio` are not edited to fit new output. If 0.6 decodes a
  format to different samples (for example MP3 delay or padding handling), stop on that
  format: record under "Checked" what differs and by how much, and decide by this rule: a
  difference in leading or trailing silence handling that brings the output closer to the
  source's declared length is accepted and the test's expectation updated with the reason
  beside it; any other difference keeps `symphonia` at 0.5, is listed under "Follow-ups",
  and the other two upgrades still go in.
- If an upgrade needs a newer compiler than the pinned toolchain or pulls a dependency whose
  licence is outside `deny.toml`'s allow-list and is not MIT-compatible, that upgrade is
  left out and listed under "Follow-ups".
- The oracle (`cargo xtask oracle`) needs Wine and local games and is not run by the flow;
  `xtask`'s own tests and a build cover `x11rb`. Say so under "Checked".

## Risks / Trade-offs

- [The oracle breaks without a test noticing] → The owner's coordinator runs the seven-game
  comparison after the merge.

## Checked

- `sha2` 0.11.0: `crates/vkxp-ruby-sys/tests/manifest.rs` compiles and passes unchanged.
- `x11rb` 0.14.0: `xtask/src/oracle/original.rs` compiles unchanged; `cargo test -p xtask` and
  clippy pass. The oracle itself needs Wine and local games and was not run. `winit` 0.30
  still depends on `x11rb` 0.13.2, so the lock file holds both lines.
- `symphonia` 0.6.1: `crates/vkxp-audio/src/decode.rs` is adapted to the new interface (the
  probe returns the reader, the end of the data is `Ok(None)`, gapless trimming is an option
  of the decoder and no longer of the reader, the decoded block is copied into a `Vec<f32>`).
  The decoding tests pass with no expectation changed.
- The tests allow a tenth of a second, so the output of 0.5.5 and 0.6.1 was also compared
  sample by sample through `decode::open`, at 44100 Hz (the rate of the files) and at
  48000 Hz, with a test that is not committed:
  - `tone.mp3`, the same with its ID3v2 tag removed, with an ID3v1 tag appended, with an
    ID3v2 tag of 4 KiB, 300 kB and 3 MB in front, and cut to two thirds: identical.
  - A 16-bit mono WAV tone and an 8-bit stereo WAV: identical.
  - `tone.ogg`: 0.5.5 yields 44608 frames, 0.6.1 yields 44100, which is the length the file
    declares (one second; the end position of its last page). The 44100 frames equal the
    first 44100 of 0.5.5 exactly; the 508 frames that are gone are the tail of the last
    block behind the declared end (peak 0.087). At 48000 Hz that is 48000 frames instead of
    48553, equal except the very last frame (difference 0.0006), which no longer has a
    successor to interpolate to. This is a difference in trailing handling that brings the
    output to the declared length, so it is accepted by the rule above. No test pinned the
    old length, so no expectation changed.
  - `tone.ogg` cut to two thirds is rejected by both versions (the message changed from
    "end of stream" to "unexpected end of file").
- In 0.6 the readers of ID3 tags are features that `default-features = false` turns off; in
  0.5 they were always built. Without `id3v2` an MP3 file behind a 3 MB ID3v2 tag (a cover
  image) is no longer recognised, where 0.5.5 plays it. The `id3v2` feature is therefore
  enabled, and that file decodes identically again. `id3v1` and `ape` stay off: a trailing
  ID3v1 tag changes nothing either way.
- Compiler and licences: the newest minimum compiler among the new and changed packages is
  1.85, below the pinned 1.99.0. The new transitive packages are `const-oid`, `hybrid-array`,
  `num-complex`, `regex-lite` (MIT OR Apache-2.0) and `symphonia-common` (MPL-2.0), all in
  the allow-list of `deny.toml`, which is unchanged. `cargo-deny` is not installed on this
  machine, so `cargo xtask audit` did not check this; the licences were read from
  `cargo metadata`.

## Follow-ups

- None.
