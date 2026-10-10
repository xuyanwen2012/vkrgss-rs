# Design

## Context

- `crates/vkxp-core/src`: `extract.rs`, `window.rs`, `tilemap.rs`, `bitmap.rs`, `text.rs`,
  `scene.rs` each end in a `#[cfg(test)] mod tests`; some have a second `#[cfg(test)]` item
  earlier (a helper or a hook). The other files are smaller; apply the same rule to them by
  the size of their tests.
- `tilemap` already has a directory (`src/tilemap/`). A file `foo.rs` and a directory `foo/`
  can coexist; `mod tests;` inside `foo.rs` resolves to `foo/tests.rs`.
- The test count of the crate is printed by `cargo test -p vkxp-core`.

## Decisions

- Rule: a file whose `mod tests` body is longer than about 300 lines gets `src/<module>/tests.rs`.
  Shorter test modules stay inline; moving them gains nothing.
- The move is textual: the body of `mod tests { .. }` becomes the file, one indentation level
  less (rustfmt does that). `use super::*;` keeps meaning the parent module.
- A `#[cfg(test)]` helper outside `mod tests` moves into the tests file when only tests use
  it. A `#[cfg(test)]` method on a production type, or a hook that production code calls
  under `cfg(test)`, stays where it is.
- No test is renamed, removed, split or edited. If a moved test needs a visibility change to
  compile, something was not a pure move; find out what instead of widening visibility.
- The one edit a moved test body needs: `include_str!` resolves its path against the file
  that holds it, so each `include_str!("../..")` of a moved test gains one `../` (nine
  lines in `extract`, `window`, `bitmap`, `text` and `plane`). Nothing else differs from the
  former body but the indentation and the lines rustfmt joins at the new width.
- The rule selects `extract`, `window`, `tilemap`, `bitmap`, `text`, `delta`, `plane`,
  `scene`, `table` and `input` (341 to 2392 lines of tests). `geometry` and `color` (about
  245 each) and the smaller ones stay inline. The modules `extract::testing` and
  `bitmap::testing` stay where they are: the tests of other modules use them.
- One commit per moved module, so `git log --follow` and `git blame -C` keep working.

## Risks / Trade-offs

- [A test silently drops out] → Compare the list of test names before and after
  (`cargo test -p vkxp-core -- --list`), not only the count.

## Follow-ups

- The same for other crates with large inline test modules, if wanted.
