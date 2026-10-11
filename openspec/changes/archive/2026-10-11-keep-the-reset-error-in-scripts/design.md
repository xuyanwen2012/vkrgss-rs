# Design

## Context

`vkxp_ruby_sys::protect` takes the pending exception out of `ruby_errinfo` and sets the
variable to nil. `run_scripts` in `vkxp-binding/src/runtime.rs` recognises `Reset`, calls
`reset` and starts over at the first script with `$!` nil.

Ruby 1.8 restores `ruby_errinfo` to the value it had on entry when a `begin ... rescue` ends,
so a value set before the scripts run stays until a script assigns `$!` or raises out of the
script; that is what the measurement shows on the original (run 3 still sees the `Reset` of
the `Graphics.update`).

## Decisions

### The loop over the scripts sets the error after a reset

After `reset(ruby)` has succeeded, `run_scripts` sets `ruby_errinfo` to the `Reset`
exception it holds, through a small safe-to-call function of the interpreter wrapper (the
`unsafe` stays in `vkxp-ruby-sys` or the wrapper that owns the other accesses). Before the
first run nothing is set.

As built: the function is `Ruby::set_current_error`, in `vkxp-ruby-sys` next to the other
calls. It is an `unsafe fn` like `funcall` and `describe`, because the value it takes must be
`nil` or a live exception; the access to `ruby_errinfo` itself stays in `vkxp-ruby-sys`. The
test harness of `vkxp-binding` plays many games on one interpreter, so its `run_game` clears
the current error with the same function before each game, which is what a new process is.

### A protected call puts back the error it found

`protect` restores `ruby_errinfo` to the value it had before the call instead of nil. For
every existing caller that value is nil unless a reset has happened or a script is inside a
`rescue`, in which case Ruby's own C functions would leave the variable alone too. This keeps
the `Reset` in place when native code takes an exception and does not raise it again.

## Risks

- A game that tests `$!` for nil at its start after F12 sees the `Reset`; so it does on the
  original.

## Follow-ups

None yet.
