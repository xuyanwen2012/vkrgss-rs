# Design

## Context

See the proposal. The window tests of `vkrgss-core` (`window/tests.rs`,
`window/rgss2/tests.rs`, `window/rgss3/tests.rs`, the window cases of `test_support`) compare
every window picture with committed expectations and with the reference compositor, so a
restructuring that changes behaviour shows. `cargo xtask xarch check` covers the ten games.

## Decisions

### One enum for the runtime of a window

```rust
pub(crate) enum Kind {
    Rgss1,
    Rgss2(rgss2::State),
    Rgss3(rgss2::State, rgss3::State),
}
```

(name and exact shape may be adjusted; the point is that each runtime's state is present
exactly when the window is of that runtime). `Window::rgss2` is replaced by this field. The
RGSS2 state no longer carries `rgss3`. Helpers such as `fn rgss2_state(&self) ->
Option<&rgss2::State>` (Some for RGSS2 and RGSS3) may remain where code genuinely shares the
RGSS2 behaviour, so the RGSS3 path is not duplicated.

### One dispatch per entry point

`Window::call` expands `Move` as today, then matches the kind once:
`Kind::Rgss1 => self.call_rgss1(call)`, `Kind::Rgss2(_) => self.call_rgss2(bitmaps, call)`,
`Kind::Rgss3(..) => self.call_rgss3(bitmaps, call)`. The same holds for `refresh_caches` and
the assembly of a window. The three per-runtime functions keep their bodies; only the way
they reach their state changes (a borrowed `&mut State` instead of `if let Some(state) =
&mut self.rgss2`).

The field borrow of `self.kind` against `&mut self` calls inside the per-runtime functions
may need the state taken by `std::mem::replace` or split borrows; whichever is chosen, no
allocation per call is added.

### This is a restructuring, not a rewrite

No rule of any runtime changes, no test is dropped or weakened, and the committed
expectations are not re-blessed. A defect found on the way is listed under "Follow-ups" and
left as it is.

## Risks

- A latch order that depended on the order of the nested checks could change: guarded by the
  per-runtime window tests and `xarch check`.

## Follow-ups

None yet.
