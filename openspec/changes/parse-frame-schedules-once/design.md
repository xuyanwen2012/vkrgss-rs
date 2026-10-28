# Design

## Context

See the proposal. `ScriptError` has `MissingColon`, `InvalidFrame`, `UnknownButton`,
`EmptyButton`, `DuplicateFrame`; `ScheduleError` has `MissingColon`, `InvalidFrame`,
`UnknownRequest`, `InvalidSlot`. Their messages differ in the noun: "script entry `…`" and
"states entry `…`". Both are public types of `vkrgss-host` used by `crates/vkrgss` to print
argument errors.

## Decisions

### A shared reader of entries

A crate-private module (for example `frames.rs`) has:

```rust
pub(crate) struct Entry<'a> { pub frame: u64, pub rest: &'a str, pub text: &'a str }
pub(crate) enum EntryError { MissingColon, InvalidFrame }
pub(crate) fn entries(text: &str) -> impl Iterator<Item = Result<Entry<'_>, (EntryError, &str)>>
```

(shape may be adjusted). It splits on `,` and `\n`, trims, skips empty entries, splits at
the first `:`, trims and parses the frame as `u64`. Each parser maps `EntryError` to its own
variant with the entry text, so the public error enums and their messages do not change.

### Messages stay

The `#[error]` strings of both enums are not touched. A test pins each message of the shared
variants for both parsers (`MissingColon`, `InvalidFrame`) before the change, so the move
cannot alter them.

### Not in this change

The parsing after the colon (button lists, `save`/`load` with a slot), duplicate-frame
handling and the order of entries stay in their modules. The two module docs keep their
syntax descriptions; the shared rule is described once in the new module and referred to.

## Follow-ups

None yet.
