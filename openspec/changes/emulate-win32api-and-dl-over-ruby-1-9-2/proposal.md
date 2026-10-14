## Why

A game of RPG Maker VX Ace sees the `Win32API` of the prelude of XP and VX today: a Ruby
class that ignores the import and export declarations, converts every argument by itself,
returns an Integer for every call and raises for nothing. The original runtime of RGSS3 was
measured on 2026-10-13 (`ace-v1-api-REPORT.md`, "the API report", section 5.8, outside the
repository): it reads the declarations, counts and converts the parameters with errors of
their own, returns a String for the export `p` and 0 for `v`, raises for a library or a
function it cannot find, keeps the label of a buffer it fills, and has `DL` beside it
(5.9). Row 10 of the table "the changes that follow" of the design of
`run-rgss3-games-in-a-guest-of-their-own` names this change. Two of the three games of the
corpus call `Win32API` from their first frame on (In Search of Immortality: `user32` and
`wininet`; Capella's Promise: `user32` and its own `tktk_bitmap.dll`), so the class must
keep them running while it becomes the original's.

## What Changes

In a game of RGSS3, and only there:

- `Win32API` is a class of the binding written in Rust with the method list of RGSS3
  (`initialize/4`, `call/-1`, `Call/-1`): the constructor takes four arguments, the library
  and the function as Strings, the declarations as a String, an Array of Strings or `nil`;
  the object keeps its state where scripts do not see it, can be duplicated and subclassed,
  and cannot be dumped.
- **Libraries.** A library is found when it is a file of the game, the game's own runtime
  library, or one of a fixed list of Windows system libraries; any other name raises
  `RuntimeError` `LoadLibrary: <name>` with a line feed, as the original does for a missing
  DLL. A function of a found library is emulated or reported once, as for XP and VX.
- **Declarations.** Each character `p`, `l`, `i`, `n` of the import (either case) is a
  parameter, `v` none, any other character is ignored; a wrong count raises `RuntimeError`
  `wrong number of parameters: expected N, got M`.
- **Conversions and values.** Integer parameters convert as the original's (Float
  truncated, `nil` and Strings refused with the measured messages) and are taken by their
  low 32 bits; pointer parameters take a String, which must not be frozen, `nil`, 0 or an
  address. The export `v` gives 0, `p` raises `ArgumentError: NULL pointer given` for 0,
  every other export a signed 32-bit Integer. A buffer keeps its encoding label.
- **Measured calls.** `GetPrivateProfileStringA` with a bare file name reads nothing, as
  Windows does; the window class `RGSS Player`, the client rectangle of 544 by 416 and
  `ShowCursor` are held by tests; the address trick of bitmap scripts reads nothing.
- **`DL`** exists with the constants of the original and refuses every way into a library
  or into memory with `DL::DLError`; no game of the corpus uses it.
- Three entries of `ruby192.allow` go: the tests of the keyboard state that used `String#[]`
  for bytes are rewritten with `unpack` and `pack`, their expected values unedited.
- XP and VX are unchanged: every existing test passes over both interpreters with its
  expected values unedited.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: `Win32API` and `DL` of RGSS3.
- `supported-runtimes`: what `docs/compatibility.md` says of them.

## Impact

- `crates/vkxp-binding/src`: `win32api.rs` (the class of RGSS3, the libraries, the
  declarations, the conversions, the values, the bare profile name), `runtime.rs` (which
  prelude files are evaluated), `prelude_shared.rb` (its `Win32API` moves out),
  `prelude_win32api.rb` (new: that class, for RGSS1 and RGSS2), `prelude_dl.rb` (new: `DL`
  of RGSS3), `lib.rs` (documentation).
- Tests: `crates/vkxp-binding-rgss3/tests/win32api_rgss3.rs` (new), the line of bound
  classes of `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs`, three tests of
  `crates/vkxp-binding/tests/win32api.rs` (their Ruby snippets, not their expected values),
  `crates/vkxp-binding-rgss3/tests/ruby192.allow` (three entries go),
  `crates/vkxp/tests/rgss3.rs` (a game that calls `Win32API` at load, through both
  executables).
- `docs/compatibility.md` (the section "RPG Maker VX Ace"), `CHANGELOG.md`.
- In flight on the same files: `bind-the-scene-classes-of-rgss3` edits the line of bound
  classes of `arity_rgss3.rs`; `match-the-endings-and-loading-of-rgss3` and
  `bind-font-and-text-of-rgss3` edit `runtime.rs` and `prelude_rgss3.rb` elsewhere (this
  change does not edit `prelude_rgss3.rb`).
- Not touched: the protocol, the host crates, `crates/vkxp-core`, the interpreter crates,
  `third_party/`, the emulation of RGSS1 and RGSS2.
