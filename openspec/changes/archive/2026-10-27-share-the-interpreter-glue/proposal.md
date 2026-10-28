# Share the interpreter glue

## Why

The whole-project review of 2026-10-26 (`target/review-2026-10-26/fable-review.md`, design
section and L1, L2) found:

- About 2.5k lines of Rust that bridge Rust and Ruby are kept twice, once in
  `vkrgss-ruby-sys` (Ruby 1.8.7) and once in `vkrgss-ruby192-sys` (Ruby 1.9.2): `root.rs` is
  identical, `host.rs` differs in three doc comments, `protect.rs` and `call.rs` are parallel
  copies. A fix to one is easily forgotten in the other; only layout tests notice drift.
- L1: the `protect` of Ruby 1.9.2 puts the previous error back with `rb_set_errinfo` outside
  the protection. After a `throw` that error is not an exception, `rb_set_errinfo` raises
  `TypeError`, and the jump crosses a Rust frame.
- L2: a `TAG_FATAL` that a host function resumes is raised again as an ordinary exception,
  so `rescue Exception` catches what Ruby does not let a script catch.
- Two `Bignum`s are made by evaluating their decimal text (`Graphics.frame_count`,
  `Audio.*_pos` byte counts) although both interpreters export `rb_ll2inum`.

## What Changes

- A crate `vkrgss-ruby-glue` holds the interpreter-independent bridge (host functions and
  their trampolines, rooting, the protected call and the method call helpers), generic over a
  small trait that each sys crate implements with its own FFI. Behaviour is unchanged.
- A pending jump that is not an exception passes through `protect` without being put back
  through `rb_set_errinfo`; `TAG_FATAL` is resumed with `rb_jump_tag`, in both interpreters.
- The binding makes `Bignum`s with `rb_ll2inum`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`, `embedded-ruby-1-9-2`: a pending `throw` and a fatal error crossing a host
  function.
- `workspace-structure`: the new crate in the crate layout.

## Impact

`crates/vkrgss-ruby-glue` (new), `crates/vkrgss-ruby-sys`, `crates/vkrgss-ruby192-sys` (code
moves out, trait impls stay), `crates/vkrgss-binding` (`graphics.rs`, `audio.rs`; imports if
paths change), `Cargo.toml`, `Cargo.lock`, `xtask/src/deps.rs` (the new crate is a guest
crate), `docs/architecture.md` (crate table, unsafe list), `docs/development.md`,
`CHANGELOG.md`. No picture changes.
