## Why

`run-rgss3-games-in-a-guest-of-their-own` started games of RPG Maker VX Ace with the
start-up, the endings, the error report and `load_data` measured on the original runtime
(`ace-v1-api-REPORT.md`, "the API report", sections 1 and 2, outside the repository), and
left a list of measured rules out (its design, "Follow-ups: left out of this change"). Row 9
of its table "the changes that follow" names this change for them. They are rules a script
can see: what `$0` is, what `caller` ends with, whether `at_exit` blocks run, what
`exit!(n)` does, which file `load_data("Data/Actors")` reads, what it raises for a
directory or a cut file, whether a frozen string is relabelled, and how `require_relative`
and `autoload` reach files. Two of them, the empty box of a long report and the exit code
after an error, are rules of the original that vkxp decides not to follow, which is recorded
with the reason. The rules of the reset key of the same list are split off into
`match-the-reset-key-of-rgss3`, because they touch `graphics.rs`, which
`bind-the-scene-classes-of-rgss3` is changing now.

## What Changes

In a game of RGSS3, and only there:

- **The program name.** The interpreter of a guest of RGSS3 is started without a script
  name, as the original's: `$0` and `$PROGRAM_NAME` are `false`, and the frame of the
  `eval` that runs a script is ``ruby:in `eval'``.
- **The title.** A `Game.ini` without a title gives the window the title `Untitled`.
- **Magic comments.** The measured cases that Ruby 1.9.2 already handles as the original
  (UTF-8, ASCII-8BIT, US-ASCII, UTF-16LE, an unknown name, a comment on line 2) are held by
  tests and their reports; `shift_jis` and `Windows-31J` stay with
  `read-strings-in-other-encodings`.
- **Endings.** `at_exit` returns its block and never runs it. `exit!(status)` ends the game
  at once without running an `ensure` clause, and the program's status is 0 for a status of
  0 or `true` and 1 otherwise. A script archive that cannot be loaded shows the modal
  message `Failed to load script.`. Kept as vkxp's own, with the reason recorded: a report
  of more than 1023 UTF-16 units is shown whole, and an uncaught exception ends the run
  with status 1 whatever its line.
- **`load_data`.** A loose file is handed to `Marshal.load` as an open `File` and an archive
  entry as a `String`, so the errors of a cut file are the original's; the proc labels a
  frozen string too; a directory raises `Errno::EACCES`; a name with a NUL byte or with
  bytes that are not UTF-8 raises as on 3.0.1, also for `save_data`; the archive is asked
  first, by the name as given or by the name without its extension, and names with `./`,
  `//`, `..` or a trailing dot fall through to the loose file; a trailing dot or space of a
  loose file name is dropped.
- **Loading code.** `require` and `load` hold their measured behaviour in RGSS3;
  `require_relative` and `autoload` follow the rule of `require` (only `.rb` files inside the
  game directory), which they bypass today; a required file is compiled as UTF-8.
- XP and VX are unchanged: every existing test passes over both interpreters with its
  expected values unedited.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: the program name, endings, the archive that cannot be loaded, the source,
  the names and the errors of `load_data` and `save_data`, and loading code, in RGSS3.
- `game-files`: the lookup of a data file of RGSS3 in the archive and among loose files.
- `embedded-ruby-1-9-2`: an interpreter started without a script name.
- `game-launch`: the title `Untitled`, the status after `exit!`.
- `supported-runtimes`: what `docs/compatibility.md` says of these rules.

## Impact

- `crates/vkxp-binding/src`: `runtime.rs` (the modal message of a failed load, `exit!`),
  `kernel.rs` (the data source, the relabelling function, `exit!`), `prelude_rgss3.rb`
  (`load_data`, `save_data`, `at_exit`, `require_relative`, `autoload`, the compiling of a
  required file), `error.rs` (`Errno::EACCES`, the conversion error of a name).
- `crates/vkxp-vfs/src/files.rs` (and `archive.rs` if the lookup lives there): the lookup of
  a data file of RGSS3, with unit tests.
- `crates/vkxp-ruby192-sys/src/interp.rs`: `Options::for_rgss(3)` and an empty script name.
- `crates/vkxp-guest-main/src/lib.rs`: the title in `guest_inputs`.
- Tests: `crates/vkxp-binding-rgss3/tests/` (new tests in its own RGSS3 files, new file
  `loading_rgss3.rs`), `crates/vkxp-ruby192-sys` (a test of the option),
  `crates/vkxp/tests/rgss3.rs` (`exit!`, `at_exit`, the failed load, the archive lookup).
- Documents: `docs/compatibility.md` (section "RPG Maker VX Ace"), `docs/running.md` where
  it describes the endings of VX Ace, `CHANGELOG.md`.
- In flight on the same files: `bind-font-and-text-of-rgss3` (`prelude_rgss3.rb`,
  `runtime.rs`, `vkxp-guest-main`), `bind-audio-of-rgss3` (`runtime.rs`,
  `crates/vkxp-vfs/src/files.rs`), `bind-the-scene-classes-of-rgss3` (the documents). Each
  touches other functions of those files; the later merge rebases.
- Not touched: the protocol, the host crates, `crates/vkxp/src`, `graphics.rs`,
  `input.rs`, `win32api.rs`, `prelude.rb`, `prelude_shared.rb`, `vkxp-ruby-sys`,
  `third_party/`, `ruby192.allow`.
