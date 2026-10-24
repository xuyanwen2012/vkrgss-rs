# Design

## Context

- `xtask/src/dist.rs` has `const TARGET: &str = "x86_64-unknown-linux-gnu"`; its tests pin the
  name; it refuses a shared libcrypt in each executable.
- `.github/workflows/release.yml` has one job `package` on `ubuntu-24.04` that builds the Linux
  package and the APK and, for a pushed tag, creates the release with `gh release create`.
- CI already runs the gate on `ubuntu-24.04-arm` (job `gate-aarch64` of `ci.yml`), with the same
  apt packages.

## Decisions

- **Target.** Chosen from `std::env::consts::ARCH` at compile time of `xtask` (the host), with
  `linux` required; tests cover the name for both arches through a function that takes the arch.
- **Workflow.** Split: a matrix job builds the Linux package on both runners (each uploads its
  archive as an artifact named after its target); the APK stays on x86_64; a final job that
  needs them downloads the artifacts and creates the release with every asset, only for a pushed
  tag, as now. The tag check runs before anything is published. Pinned action SHAs as now.
- **Notes.** One requirements paragraph per target (both Ubuntu 24.04, glibc 2.39).
- **Outside the listed documents.** `docs/releasing.md` describes the assets of a release and
  the command; it named only the x86_64 archive and said "two files", which would be wrong
  after this change. It is edited in a few lines (the targets, `<target>` in the file names,
  the refusal on another machine), as the requirement plainly needs.
- The workflow cannot be run here; it is checked by reading, by `actionlint` if available, and
  by the YAML parser test of `xtask` if one exists. The owner's session runs it by hand
  (`workflow_dispatch`) after the merge.

## Out of scope

musl or older-glibc builds, other architectures, macOS packages in the release.

## Follow-ups
