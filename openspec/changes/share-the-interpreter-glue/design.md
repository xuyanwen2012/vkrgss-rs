# Design

## Context

- `crates/vkrgss-ruby-sys/src` and `crates/vkrgss-ruby192-sys/src`: `root.rs` identical,
  `host.rs` differs in comments only, `protect.rs` 55 differing lines, `call.rs` 126,
  `interp.rs`, `signals.rs` and `ffi.rs` are interpreter-specific.
- Runs after `call-rgss-from-script-threads`, which changes how host functions find the
  guest and the SAFETY comment of `host.rs` that says Ruby calls methods on the interpreter
  thread only. Move the code as that change left it; do not reintroduce the old comment.
- Requirement "The two interpreters are never linked together" (embedded-ruby-1-9-2): the
  glue crate has no C code and no `#[no_mangle]` symbol, so both sys crates may depend on it.

## Decisions

- **A crate generic over a trait.** `vkrgss-ruby-glue` defines `trait Interpreter` with what
  the shared code needs from the FFI: `VALUE`, the nil/false/true constants, `rb_protect`,
  `rb_errinfo`/setting it, `rb_jump_tag`, `rb_exc_raise`, `rb_define_method`-style
  registration, `rb_gc_register_address`, string and array accessors. Each sys crate
  implements it on a unit type and re-exports the shared items under their present paths so
  that `vkrgss-binding` compiles unchanged except for imports. Trampolines stay `extern "C"`
  functions, monomorphised per interpreter. If the trait cannot express a piece without a
  cost in the hot path (measured with `--frame-stats` on a game, not guessed), that piece
  stays duplicated and design.md says which and why.
- **What moves.** `root.rs` and `host.rs` entirely; from `protect.rs` and `call.rs` whatever
  is the same algorithm. `interp.rs`, `signals.rs`, `ffi.rs` stay where they are.
- **L1.** `protect` restores the previous error with `rb_set_errinfo` only when it is nil or
  an exception. Otherwise (a pending `throw`) it restores it in the way the interpreter itself
  does for its tag jumps (1.9.2: `th->errinfo` through a small accessor in the C glue or
  `rb_protect` around the restore); record which.
- **L2.** `TAG_FATAL` is resumed with `rb_jump_tag(TAG_FATAL)` after putting the fatal error
  back as the current error, so it is not rescuable, in both interpreters.
- **Bignums.** `rb_ll2inum` for values up to `i64::MAX`; the counters cannot pass it.
- **Unsafe.** The new crate allows `unsafe` at its root with the workspace lints; every block
  keeps its SAFETY comment. Update the unsafe list in `docs/architecture.md`.
- **Same behaviour.** Every existing test of both sys crates and both binding packages passes
  without change; the upstream Ruby suites and the stress tests are part of the gate already.

## Follow-ups

None yet.
