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

- Outside this change: `a_copy_of_the_program_alone_names_the_missing_guest_executable` of
  `crates/vkrgss/tests/rgss3.rs` failed once in a gate run of 2026-10-27 with `ETXTBSY`
  (`Text file busy`) when it started the copy of the program it had just written; it passed
  in the gate before and in three runs of the file alone after. It looks like the known
  race of a copied executable with a fork of another test thread that inherits the
  descriptor it was written through. Not examined further here.

## As built

### Modules

- `runtime/`: `link` (the link, the pump, the modal message, the copies for states and the
  raise of what the host asked for, with `Copier` and its results), `prelude`, `startup`,
  `scripts`, `report`; `mod.rs` keeps `RunConfig`, the guest state of the run (`State`),
  `RunError`, `play` and `run_game`, and re-exports what other modules named before, so no
  path outside `runtime/` changed. The preludes stay in `src/` and are included from there.
- `win32api/`: as intended; `mod.rs` keeps the state of the emulated functions and the host
  functions of RGSS1 and RGSS2 (`vkrgss_win32api`, `vkrgss_win32api_names`, `define`), and
  `table` has the shim plumbing (`Arg`, `find`, the argument readers, `run_shim`).
- `font/`: as intended; `install_defaults` and `define` stay in `mod.rs`, because they serve
  every runtime.

No file under `crates/vkrgss-binding/src` is longer than 887 lines (`bitmap.rs`).

### Readers of prelude positions

A position inside a prelude is a backtrace entry `(prelude):<line>`: the preludes are
evaluated under the file name `(prelude)` (`PRELUDE_NAME` of `runtime/prelude.rs` and of
`script.rs`, and the proc of `load_data` in `kernel.rs`). The search, on the tree of
2026-10-27 after tasks 1 and 2:

- Tests: `grep -rn -E "\(prelude|prelude\)" crates/*/tests crates/*/src xtask/src docs`,
  leaving out the two interpreter crates, finds only the three definitions of the name
  above; no test of any crate asserts a backtrace, a report, `$@` or a message that holds
  `(prelude)`. The tests that look at positions (`report_rgss3.rs`, `builtin_rgss3.rs`,
  `run_game_rgss3.rs` with `caller(0)`, `rgss3.rs` and `end_to_end.rs` of `vkrgss`,
  `script.rs`) assert positions in scripts, `(builtin)` and `ruby`, never in a prelude; a
  search for `no such file to load`, ``in `require'`` and ``in `load'`` finds messages
  only, without positions.
- The error reports: `report` (RGSS1 and RGSS2) takes the first position that lies in a
  script and falls back to the first position at all; `report_rgss3` takes the first entry
  that does not start with `(eval)`. Either can name `(prelude)` with a line when an
  exception leaves a function of a prelude, but nothing compares that text: no test does
  (above), and the original runtime has no prelude, so its reports never hold such a
  position.
- The oracle comparisons (`xtask/src/oracle`) compare frames and save files with those of
  the original; no probe (`probe*.rb`) reads a backtrace, `$@` or `caller`. A position in a
  prelude is vkrgss's own and has no counterpart there. `grep -rIl "(prelude)"` over
  `ref/tools` and the reports of `ref/` finds only the generated prelude of Ruby 1.9.2
  itself (`ref/tools/ruby192-feasibility/prelude.c.generated`), which is not ours.
- `$@` seen by a script, and `caller`: `prelude_load_rgss3.rb` reads `caller(1)[0]` for
  `require_relative`, and takes only the file part of it. No game of the comparison list
  of `cargo xtask xarch` is known to read a line of a prelude; `xarch check` is what shows
  that none of them draws or saves differently.

Nothing reads the numbers. The padding of `prelude_shared.rb` (kept since the change
`emulate-win32api-and-dl-over-ruby-1-9-2`) and of `prelude_rgss3.rb` (since
`run-a-ruby-script`) kept them only so that no position moved, and both records say no
more than that.

### Prelude line numbers

The padding is removed: lines 5 to 29 of `prelude_shared.rb` and 48 to 95 of
`prelude_rgss3.rb`. `PRELUDE_LOAD_RGSS3_LINE` goes, and `prelude_load_rgss3.rb` is
evaluated from line 1 like every other prelude. The positions a game can observe in a
prelude move: those of `prelude_shared.rb` after line 29 by 25 lines up, those of
`prelude_rgss3.rb` after line 95 by 48 lines up, and those of `prelude_load_rgss3.rb` by
47 lines up. The change is harmless by the search above: no test, report check or oracle
comparison reads them, the original has none of them, and `xarch check` shows every
comparison game identical (task 4.1).

### Integration check

On 2026-10-27: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` passed every step;
`cargo test -p vkrgss-binding -p vkrgss-binding-rgss3` ran the same 947 tests before the
move and after each task group, all passing; `cargo xtask xarch baseline --at 84a98559`
(the proposal, before any move) and `cargo xtask xarch check` gave every one of the ten
games identical, 84 frames and 17 saves. The guest processes of the RGSS3 games put their
socket under `TMPDIR` for these runs (`XDG_RUNTIME_DIR` unset), because the sandbox of the
run refuses a directory in `/run/user`.
