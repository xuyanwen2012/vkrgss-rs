# Split the large modules of the binding

## Why

Three files of `vkrgss-binding` hold several concerns each:

- `runtime.rs` (1175 lines): the link to the host and its pump, the evaluation of the
  preludes, the start-up values of each runtime (frame size and rate, `RGSS_VERSION`, test
  mode globals), the loading of the built-in scripts and the script archive, the script
  loop with reset, and the two formats of the report of an uncaught error (RGSS1/2 and
  RGSS3, `report`, `report_rgss3` and their position parsers, `:705-904`).
- `win32api.rs` (1323 lines, about 100 of them tests): the table of emulated functions, the
  private-profile (INI) functions, the code-page conversions, the keyboard, mouse and window
  functions, the host function of RGSS1/2, and the `Win32API` class of RGSS3 with its import
  and export parsing (`:923-1222`).
- `font.rs` (911 lines): the `Font` class of three runtimes with the RGSS3 variants (name and
  size objects, outline, out colour, the default objects) interleaved with the RGSS1/2
  accessors.

Beside that, `prelude_shared.rb` and `prelude_rgss3.rb` start with 21 and 44 comment-only
lines whose stated purpose is to keep the line numbers of what follows, after code moved out
of them (`prelude_shared.rb:5-7`); `runtime.rs` also evaluates `prelude_load_rgss3.rb` from
line 48 (`PRELUDE_LOAD_RGSS3_LINE`) for the same reason. Nothing records who reads those
line numbers.

## What Changes

- The three files become directory modules split by concern; code moves, no body is
  rewritten, and the public API of the crate does not change.
- Why prelude line numbers are kept is found out and recorded: if a test, a report format or
  an oracle comparison depends on them, the padding stays with that reason written next to
  it, or is replaced by evaluating from a starting line as `PRELUDE_LOAD_RGSS3_LINE` does;
  if nothing depends on them, the padding goes.
- No behaviour of a game changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `binding-over-two-interpreters`: the layout of the binding's source is stated.

## Impact

`crates/vkrgss-binding/src` only (the three modules, the two preludes, `runtime.rs`'s
evaluation of them, tests that name module paths). `vkrgss-binding-rgss3` compiles the same
source and follows without edits beyond paths. It runs after
`confine-loading-and-process-functions`, `call-rgss-from-script-threads` and
`share-the-interpreter-glue`, which all edit the binding. No protocol, no picture.
