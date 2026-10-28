# Design

## Context

See the proposal. This change runs after `confine-loading-and-process-functions` (preludes
and their tests), `call-rgss-from-script-threads` (`guest.rs` and the guest state) and
`share-the-interpreter-glue` (the sys crates and the bignum calls in `graphics.rs` and
`audio.rs`); it starts from the `main` they left and does not undo any of their edits. The
binding's tests run over both interpreters, so a move that changes behaviour shows.
`split-presentation-modules` and `move-core-tests-out-of-line` did the same for other crates.

## Decisions

### This is a move

Code moves between files; visibility is widened only as far as the move needs
(`pub(super)`, `pub(crate)`). No function body is rewritten, no public item is renamed, no
test is dropped or weakened. A defect found on the way goes under "Follow-ups".

### Intended modules

The names may be adjusted where a boundary turns out to lie elsewhere:

- `runtime/`: `link` (the pump and the link to the host), `prelude` (the prelude constants
  and their evaluation), `startup` (frame size and rate, rules, new scene, `RGSS_VERSION`,
  test mode globals), `scripts` (built-in scripts, archive loading, the script loop, reset),
  `report` (both report formats and their position parsers), with `mod.rs` keeping
  `RunConfig`, `play` and `run_game`.
- `win32api/`: `table` (the table and the shim plumbing), `profile` (the INI functions),
  `text` (code pages, `MultiByteToWideChar`, `WideCharToMultiByte`), `input` (keyboard,
  mouse, cursor), `system` (window, metrics, ticks, process times, memory), `rgss3` (the
  class of RGSS3), `tests`.
- `font/`: the shared accessors in `mod.rs`, the RGSS3 objects and defaults in `rgss3`,
  `tests`.

### Prelude line numbers

The task finds every reader of positions inside the preludes: tests that assert a backtrace
or an error report naming `(prelude)` with a line, the RGSS3 error report, the oracle
comparisons of error dialogs, and `$@` seen by a script. The result is written here under
"As built" with the evidence (file and line of each reader, or the search that found none).
Then:

- if something depends on the numbers, each padded file keeps its padding with a comment
  naming that reader, or the padding is replaced by evaluating the file from its starting
  line, as `PRELUDE_LOAD_RGSS3_LINE` does, whichever keeps every reported position equal;
- if nothing depends on them, the padding is removed and `PRELUDE_LOAD_RGSS3_LINE` follows
  the new layout or goes.

Either way the positions a game can observe are proved unchanged by a test, or the change in
them is recorded as harmless with that evidence.

### Size check

No non-test file under `crates/vkrgss-binding/src` SHALL be longer than 1000 lines after the
move (the spec says so); a test or an `xtask` check is not added for it, the scenario is
checked by counting.

## Follow-ups

None yet.

## As built

(Readers of prelude positions go here.)
