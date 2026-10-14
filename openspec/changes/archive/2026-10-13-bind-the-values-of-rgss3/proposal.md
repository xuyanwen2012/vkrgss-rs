## Why

The standard scripts of RPG Maker VX Ace call, before their first frame, `Tone.new` and
`Color.new` without arguments, `Rect.new` without arguments, `Tone#set(tone)`,
`Rect#set(rect)` and `Color#set(color)` (fact 1 of the design of
`run-rgss3-games-in-a-guest-of-their-own`). Each raises `ArgumentError` today: in a game of
RGSS3 the classes `Color`, `Tone`, `Rect` and `Table` are those of RGSS2. What RGSS3 has
differently was measured on the original runtime on 2026-10-13 (`ace-v1-api-REPORT.md`,
sections 0, 0.2, 2.4 and 3.3, and `ace-v1-bitmap-REPORT.md`, sections 3 and 12, with their
sample files, outside the repository). This is the first of the changes that bind the class
surface of RGSS3, and the others build on it.

## What Changes

- In a game of RGSS3, and only there:
  - `Color.new` and `Tone.new` without arguments give (0, 0, 0, 0); `set` takes no
    argument, three or four numbers, or one `Color` or `Tone` whose four numbers it copies.
  - `Rect.new` takes no argument or four Integers; `Rect#set` four Integers or one `Rect`.
    `initialize` and `set` of `Rect` have the arity -1 of the method list of RGSS3.
  - A wrong count of arguments of these methods and of `Table#[]` and `Table#[]=` raises
    `ArgumentError` with the message of the original, `wrong number of arguments (E for
    G)`, the expected count first. Arguments are converted by Ruby 1.9, whose messages are
    the measured ones.
  - `Color._load` of a String that is not 32 bytes long gives (0, 0, 0, 0); `Rect._load` of
    a short String reads zeros for what is missing.
- Held by tests for RGSS3 with the measured values, with no change of code expected: the
  limits of the components, `==`, `===` and `eql?` with `nil` or another class (false, as
  version 3.0.1 of the original; the `TypeError` of 3.0.0 is not followed, by the decision
  on DLL versions of `docs/decisions.md`), `_dump` and its bytes, `to_s` and `inspect`,
  `Table`.
- Where the original ends or reads memory that is not its own (`Table._load` of a short
  String, `Rect._load` of a short String, `set` with an object of another native class),
  vkxp raises or reads zeros; recorded as known differences.
- The method list of RGSS3 is committed whole, as those of RGSS1 and RGSS2 are, with a test
  that checks the classes bound so far: these four.
- `Font` is not touched: all of it comes with `bind-font-and-text-of-rgss3`.
- XP and VX are unchanged: every existing test passes over both interpreters with its
  expected values unedited.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-values`: the forms, messages, serialized forms and method surface of `Color`,
  `Tone`, `Rect` and `Table` in a game of RGSS3.
- `supported-runtimes`: the known differences of these classes in `docs/compatibility.md`.

## Impact

- `crates/vkxp-binding/src`: `color.rs`, `rect.rs`, `table.rs` (the functions of RGSS3 and
  the runtime as an argument of their `define`), `rb.rs` (one function for the count
  message of RGSS3), `guest.rs` (the runtime handed to those `define` functions), `lib.rs`
  (documentation only).
- `crates/vkxp-binding/tests/data/rgss3_api.txt` (new: names and arities only).
- `crates/vkxp-binding-rgss3/tests/`: `values_rgss3.rs` and `arity_rgss3.rs` (new).
- `crates/vkxp-binding/tests/ruby.rs` and `scene_rgss2.rs`: one new test each, which holds
  that RGSS1 and RGSS2 do not take the new forms.
- `docs/compatibility.md`: a subsection of "RPG Maker VX Ace".
- Not touched: `crates/vkxp-core` and every other crate; `font.rs`, `bitmap.rs`,
  `window.rs`, `element.rs`, `wrap.rs`, `runtime.rs` and the prelude files of the binding;
  every existing test of both binding packages; `ruby192.allow`; the protocol;
  `third_party/`; `docs/decisions.md`, whose decision on DLL versions is followed as it is.
- Nothing of the original is committed: the tests hold rules and numbers of the
  measurement, and the method list holds names and arities.
