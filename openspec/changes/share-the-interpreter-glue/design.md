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

## What stays per interpreter

- **Everything of `root.rs`, `host.rs`, `call.rs` and `protect.rs` moved.** The glue holds
  `Ruby<I>` and `Root<I>` with every method of the former `impl Ruby` blocks, the
  trampolines, the definitions, `protect`, the method call, evaluation, the description of
  an exception and the string helpers. Where the two copies differed, the difference is an
  item of the trait: the current error (`errinfo`, `restore_errinfo`, `set_errinfo`: a
  variable of Ruby 1.8.7, functions of 1.9.2), whether a jump that is not a raise keeps what
  it carries in the current error (`JUMP_IN_CURRENT_ERROR`, which decides whether `protect`
  puts the previous error back after such a jump and whether `describe` clears it), a string
  of text (`rb_str_new` against a UTF-8 string), the main object and the mark of the game's
  scripts. The type tags of strings and arrays are constants of the trait, since they
  differ. Nothing stayed duplicated for a cost in the hot path: every item is a direct call
  or a constant in code monomorphised per interpreter, so no measurement with
  `--frame-stats` was needed.
- **`ffi.rs`, `interp.rs`, `signals.rs`** stay, as designed.
- **`capi.rs`** of each sys crate: the unit type `CApi` and its implementation of the trait.
- **`protect.rs`** of each sys crate: `protect<T>` for its interpreter, under the present
  path, because the generic `protect<I, T>` of the glue cannot be named as `protect::<()>`,
  as the callers and the tests do. Ruby 1.8.7 also keeps `call_void`, which only its
  finalisation uses.
- `Ruby` and `Root` of each sys crate are type aliases of the glue's types for `CApi`, so
  `Ruby::eval` and the other paths of the tests still name the methods. `Ruby::new` is
  public in the glue, still `unsafe`, because the interpreter crate makes the `Ruby` it hands
  out. `Ruby::str_new_utf8`, which only 1.9.2 had, exists over 1.8.7 too, as the string of
  `str_new`. The trait has no variadic function, so `describe` and `inspect` call a method
  without arguments with `rb_funcall2` and a null argument vector, which is what
  `rb_funcall` passes for no arguments in both interpreters.

## Follow-ups

None yet.
