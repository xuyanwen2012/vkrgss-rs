# Harden the C build and pin the inputs of CI

## Why

The modernization audit of 2026-10-14 found two problems around the build.

The released executables carry `_FORTIFY_SOURCE=3`, a stack protector and CET only because
Ubuntu's gcc turns them on by default. The build scripts of both interpreters
(`crates/vkrgss-ruby-sys/build.rs`, `crates/vkrgss-ruby192-sys/build.rs`) do not ask for
them, so a build on Fedora or with clang has none of them, and the compiler that builds the
release is whatever the runner image calls `cc`. For an interpreter with a conservative
collector, the compiler also shapes stack frames and so what the collector sees.

The checks outside the gate (`cargo xtask audit`, `cargo xtask stress`, the sanitized
suites, `cargo xtask fuzz`) run only when someone runs them by hand. The workflows install
`@fission-ai/openspec` without a version, refer to actions by movable tags instead of commit
hashes, and `cancel-in-progress: true` also cancels the check of the previous push to
`main`.

## What Changes

- Both build scripts pass the hardening flags themselves: `-D_FORTIFY_SOURCE=3` (after
  `-U_FORTIFY_SOURCE`), `-fstack-protector-strong`, `-fstack-clash-protection`, and on
  x86_64 `-fcf-protection=full`, for gcc and clang alike. Builds for the sanitizers and the
  fuzz targets leave out what conflicts with them.
- The workflows name the C compiler explicitly (`CC`), the one the runner image uses today,
  so a change of the image's default does not change the compiler of the release silently.
- Actions are referred to by commit hash with the tag as a comment; the OpenSpec CLI is
  installed at a fixed version.
- `cancel-in-progress` cancels only runs of pull requests, not of `main`.
- A scheduled workflow runs the checks outside the gate once a day and by hand: the
  dependency audit, the collector stress, the sanitized suites of Ruby 1.8.7 and a short run
  of each fuzz target. Its failure does not block merging.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the build script asks for the hardening flags.
- `embedded-ruby-1-9-2`: the same.
- `workspace-structure`: pinned inputs of the workflows, cancelling only pull requests, and
  the scheduled workflow.

## Impact

`crates/vkrgss-ruby-sys/build.rs`, `crates/vkrgss-ruby192-sys/build.rs`, `.github/workflows/`,
`xtask` (tests that read the workflow files, if any are added), `docs/building.md`,
`docs/development.md`, `CHANGELOG.md`. Behaviour of games could change only through the
compiler's stack layout under the conservative collector; the owner reruns the comparison
with the original runtime after the merge.
