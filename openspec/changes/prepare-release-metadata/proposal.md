# Package metadata and the version option for the first release

## Why

The first public release needs what a reader of a published Rust project expects: the
manifests say what each crate is and where the project lives, and the executable can say
which version it is. `vkxp --version` is an unknown option today.

## What Changes

- Workspace metadata: `repository`, `homepage`, `readme`, `keywords`, `categories`; every
  crate has a `description`.
- `vkxp --version` and `-V` print the name and the version and exit with status 0.
- A pass over the crates for leftovers that should not be published: debugging output,
  commented-out code, dependencies no crate uses.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: package metadata.
- `game-launch`: the version option.

## Impact

`Cargo.toml`, the crates' manifests, `crates/vkxp` (arguments), `Cargo.lock` only if a
dependency is removed. No frame changes.
