# Continuous integration and the release package

## Why

The repository is about to be published on GitHub. A published project shows that its checks
pass on a machine other than the author's, and a release offers something to download. Today
the gate runs only locally and a release would have to be assembled by hand.

## What Changes

- `cargo xtask dist` builds the release executable and packs it with the licence texts into
  one archive with a checksum.
- A GitHub Actions workflow runs the merge gate on every push and pull request.
- A second workflow builds the package for a version tag and publishes the GitHub release;
  started by hand it only builds and keeps the package as an artifact.
- Issue and pull request templates; `docs/releasing.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: the release package command and the workflows.

## Impact

`xtask` (new module), `.github/`, `docs/releasing.md`, one line in `docs/development.md` and
`README.md` that link to it. No code of the runtime changes.
