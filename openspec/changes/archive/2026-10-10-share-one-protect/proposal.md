# Proposal

## Why

Running C code of the interpreter so that a raised exception ends in a status instead of a
jump through Rust frames is the most delicate piece of `unsafe` in the project. It exists
twice: `protect` in `vkxp-ruby-sys` and a copy of the same thirty-five lines in `perform` of
`vkxp-binding`, because the first is private to its crate.

## What Changes

- `vkxp-ruby-sys` exports its protected call, with its safety contract, and `vkxp-binding`
  uses it; the copy goes.

Out of scope: any other change to how the binding calls the interpreter.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the protected call is offered to other crates.

## Impact

- `crates/vkxp-ruby-sys` (`src/protect.rs`, `src/lib.rs`), `crates/vkxp-binding` (`src/rb.rs`).
