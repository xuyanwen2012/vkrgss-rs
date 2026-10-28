# Dispatch the window once

## Why

A `Window` of `vkrgss-core` (`element.rs:297`) carries the fields of RGSS1 and
`rgss2: Option<window::rgss2::State>`, whose `State` carries `rgss3: Option<window::rgss3::State>`.
Which runtime a window belongs to is therefore read from two nested options, and it is read
again in every place that behaves by runtime: `Window::call` checks `rgss2.is_none()` and
then `is_rgss3()` before its own RGSS2 match (`window/rgss2.rs:347-443`), `call_rgss1`
(`:478-499`) and `call_rgss3` (`window/rgss3.rs:156-265`) each enumerate the same
`WindowCall` variants, `refresh_caches` (`window.rs:479`), the assembly of a window
(`window/rgss2.rs:632-635`) and the accessors of `rgss3.rs` (`:115-142`) each unwrap the
options again. About thirty such checks exist; a fourth runtime rule or a new call has to be
added in all of them, and a window of RGSS3 whose `rgss3` is `None` cannot be ruled out by
the types.

## What Changes

- The runtime of a window is one enum field with a variant per runtime, each holding exactly
  the state that runtime keeps; RGSS3 holds the RGSS2 state and its own.
- `Window::call` chooses the variant once and hands the call to one function per runtime;
  the other places that behave by runtime match the enum instead of unwrapping options.
- Nothing a caller of `vkrgss-core` can see changes, and no frame changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: the runtime of a window is chosen in one place.

## Impact

`crates/vkrgss-core/src` only (`element.rs`, `window.rs`, `window/rgss2.rs`,
`window/rgss3.rs`, `scene.rs` where windows are made, and their tests). The binding calls
the public `Window` API, which keeps its signatures; if a constructor's arguments change, the
calls in `crates/vkrgss-binding/src/window.rs` follow. No protocol, shader or picture
changes.
