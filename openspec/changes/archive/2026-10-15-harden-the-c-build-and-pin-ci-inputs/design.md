# Design

## Context

`base_build()` in both build scripts sets `-O2`, PIC, frame pointers and the pinned `FLAGS`
(`-std=gnu89 -fwrapv -fno-strict-aliasing -fvisibility=hidden`). The requirement "Fortified
build" of `embedded-ruby-1-9-2` already demands that Ruby 1.9.2 passes its suites with
`-O2 -D_FORTIFY_SOURCE=3` given through `CFLAGS`; this change makes the build script ask for
it. Ruby 1.8.7 is built that way by Ubuntu's gcc in CI already, so its suites are expected
to pass; if one does not on this machine (Fedora gcc), the correction is a recorded patch as
for 1.9.2.

## Decisions

- **Flags.** `-U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=3 -fstack-protector-strong
  -fstack-clash-protection`, plus `-fcf-protection=full` when the target architecture is
  x86_64. They go into `base_build()` behind one named constant per architecture, with a
  comment. A flag the compiler rejects is a build error, not silently dropped.
- **Sanitizer and fuzz builds.** Where the crate builds the interpreter again for
  AddressSanitizer or libFuzzer, `_FORTIFY_SOURCE` is left out (it conflicts with ASan's
  interceptors); the rest may stay. Find where those builds take their flags and keep them
  apart from `base_build()` if they are not already.
- **Compiler in CI.** Set `CC: gcc-13` (the default of `ubuntu-24.04` today) at the job level
  of the gate and release jobs, with `gcc-13` in the installed packages. Choosing a newer
  compiler or clang is not part of this change.
- **Pins.** As resolved on 2026-10-15 with `gh api repos/<repo>/commits/<tag>`:
  `actions/checkout` v5 = `fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09`,
  `Swatinem/rust-cache` v2 = `6323deb102c322ba6fcbdcafc7e3dddab59af2b6`,
  `actions/upload-artifact` v4 = `ea165f8d65b6e75b540449e92b4886f43607fa02`.
  Write them as `uses: owner/name@<sha> # v5`. The OpenSpec CLI: `@fission-ai/openspec@1.14.2`
  (the version used on this machine). The flow sandbox has no network; use these values.
- **Cancelling.** `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`.
- **Scheduled workflow.** `.github/workflows/nightly.yml`: `schedule` (once a day) and
  `workflow_dispatch`; `permissions: contents: read`; separate jobs so one failure does not
  hide the others: `audit` (install `cargo-deny` at a fixed version with `--locked`, then
  `cargo xtask audit`), `stress` (`cargo xtask stress` with what docs/development.md says it
  needs: clang and its sanitizer runtimes, lavapipe), `sanitized` (the sanitized suites of
  `vkrgss-ruby-sys` as `docs/development.md` names them), `fuzz` (each target of
  `cargo xtask fuzz` with a short `--seconds`, e.g. 300, plus whatever the command needs
  installed). Same pins and `CC` as above. It cannot be run from the flow; correctness is
  checked by reading it and by the tests below.
- **Tests.** A test of `xtask` reads the workflow files and fails on a `uses:` that is not a
  40-hex commit hash, on an `npm install` of `@fission-ai/openspec` without a version, and on
  an unconditional `cancel-in-progress: true`. If `xtask` already has such a test module,
  extend it.

## Notes of the implementation

- **Flags.** `FORTIFY_FLAGS`, `HARDENING_FLAGS` and `X86_64_HARDENING_FLAGS` in both build
  scripts; `base_build()` adds the last only when `CARGO_CFG_TARGET_ARCH` is `x86_64`. The
  tests `tests/hardening.rs` of both crates read the arguments the build script gave the
  compiler (`VKRGSS_RUBY_CFLAGS`, `VKRGSS_RUBY192_CFLAGS`).
- **Sanitizer and fuzz builds.** Only `vkrgss-ruby-sys` has them, and they took the flags of
  the build script through `instrumented_compiler()` in `tests/common/mod.rs`. Its build
  script now also exports `VKRGSS_RUBY_FORTIFY_CFLAGS`, and `instrumented_compiler()` drops
  those arguments; the other hardening flags stay.
- **Check on this machine** (Fedora 44, gcc 16.2.1, clang 22.1.8 with `compiler-rt`):
  `cargo test -p vkrgss-ruby-sys -p vkrgss-ruby192-sys --locked`, the sanitized suites and
  all six steps of `cargo xtask stress` pass with the flags. No source correction was
  needed, so neither `PATCHES.md` changed.
- **Workflows.** Every job of the three workflows sets `CC: gcc-13` at the job level, the
  jobs that install packages install `gcc-13`, and the aarch64 job changed only so. The
  nightly `fuzz` job is a matrix over the three targets with `fail-fast: false`, each run
  for 300 seconds. `cargo-deny` is installed as `cargo-deny@0.18.2`: design.md names no
  version and the flow had no network, so it is a release known to exist, not the newest
  one. `xtask/tests/workflows.rs` checks the pins, the OpenSpec version, the conditional
  cancelling, `CC` in every job, the gate, the tag condition of the release and the jobs of
  the nightly workflow.

## Out of scope

Splitting the gate job, sccache, cargo-nextest, Dependabot, zizmor, release attestations,
pinning clang, a sanitized run of Ruby 1.9.2 (change `fuzz-and-review-ruby-1-9-2`).

## Follow-ups

- The nightly workflow could not be run from the flow. Its first run shows whether
  `libclang-rt-dev` brings the sanitizer runtimes and libFuzzer on `ubuntu-24.04` (the name
  `cargo xtask fuzz` gives) and whether `cargo-deny` 0.18.2 reads the current advisory
  database; a newer version is a change of the pin.
- The `fuzz` job keeps no input of a finding beyond the log; uploading
  `target/tmp/fuzz/<target>/artifacts/` on failure is not named here and was not built.
