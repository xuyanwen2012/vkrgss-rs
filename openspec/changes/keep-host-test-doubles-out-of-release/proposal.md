# Proposal

## Why

`vkxp-host` exports two modules that only tests and examples use: `testing` (a recording
display, a recording audio sink, a readable error stream) and `demo` (a guest written in Rust
that stood in for a game before the Ruby side existed). Both are compiled into the library
the executable links, and both are part of its public interface.

## What Changes

- The two modules are compiled only with a feature of `vkxp-host` named `testing`. The
  executable does not enable it; the crate's own tests and examples do.

Out of scope: changing what the doubles or the demo guest do; `vkxp-binding`'s own test
support; removing the demo.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: test doubles are not part of the release library.

## Impact

- `crates/vkxp-host` (`Cargo.toml`, `src/lib.rs`, the uses inside `#[cfg(test)]` code), and
  the README where it says how to run the demo examples.
