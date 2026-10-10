# Design

## Context

- `crates/vkxp-ruby-sys/src/protect.rs`: `pub(crate) unsafe fn protect<T>(data: &mut T, body)`
  calls `rb_protect`, and on a non-zero state takes the pending exception out of
  `ruby_errinfo` and returns `Raised { state, exception }`, the exception only for a raise or
  a fatal error. `Raised` is already public.
- `crates/vkxp-binding/src/rb.rs`: `perform` does the same around `call_body` with its `Call`
  record, statement for statement.

## Decisions

- `protect` becomes `pub`, re-exported from the crate root, with its `# Safety` section
  covering what a caller in another crate must hold (interpreter thread, the body casts the
  argument back, does not unwind, holds no value with a destructor across a call that can
  raise). The crate-level documentation that lists the rules for calling into Ruby mentions
  it.
- `perform` calls it. Its own `rb_protect` call and the handling of `ruby_errinfo` go.
- A test in `vkxp-ruby-sys` calls the exported function from outside the crate (an
  integration test) with a body that raises and one that returns.

## Risks / Trade-offs

- [A wider unsafe surface of the crate] → It is one function with a written contract, and the
  alternative is the same code held twice.
