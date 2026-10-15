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

## Out of scope

Splitting the gate job, sccache, cargo-nextest, Dependabot, zizmor, release attestations,
pinning clang, a sanitized run of Ruby 1.9.2 (change `fuzz-and-review-ruby-1-9-2`).

## Follow-ups

None yet.
