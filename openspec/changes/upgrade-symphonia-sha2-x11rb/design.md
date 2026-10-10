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

## Follow-ups

- None yet.
