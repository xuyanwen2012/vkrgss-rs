# Design

## Context

Row 9 of the table "Follow-ups: the changes that follow" of the design of
`run-rgss3-games-in-a-guest-of-their-own` (archived 2026-10-13) names this change, with the
measured sources API report 1.1, 1.2, 1.4, 1.6, 2.1, 2.2 and the core API "none". Its list
"Left out of this change, with the measured source" holds the items; those of 1.4 (the
reset key) are split off into `match-the-reset-key-of-rgss3`, which comes after
`bind-the-scene-classes-of-rgss3` because both change `graphics.rs`. This change keeps the
rest, so that it can start at once.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{runtime,kernel,error}.rs`, `prelude_rgss3.rb`,
  `prelude_shared.rb`, `crates/vkxp-vfs/src/{files,archive}.rs`,
  `crates/vkxp-ruby192-sys/src/interp.rs`, `crates/vkxp-guest-main/src/lib.rs`
  (`guest_inputs`), `crates/vkxp-binding/src/testing/host.rs`,
  `crates/vkxp/tests/{rgss3.rs,support/ace.rs}`.
- The design of `run-rgss3-games-in-a-guest-of-their-own`, decisions 9 to 13 and its
  Follow-ups; the requirements "... of RGSS3" of `openspec/specs/rgss-runtime`.
- The measurements, outside the repository: `../../ref/tools/oracle/measurements/
  ace-v1-api-REPORT.md` ("the API report", sections 0.2, 1.1, 1.2, 1.6, 2.1 to 2.3, 3.3, 8)
  with its sample files `ace-v1-api-cases-samples.301.txt`, `-data-samples`,
  `-data2-samples`, `-arc-samples`; `ace-v0-REPORT.md` ("the V0 report", section 8, row
  `require`, `load`); `../../ref/RGSS301_api.md`.

What vkxp does today in a game of RGSS3, against the measurement:

| Item | Original (source) | vkxp today |
|---|---|---|
| `$0`, `$PROGRAM_NAME` | `false` (1.1, V0 8) | `"vkxp"`, the script name of the interpreter crate |
| last entry of `caller` in a script | ``ruby:in `eval'`` (1.1) | ``vkxp:in `eval'`` |
| title without `Title=` | `Untitled` (1.1) | empty |
| report longer than 1023 UTF-16 units | an empty box (1.2) | the text |
| exit code after an error | the line number in 69 of 98 cases, 0 or 1 in the others; "the meaning is not established" (1.2, 8 point 3) | 1 |
| archive that cannot be loaded | box `Failed to load script.`, code 0 (1.1) | an error naming the archive, status 1, no modal message |
| `at_exit { }` | returns the Proc, never runs it (1.6) | runs it when the interpreter ends |
| `exit!(4)` | code 4, no box (1.6) | Ruby's `_exit(4)`: the guest process vanishes without its shutdown message, the program reports a failure |
| `load_data` source | a `File` for a loose file, a `String` for an entry (2.1) | always a `String` |
| a frozen ASCII-8BIT string from `_load` | labelled UTF-8, stays frozen (2.1, 2 of 71 samples) | the proc of the prelude raises `RuntimeError` |
| a cut loose file | `EOFError: end of file reached` (2.1) | `ArgumentError: marshal data too short` |
| a directory | `Errno::EACCES: Permission denied - d` (2.1) | `Errno::ENOENT` |
| 0 or 2 arguments | `wrong number of arguments(0 for 1)` (2.1) | Ruby's wording for a block method, with a space |
| name with NUL | `ArgumentError: string contains null byte` (0.2, 3.0.1) | the name is looked up with its NUL |
| ASCII-8BIT name that is not UTF-8 | `Encoding::UndefinedConversionError: "\xE6" from ASCII-8BIT to UTF-8` (0.2) | looked up as bytes |
| archive entry by the name without extension | found (2.2) | not found |
| `./`, `//`, `/./`, `..` in a name | not looked for in the archive (2.2) | normalised and found in the archive |
| trailing dot or space of a loose name | dropped, as Windows does (2.1) | not found |
| `Dir.chdir` before `load_data` | the current directory counts (2.1) | the game directory always |
| absolute name of a file of the game | found (2.1) | not found |
| `require_relative`, `autoload` | exist (V0 8) | Ruby's own: they call the interpreter's `require` and pass by the rule that only `.rb` files inside the game directory are loaded |

Three facts found while reading:

1. **Why the original's `$0` is `false` and its frame reads `ruby`.** In Ruby 1.9.2, `$0` is
   `GET_VM()->progname`, which `ruby_script` sets; the VM structure starts zeroed, and 0 is
   `false`. `vm_backtrace_each` of `vm.c` names a C frame by `progname` and, when that is
   unset, by `ruby_engine_name`, which is `"ruby"`. So an interpreter on which
   `ruby_script` is never called has both measured values. `vkxp-ruby192-sys` calls
   `ruby_script(options.script_name)` today.
2. **`require_relative` and `autoload` call `rb_require_safe` directly** (`load.c` line
   452, `variable.c` line 1552 of the vendored source), not `Kernel#require`, so the
   definition of `require` in `prelude_shared.rb` does not see them. The interpreter loads
   no compiled extension (`dmydln`), so what passes is a Ruby file the confined process
   may read, or with `--sandbox off` any Ruby file.
3. **The editor's data names files exactly.** In the three games of the corpus every
   `load_data` names a file with its extension (`Data/Actors.rvdata2`,
   `sprintf("Data/Map%03d.rvdata2", id)`, `Volume.rvdata2`, `PublicData.rvdata2`); the
   one name of another extension (`Data/MapInfos.rvdata` in Capella's Promise) is on a
   branch for RGSS2 that a game of RGSS3 does not take once `Graphics.play_movie` is
   defined.

## Goals / Non-Goals

**Goals:**

- The rules above, measured on the original, in a game of RGSS3, each held by a test with
  the measured value; the two rules vkxp does not follow, decided and recorded.
- `require_relative` and `autoload` under the rule of `require`.
- XP and VX unchanged: every existing test passes over both interpreters with its expected
  values unedited; no entry of `ruby192.allow` changes.

**Non-Goals:**

- The reset key: `match-the-reset-key-of-rgss3`.
- `Win32API`, `DL`: `emulate-win32api-and-dl-over-ruby-1-9-2`.
- `RGSS_VERSION` and the glyph rule by the DLL: `bind-font-and-text-of-rgss3`.
- Encodings made at first use, and with them `# coding: shift_jis` and `Windows-31J`:
  `read-strings-in-other-encodings`.
- The lookup rules of 2.2 for bitmaps, audio and movies: only `load_data` was measured
  (API report 8, point 5).
- The behaviour of 3.0.0 where it differs from 3.0.1 (0.2): vkxp follows 3.0.1 (the decision
  on the DLL version in `docs/decisions.md`).
- Any change of RGSS1, RGSS2, the protocol or the host crates.

## Decisions

1. **The report of a long error is shown whole (vkxp's own rule).** The original shows an
   empty box when the text is longer than 1023 UTF-16 units (1.2: 980 characters of message
   show, 981 do not; 500 three-byte characters show). That is the limit of a buffer of the
   runtime, it hides the error from the player, and no script can read the box. vkxp sends
   the whole text as the modal message and writes it to the error stream. Recorded as a
   known difference.

2. **An uncaught exception ends the run with status 1 (vkxp's own rule).** The original's
   code after the box is a table of 98 cases without an established rule (1.2; 8, point
   3): the line number when the name is a script of the archive, 0 for `Errno::ENOENT`,
   an empty backtrace, `SystemStackError`, a named `eval` and a required file, 1 for the
   six coding errors. vkxp's status says whether the game ended without a failure; 0 after
   an error would hide it from a caller, and a line number of 0 to 255 is no status a
   caller can read as such. The requirement "Uncaught errors of RGSS3" says status 1
   already; this change records the reason.

3. **A script archive that cannot be loaded shows `Failed to load script.`.** For the cases
   of 1.1 (no file, not a dump, a String instead of an Array, an entry whose third value is
   no deflated data or is missing) `play` of `runtime.rs` shows the modal message
   `Failed to load script.` (written to the error stream too) before the outcome and the
   shutdown message, in RGSS3 only. The line `vkxp: cannot load the script archive <path>:
   <reason>` that the glue writes stays, so the error stream still names the archive
   (scenario "Missing archive" of "Script archive of RGSS3" is unchanged). The status stays
   1, by the reason of decision 2, where the original ends with 0. An empty Array runs
   nothing and ends with status 0, as measured and as today. Not followed, recorded: a
   `Game.ini` without `Scripts=` or without the file `Game.ini` is an error of the program
   before the guest starts (the runtime is found from `Game.ini`); `Library=` naming a
   missing DLL is not looked at (vkxp loads no DLL; the original shows an empty box).

4. **`at_exit` never runs its block.** `prelude_rgss3.rb` defines `at_exit` as a private
   function of `Kernel` and a module function (`Kernel.at_exit`), arity 0 with a block, that
   returns the block as a Proc and registers nothing. Without a block it raises
   `ArgumentError` with Ruby's message `called without a block`. `END { }` is compiled by
   the parser into another call and is not covered (not measured; Follow-ups).

5. **`exit!` ends the game at once.** `prelude_rgss3.rb` defines `exit!(status = false)` as
   a private function of `Kernel` and a module function, and `Process.exit!` the same, over
   a host function `vkxp_exit_now(ok)` of `kernel.rs`: it stops the audio, finishes the
   frame statistics, sends the outcome (`ok` is true for `true` and for an Integer 0) and
   the shutdown message, and ends the process with `std::process::exit` and the status the
   script gave (`true` 0, `false` 1, an Integer as it is), so no `ensure` clause and no
   `at_exit` block runs. The program then exits with status 0 or 1 by the outcome, as for
   every other ending: vkxp's status says success or failure and does not carry `n` (the
   original's code is `n`, 1.6). A game of RGSS3 always runs in a guest process (decision
   7 of `run-rgss3-games-in-a-guest-of-their-own`), so ending the process ends the guest
   only; the definition is made for RGSS3 only, and the test harness of the binding, which
   runs games in its own process, does not call it (tests through both executables do).
   Rejected: leaving Ruby's `exit!`, whose `_exit` leaves the host without a shutdown
   message, so that the session ends as `Disconnected` and the program reports a failure
   for `exit!(0)`.

6. **No script name for RGSS3.** `vkxp_ruby192_sys::Options::for_rgss(3)` gives an empty
   `script_name`, and the crate does not call `ruby_script` when the name is empty: `$0` is
   `false` and the frame of a C function at the bottom reads `ruby` (fact 1). The default of
   `Options` stays `"vkxp"`, so every test of the crate keeps its values; `for_rgss(1)` and
   `for_rgss(2)` keep the default. The test harness of the binding starts its interpreter
   with `for_rgss` of its runtime already, so the tests of RGSS3 in `vkxp-binding-rgss3`
   see the new values. `report_rgss3` reads `:in `` in a frame of the `eval` already and
   needs no change; the report of an unknown coding becomes `ArgumentError occurred.` with
   the message (line 0), as measured.

7. **`Untitled`.** `guest_inputs` of `vkxp-guest-main` puts `Untitled` in the title of the
   configuration of a game of RGSS3 whose `Game.ini` has no `Title=` or an empty one. Both
   the window the program opens and the guest's title message take it from there. An empty
   value was not measured (only a missing key, 1.1); it is taken as missing. The
   configuration file itself is not changed, so `GetPrivateProfileStringA` reads it as it
   is.

8. **Magic comments are Ruby's.** Ruby 1.9.2 reads the magic comment of a string given to
   `eval` as of a file, and the script is evaluated as a UTF-8 string of its bytes
   (decision 10 of the guest change). The measured table of 1.1 is held by tests for:
   none or `# coding: utf-8` (UTF-8), `# coding: ascii-8bit`, `# encoding: ASCII-8BIT`,
   `# vim: set fileencoding=ascii-8bit :` (ASCII-8BIT, `"日本".size` 6), a coding comment on
   line 2 after a comment (ignored), `# coding: us-ascii` with a byte above 127 in a literal
   (`Script 'magic' line 2: SyntaxError occurred.` and `invalid multibyte char
   (US-ASCII)`), `euc-jp`, `binary`, `nosuch` (`ArgumentError occurred.`, `unknown encoding
   name: <name>`, status 1), `utf-16le` (`ArgumentError occurred.`, `UTF-16LE is not ASCII
   compatible`). If one of them fails, the cause is fixed in the binding with a check of
   the runtime. `shift_jis` and `Windows-31J` are not: `vkxp-ruby192-sys` registers
   Shift_JIS and Windows-31J at the first look-up of their names, so a script that names
   them is compiled in them; `read-strings-in-other-encodings` makes them dummy encodings
   created at first transcoding (API report 3.1), and the fatal report then follows.
   Known difference until then.

9. **The source of `load_data`** (2.1). `load_data(name)` of RGSS3 becomes a host function
   of `kernel.rs` with arity 1, so that a wrong count reads `wrong number of
   arguments(0 for 1)` as measured. It converts the name as the interpreter converts a
   String argument (`can't convert nil into String`, an object with `to_str` accepted),
   checks it (decision 10), asks the game's files (decision 11) and calls `Marshal.load`
   through Ruby with
   - an entry of the archive: a String of its bytes;
   - a loose file: a `File` opened `"rb"` on the path found, closed afterwards also when
     `Marshal.load` raises,
   and the relabelling proc. Through Ruby, so that a redefined `Marshal.load` is called, as
   today. With a `File` the errors of a cut file are Ruby's for an IO source: `EOFError` for
   an empty file, one byte and an Array cut short, `ArgumentError: marshal data too short`
   for a String cut short, as measured.
   - *The proc* is made once and calls a host function `vkxp_label_utf8(object)` that labels
     a String whose encoding is ASCII-8BIT as UTF-8 with the encoding call of the C API,
     which does not check whether the string is frozen, and returns the object. A frozen
     string so stays frozen and is labelled, as the original does (the two samples the
     Ruby model could not explain).
   - `save_data` stays Ruby (`Marshal.dump` written to the file opened `"wb"`, value `nil`),
     with the name check of decision 10 first.

10. **Names that are not ASCII** (0.2, 3.0.1). Before any look-up, for `load_data` and
    `save_data`: a name that holds a NUL byte raises `ArgumentError: string contains null
    byte`; a name labelled ASCII-8BIT whose bytes are not valid UTF-8 raises
    `Encoding::UndefinedConversionError` with the message `"\xHH" from ASCII-8BIT to UTF-8`,
    `HH` the first byte that is not valid, in upper case (samples: `"\x83" from ASCII-8BIT
    to UTF-8` for `"d/\x83\x65.bin"`, both functions). A UTF-16LE name holds NUL bytes and
    so raises the first error, as measured. A name that is valid UTF-8, whatever its label,
    is looked up as UTF-8. A name labelled UTF-8 whose bytes are not valid is accepted
    (`save_data("bad", "d/\xff\xfe.bin")` returned `nil`; the file appeared as `??.bin` in
    the ANSI listing); vkxp uses its bytes as they are.

11. **The lookup of a data file of RGSS3** (2.1, 2.2), a function of `vkxp-vfs`:
    `GameFiles::data_rgss3(name) -> Result<DataFile, VfsError>`, `DataFile` being an
    entry's bytes or a path. Interpreter-free, with unit tests in the crate.
    - *The archive first.* The name is split at `/` and `\`. When a part is empty, `.` or
      `..`, when the name starts with a separator or a drive (`C:`), or when it ends with
      `.` or a space, the archive is not asked. Otherwise an entry matches without regard to
      letter case when its name is the name as given, or when its name without the
      extension of its last part (from the last `.` of that part) is the name. So
      `Data/arconly` finds `Data\arconly.rvdata2`, `Other/x` finds `Other\x.bin`, and
      `Data/arconly.rvdata` or `Data/arconly.rvdata2.rvdata2` find nothing there. The
      current directory does not matter to the archive (inside `Dir.chdir("Data")`,
      `Data/arconly.rvdata2` was the entry and `arconly.rvdata2` was not found: arc
      samples). When two entries match by their stem, the first in the order of the
      archive is taken (not measured).
    - *Then the loose file*, as `GameFiles::resolve` finds a `FileKind::Data` file today
      (no extension added, `/` and `\`, `./`, `//`, `/./` and `..` resolved, letter case
      ignored), after dropping trailing dots and spaces of the last part of the name
      (`d/p.bin.` and `d/p.bin ` found, ` d/p.bin` not: data samples, "paths"). A relative
      name is taken from the current directory of the process, which the glue sets to the
      game directory and a script may change with `Dir.chdir` (`Dir.chdir("d") {
      load_data("p.bin") }` found the file, `load_data("d/p.bin")` there did not, and
      `load_data("../Game.ini")` there read `Game.ini`). An absolute name is taken as it
      is (`load_data(Dir.pwd + "/d/p.bin")` found). The `GameFiles` call gets the name
      joined to the current directory relative to the game directory; a name that then
      lies outside the game directory is not found, so the rule that the game's files
      never reach outside it stays. A match that is a directory is
      `VfsError::IsDirectory`, which the binding raises as `Errno::EACCES` with the
      message `Permission denied - <name as given>`. Not found: `VfsError::NotFound` with
      the name as given, raised as `Errno::ENOENT` as today.
    - *Outside the game directory*: the original reads any path the process may read
      (2.1). vkxp raises `Errno::ENOENT`. Known difference.
    - `FileKind::Data` and `GameFiles::read` keep their behaviour for RGSS1 and RGSS2 and
      for every other use.

12. **Loading code in RGSS3** (V0 8). `require` and `load` keep the definitions of
    `prelude_shared.rb`, whose measured behaviour tests now hold in a game of RGSS3: `require
    "./name.rb"` and `require "name"` load the file of the game directory, `require` returns
    `true` and then `false`, the full path is added to `$LOADED_FEATURES`, a missing file
    raises `LoadError: no such file to load -- name`, `require "zlib"` raises `LoadError`.
    `prelude_rgss3.rb`, evaluated before `prelude_shared.rb`, adds:
    - `require_relative(name)`, private in `Kernel`, arity 1: `require` of the name taken
      relative to the directory of the file that calls it: the game directory for a script
      (its file name `{NNNN}` has no directory), the file's own directory for a file loaded
      by `require` or `load`. The directory comes from the caller's backtrace entry. The
      original was measured to have the method only; the rule is Ruby 1.9.2's.
    - `Module#autoload(name, file)` and `Kernel#autoload`: the file is located by the rule
      of `require` (`vkxp_locate(file, true)`) at the call. Found: Ruby's own `autoload`
      is called with the absolute path of that file, which Ruby's `require` then loads and
      enters into `$LOADED_FEATURES` under the path `require` of the prelude uses, so a
      later `require` of the same file returns `false`. Not found or not allowed:
      `LoadError: no such file to load -- <file>` at the call. Known difference: the
      original raises at the first use of the constant.
    - A file loaded by `require` or `load` is compiled as UTF-8 (a text string of its
      bytes), as the scripts are. How the original compiles it was not measured.
    The rule for RGSS1 and RGSS2 is unchanged; Ruby 1.8.7 has no `require_relative`, and
    `autoload` of those runtimes is a follow-up.

13. **Tests.**
    - `crates/vkxp-binding-rgss3/tests/`: new tests in `run_game_rgss3.rs` (program name,
      magic comments), `report_rgss3.rs` (long report, failed load, magic comment reports),
      `data_rgss3.rs` (source, frozen string, errors, names, `save_data` names), and a new
      file `loading_rgss3.rs` (`require`, `load`, `require_relative`, `autoload`, `at_exit`
      value). These files are RGSS3's own; no existing test is edited.
    - `crates/vkxp-vfs`: unit tests of `data_rgss3` with an archive written by the test
      support of the crate.
    - `crates/vkxp-ruby192-sys`: a test that `for_rgss(3)` starts with `$0` false and a
      backtrace whose last entry is ``ruby:in `eval'``, and that the default keeps `vkxp`.
    - `crates/vkxp-guest-main/src/lib.rs`: a unit test of `guest_inputs` for `Untitled`.
    - `crates/vkxp/tests/rgss3.rs`: `exit!`, `at_exit` in four endings, the failed load
      through both executables (modal message on the error stream, status 1), and the
      archive lookup (an entry found without its extension, a loose file beside the archive
      entry of the same name).

## Implementation notes

- A flow reads `../../ref/` and `../../games/` and copies nothing from them. No Wine and no
  original runtime is run. Scratch output goes under `target/` or `local/`.
- Before anything changes, keep under `local/` the lists of `cargo test --locked -- --list`
  of `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-vfs`, `vkxp-ruby192-sys`,
  `vkxp-guest-main` and of `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp -p
  vkxp-guest-rgss3 --locked -- --list`. At the end every name is there again.
- Every command that reaches Vulkan has `VK_LOADER_DRIVERS_SELECT='*lvp*'`.
- `load_data` of RGSS3 replaces the Ruby definition of `prelude_rgss3.rb` and the host
  function `vkxp_data_bytes`, whose only user it was; `load_data` of `kernel.rs` for
  RGSS1 and RGSS2 is untouched. `vkxp_load_scripts` keeps calling `load_data`.
- The file a loose `load_data` opens is the path `GameFiles` found, handed to Ruby as
  bytes; the sandbox allows reading the game directory.
- A scenario that asks for a line of the diagnostics is met by the property, not by a
  fixed wording.
- `exit!` and the failed load are tested through both executables in
  `crates/vkxp/tests/rgss3.rs`, which needs `cargo test -p vkxp -p vkxp-guest-rgss3`.

## Which milestone needs which item

The corpus is Crysalis, In Search of Immortality and Capella's Promise (`docs/decisions.md`,
"VX Ace"). Their scripts were read (decrypted into `target/`, nothing copied): none calls
`require`, `require_relative`, `autoload`, `load`, `at_exit`, `exit!`, `$0` or
`$PROGRAM_NAME`; each `Game.ini` has a title; the only magic comment is `# coding: utf-8`;
every `load_data` names an existing file exactly (fact 3), and saves are written and read
with `Marshal` on a `File` by `DataManager`, not with `load_data`.

- **A2 (walking on a map)** depends on none of these items.
- **A3 (battle, saving and loading)** depends on none of them either. The battle-test
  database (`Data/BT_*.rvdata2`) and the saves use exact names.
- Every item is fidelity for other games and for scripts that probe the runtime. The one
  that changes the safety of a run is decision 12 (`require_relative`, `autoload`).

## Implementation record

What the implementation had to do beyond the decisions above, with the reason:

- `crates/vkxp-ruby192-sys/tests/calling.rs`, `options_for_a_runtime_are_the_defaults`,
  asserted that `Options::for_rgss(3)` has the default script name, the rule this change
  reverses (decision 6). Its assertion for generation 3 now expects the empty name; those
  for 1 and 2 are unchanged. No other expected value of an existing test was edited.
- `GameFiles::data_rgss3` takes the current directory of the process as a second argument,
  so that the crate stays free of global state and its tests can name one. A current
  directory outside the game directory counts as the game directory: the harness of the
  binding plays its games in the process of the test, whose current directory is not the
  game's, and the guest sets it to the game directory.
- `vkxp_label_utf8` labels through `String#force_encoding` with the frozen flag taken off
  for the call and put back: the binding compiles over both interpreter crates, and no
  encoding function of the C API is declared by both (decision 9 named the C API).
- The name check of `save_data` is the host function `vkxp_check_data_name`, which
  `load_data` of RGSS3 shares.
- In a game of RGSS3 `prelude_shared.rb` is evaluated before `prelude_rgss3.rb`, whose
  `require` and `load`, those of the shared file with the source labelled UTF-8, take their
  place (decision 12 said the shared definitions stay and the order is the other way):
  `prelude_shared.rb` may not be edited by this change, and its `require` reads the file as
  bytes, which the interpreter compiles as ASCII-8BIT.
- `require_relative` turns the directory of the calling file into a name below the game
  directory with the host function `vkxp_relative`: `vkxp_locate`, and `GameFiles::resolve`
  under it, take names relative to the game directory only.
- `Kernel#autoload` registers in `Object`, the namespace of a call at the top level: Ruby's
  own registers in the lexical namespace of its caller, which for a definition in the
  prelude would be `Kernel`. Inside a class body `Module#autoload` is called and registers
  in that class.
- The scenario "require" writes `(require("nosuch") rescue $!.message)`; a `rescue`
  modifier does not catch a `LoadError`, so its test rescues `Exception` explicitly.

## Follow-ups

Left out, with the measured source (API report unless named):

- The reset key: F12 held, the asynchronous raise, waits and fades cut short, `__reset__`
  waiting for the release, F2: 1.4. `match-the-reset-key-of-rgss3`.
- `# coding: shift_jis`, `Windows-31J`, and after the first transcoding `Shift_JIS is not
  ASCII compatible`: 1.1, 3.1, 3.3. `read-strings-in-other-encodings`.
- A BOM followed by a here-document failed with a `SyntaxError` on the original (1.1, one
  sample, observed).
- The exit code of the original after a box (1.2; 8 point 3): no rule; kept at 1
  (decision 2).
- The empty box above 1023 units (1.2): not followed (decision 1).
- `Failed to load script.` with code 0, `Game.ini` without `Scripts=`, no `Game.ini`, and the
  empty box of `Library=` naming a missing DLL (1.1; 8 point 8): decision 3.
- `END { }` blocks: not measured; they run when the interpreter ends.
- `WM_CLOSE` and Alt+F4 end the original inside the next `Graphics.update` without running
  `ensure` clauses (1.6); vkxp raises `SystemExit`, whose `ensure` clauses run. Not in this
  change's list; a known difference to record when the reset key is done.
- The name of the file `save_data` writes for a name labelled UTF-8 whose bytes are not
  valid: `??.bin` in the ANSI listing of the original (data samples), its bytes in vkxp.
- Two archive entries with the same stem (2.2: not measured).
- The lookup rules of 2.2 for bitmaps, audio and movies (8, point 5).
- `Dir.entries` names in the ANSI code page (2.1): Windows' and Wine's, not followed.
- A path outside the game directory for `load_data` (2.1): refused (decision 11).
- `load_data("Game.rgss3a")` reads the archive file itself (2.2): not tested.
- 3.0.0: names with NUL cut at the NUL, every name that is not ASCII raising (0.2); vkxp
  follows 3.0.1.
- How the original compiles a file loaded by `require` or `load`, the name a report gives
  such a file, the path form in `$LOADED_FEATURES`, `load(name, true)`, `require
  "enumerator"` against the loaded feature `enumerator.so`: not measured (V0 8 measured
  that `require` and `load` load files of the game and the `LoadError`).
- `autoload` in RGSS1 and RGSS2 calls the interpreter's own `require` over both
  interpreters, past the rule of the prelude (fact 2 holds for Ruby 1.8.7 too); a change
  for those runtimes, with a test over both interpreters.
- The name of the box's caption (the game title) has no counterpart: vkxp's modal message
  has none.
- A file that `autoload` loads is compiled by the interpreter's own loader, which reads it
  as US-ASCII, the default of Ruby 1.9.2, and not as UTF-8 as `require` of the prelude does.
  Loading it through the prelude would need a hook of the constant lookup that this
  design does not name.
- The opening comment of `prelude_shared.rb` still says that it is evaluated after
  `prelude_rgss3.rb`; the file is outside this change.
- `load_data` after `Dir.chdir` to a directory outside the game directory looks from the
  game directory (see the implementation record); the original looks outside.
