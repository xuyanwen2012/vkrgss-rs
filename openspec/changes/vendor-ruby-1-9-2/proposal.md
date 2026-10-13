## Why

VX Ace (RGSS3) embeds Ruby 1.9.2p0, revision 29036, and the decisions of 2026-10-13
(change `plan-vx-ace`, decision 4) say how vkxp gets it: the release vendored under
`third_party/ruby-1.9.2/` as 1.8.7 is, built by a sys crate of its own that offers the Rust
API of `vkxp-ruby-sys`, with the collector stress in the gate from the start. A study of
the same day built that release as a static library from the tarball, one checked-in
`config.h` and eight patches, and compared 23 groups of values with the original runtime
(`ref/tools/ruby192-feasibility/REPORT.md` and
`ref/tools/oracle/measurements/ace-v0-REPORT.md`, both outside the repository). Nothing of
it is in the repository yet, and the binding of RGSS3 cannot start before it is.

## What Changes

- `third_party/ruby-1.9.2/`: the part of `ruby-1.9.2-p0.tar.gz` that the interpreter and its
  own tests need, with the generated files the release ships, a manifest of upstream hashes,
  the licence files and a patch log that starts with the eight patches of the study.
- A new crate `vkxp-ruby192-sys`: builds that tree with `cc` (no `configure`, no `make`, no
  Ruby, nothing generated at build time) and offers the modules and items of
  `vkxp-ruby-sys` over it. On a target other than `x86_64-unknown-linux-gnu` it builds as
  an empty crate.
- The interpreter starts as the original was measured to: no prelude, `$:` is `["."]`,
  `$LOADED_FEATURES` is `["enumerator.so"]`, default external encoding UTF-8, `Zlib` built
  in without being a feature, four encodings listed, Shift_JIS, Windows-31J and ISO-8859-1
  usable, every other encoding unknown.
- `String#hash` and the other hashes have a fixed seed, and the interpreter creates no
  thread of its own before a script creates a Ruby thread.
- Tests: the manifest; the release's `bootstraptest` and a selection of `test/ruby` with a
  list of the failures that are explained; the same selection under `GC.stress` as part of
  the workspace tests, the slow files under `cargo xtask stress`; a copy of the process made
  by `fork` between two frames of a script with Fibers; `Marshal`, `srand` and `rand` and
  float text against the values measured on the original.
- A check that no package of the workspace links both interpreters.
- No program change: `vkxp` does not link the new crate, and no game of RGSS3 runs yet.

## Capabilities

### New Capabilities

- `embedded-ruby-1-9-2`: the Ruby 1.9.2 interpreter embedded for RGSS3: how it is built
  from vendored source, what is true of it at start-up, what of it is held against the
  original runtime, and how it is tested.

### Modified Capabilities

None.

## Impact

- New: `third_party/ruby-1.9.2/`, `crates/vkxp-ruby192-sys/`.
- `Cargo.toml` and `Cargo.lock` (one workspace member, no new external dependency),
  `xtask` (`deps.rs`: the new crate on the guest side and the rule on the two interpreters;
  `stress.rs` and `stress.skip`: one more step), `crates/vkxp-ruby-sys/tests/manifest.rs`
  (one more tree), `THIRD-PARTY.md`, `AGENTS.md` (the crates that may hold `unsafe`),
  `README.md` (the table of crates), `docs/building.md`, `docs/development.md`.
- The workspace tests take longer: the upstream suites of 1.9.2 and their stressed run are
  part of them.
- Not touched: `crates/vkxp-ruby-sys/src`, `third_party/ruby-1.8.7/`, `vkxp-binding`, the
  executable, `deny.toml`.
