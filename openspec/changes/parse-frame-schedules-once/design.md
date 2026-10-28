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

- Task 1.1 names `cargo test -p vkrgss-host script states`, which cargo refuses (one test
  name filter only); the tests were run as `cargo test -p vkrgss-host --lib -- script states`.
- The first gate run failed once in `crates/vkrgss/tests/rgss3.rs`,
  `a_copy_of_the_program_alone_names_the_missing_guest_executable`, with "Text file busy"
  (`ETXTBSY`) when it ran its fresh copy of the program: a test that forks while another
  thread still holds the copy open for writing. Outside this change; the second run passed.
- In this sandbox `cargo xtask xarch` needs `XDG_RUNTIME_DIR` set to a writable directory,
  since vkrgss cannot create its directory under `/run/user/1000`.
