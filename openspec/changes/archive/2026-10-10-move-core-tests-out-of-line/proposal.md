# Proposal

## Why

The large files of `vkxp-core` are mostly tests: `extract.rs` has 3104 lines of which about
590 are production code, `window.rs` 2461 of which about 550, `bitmap.rs` 1710 of which about
600, `tilemap.rs`, `scene.rs` and `text.rs` likewise. Reading or reviewing the production code
means scrolling past the tests, and a change to either shows up as a change to the same file.
`vkxp-host` already keeps a module's tests in `src/<module>/tests.rs`.

## What Changes

- In `vkxp-core`, the inline `mod tests { .. }` of a source file with more than about 300
  lines of tests moves to `src/<module>/tests.rs`, declared as `#[cfg(test)] mod tests;`.
- Test-only helpers that sit between production items in those files move with the tests
  where nothing else uses them.

Out of scope: any change to what a test checks, to test names, or to production code beyond
the `mod` declaration and imports; other crates; splitting production modules.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: where the unit tests of `vkxp-core` live.

## Impact

- `crates/vkxp-core` only. Moved lines; no behaviour changes.
