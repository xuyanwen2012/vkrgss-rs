# Design

## Context

A game of RGSS3 sees the classes of RGSS2 today (`run-rgss3-games-in-a-guest-of-their-own`,
table "What is there today"). The table "the changes that follow" of that design names this
change first: the forms of `Color`, `Tone` and `Rect` that RGSS3 adds, `==` with another
class, `_load` of a short string, and the wording of the argument errors, over
`vkxp_core::{Color, Tone, Rect, Table}` with nothing new in the core.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{color,rect,table,rb,wrap,guest,font}.rs`; how `font::define`
  and `bitmap::define` take the runtime; `testing.rs` (`start_as`) and
  `crates/vkxp-binding-rgss3/tests/{strings,rgss3_script}.rs`, the model of a test file that
  exists in the second package only.
- The design of `build-the-binding-over-ruby-1-9-2` (text and bytes, the allow-list, how
  tests run over both interpreters) and `docs/decisions.md`, "VX Ace" (the decision on DLL
  versions, the word size).
- The measurements, outside the repository: `../../ref/tools/oracle/measurements/
  ace-v1-api-REPORT.md` ("the API report": sections 0, 0.2, 2.4, 3.3; samples
  `ace-v1-api-vxsections-samples.301.txt` and `.300.txt`, comparison with RGSS2
  `ace-v1-api-vxsections-cmp.txt`), `ace-v1-bitmap-REPORT.md` ("the bitmap report":
  sections 3 and 12; samples `ace-v1-bitmap-samples.txt`, lines 170 to 269), and
  `../../ref/RGSS301_api.md` ("the method list").

What the binding does today, in every runtime, and what was measured for RGSS3:

| | Today | RGSS3, measured |
|---|---|---|
| `Color.new`, `Tone.new` | 3 or 4 numbers | also none: (0, 0, 0, 0) |
| `Color#set`, `Tone#set` | 3 or 4 numbers | also none (zeros) and one `Color` or `Tone` |
| `Rect.new` | 4 Integers, arity 4 | none or 4, arity -1 |
| `Rect#set` | 4 Integers, arity 4 | 4 or one `Rect`, arity -1; none raises |
| count message | `wrong number of arguments (G for E)`, Ruby's order | `wrong number of arguments (E for G)` |
| `==` with another class | false | false on 3.0.1, `TypeError` on 3.0.0 |
| `Color._load`, wrong length | `ArgumentError` | (0, 0, 0, 0) |
| `Rect._load`, short | `ArgumentError` | reads memory behind the string |
| `Table._load`, short | `ArgumentError` | the process ends |
| conversions | Ruby's own | Ruby's own (1.9.2) |

Three facts:

1. **Most of the measured behaviour needs no code.** The messages `can't convert String
   into Integer`, `no implicit conversion from nil to integer`, `no implicit conversion to
   float from string` are those of `rb_num2long` and `rb_num2dbl` of Ruby 1.9.2, which
   `rb::int` and `rb::float` call. `inspect` of a data object of Ruby 1.9.2 is its `to_s`.
   `_dump` and `to_s` return bytes since `build-the-binding-over-ruby-1-9-2`. `==` with
   another class is false already. This change holds them by tests of RGSS3.
2. **The count message of the original is reversed against Ruby's.** `Color.new(1)` says
   `(3 for 1)`: expected first. The binding words Ruby's order for RGSS1 and RGSS2, where
   the original says `wrong # of arguments (3 for 1)`; that is not this change's.
3. **A test of RGSS3 cannot share a process with one of another runtime**, because the
   classes are defined once per process, and it cannot run over Ruby 1.8.7. The mechanism
   that exists: a test file of `crates/vkxp-binding-rgss3/tests/` alone, with the `cfg`
   attribute, which starts the harness with `testing::start_as(Runtime::Rgss3)`.

## Goals / Non-Goals

**Goals:**

- The value classes of a game of RGSS3 take every form the standard scripts use and answer
  as measured, error messages included.
- The method list of RGSS3 is in the repository, and the four classes match it exactly.
- Where the original faults or reads foreign memory, vkxp does something defined, and the
  documents say what.
- RGSS1 and RGSS2 do not change.

**Non-Goals:**

- `Font` in any part: `outline`, `out_color`, the defaults, `size` limits, `name`, the
  message of `color=`. `bind-font-and-text-of-rgss3`.
- The count messages and `_load` of RGSS1 and RGSS2 (the original of RGSS2 says `wrong # of
  arguments (E for G)`, and reads zeros for a short string; vkxp does neither).
- `==` as version 3.0.0 has it; a switch by the DLL of the game.
- What only a 32-bit build shows: the message of a `RangeError`, integers from 2^30.
- Raw copies between native classes, unclamped components from `_load`, the fault of
  `to_s`.
- Names the original interns at first use (`observer`): `intern-the-names-of-rgss3`.
- Any change of `vkxp-core`.

## Decisions

1. **Selected by the runtime of the game.** `color::define_color`, `color::define_tone`,
   `rect::define` and `table::define` take the `Runtime`, as `font::define` does, and
   `Guest::install` hands it over. For `Runtime::Rgss3` they register the functions of
   RGSS3 for the methods of the table below; every other method, and every method of
   RGSS1 and RGSS2, is registered as today, with today's function and arity. `rb::RUBY_19`
   is not asked: the runtime decides, not the interpreter.

   | Class | Registered for RGSS3 only |
   |---|---|
   | `Color`, `Tone` | `initialize` (-1), `set` (-1), `_load` |
   | `Rect` | `initialize` (-1), `set` (-1), `_load` |
   | `Table` | `[]`, `[]=` (the count message) |

2. **`Color` and `Tone`** (bitmap report 3).

   | Call | Result |
   |---|---|
   | `new` | (0, 0, 0, 0) |
   | `new(r, g, b)`, `new(r, g, b, fourth)` | as today: alpha 255, gray 0 |
   | `new` with 1, 2 or 5 arguments | `ArgumentError`: `(3 for 1)`, `(3 for 2)`, `(4 for 5)`; one argument is never an object to copy |
   | `set` | sets (0, 0, 0, 0) |
   | `set(r, g, b)`, `set(r, g, b, fourth)` | as `new` |
   | `set(other)`, `other` a `Color` or a `Tone` | copies the four numbers in order (`From<Tone> for Color` and `From<Color> for Tone` of the core) |
   | `set(other)`, anything else | `TypeError`, decision 4 |
   | `set` with 2 or 5 arguments | `ArgumentError`: `(3 for 2)`, `(4 for 5)` |

   `set` returns the receiver in every form. Limits, fractions and the writers are those of
   the core, unchanged.

3. **`Rect`** (bitmap report 3).

   | Call | Result |
   |---|---|
   | `new` | (0, 0, 0, 0) |
   | `new(x, y, w, h)`, `set(x, y, w, h)` | as today |
   | `new` with 1, 2, 3 or 5 arguments | `ArgumentError` `(4 for n)`, also when the one argument is a `Rect` |
   | `set(other)`, `other` a `Rect` | copies the four values |
   | `set(other)`, anything else | `TypeError`, decision 4 |
   | `set` with 0, 2, 3 or 5 arguments | `ArgumentError` `(4 for n)` |

   `set` and `empty` return the receiver.

4. **One argument that is no object to copy.** The original takes any native object and
   copies its bytes: `rect.set(color)` gave (0, 1072693248, 0, -1073741824), and the bitmap
   report says not to imitate it beyond `Color` and `Tone`. vkxp accepts the classes of
   decisions 2 and 3 and raises `TypeError` for everything else:
   - for a value that is no data object, the measured message `wrong argument type Fixnum
     (expected Data)`, with the name `rb_check_type` of Ruby 1.9.2 gives: `nil`, `true` and
     `false` for those three, and the name of the class otherwise;
   - for a data object of another class (the raw copy of the original), `wrong argument
     type <class> (expected <class of the receiver>)`, which `wrap::expect` words today.
     Known difference.

5. **The count message.** `rb.rs` gets `check_count_rgss3(ruby, args, min, max)` beside
   `check_count`: the `ArgumentError` `wrong number of arguments ({expected} for {given})`,
   `expected` being `min` when fewer were given and `max` when more. The classes call it
   with (3, 4) for `Color` and `Tone` and (4, 4) for `Rect`, after the counts they accept
   have been taken out. `Table#[]` calls it with the number of dimensions for both bounds;
   `Table#[]=` with fewer arguments than dimensions calls it with the number of dimensions
   (`t[0] = 5` on a table of three dimensions: `(3 for 2)`). `check_count` and
   `count_error` stay as they are for RGSS1, RGSS2 and every other class. A method of
   fixed arity is counted by the interpreter, whose message over Ruby 1.9.2 is the measured
   one (`wrong number of arguments(1 for 0)`, no space, given first).

6. **Conversions and limits are the interpreter's and the core's.** Nothing is coded.
   Tests hold the measured messages of `TypeError` and the class `RangeError`. Not
   followed, because the guest is a 64-bit build (`docs/decisions.md`, "Word size"): the
   message `bignum too big to convert into `long'` (vkxp says `integer 2147483648 too big to
   convert to `int'`), and the integers from 2^30 to 2^31 - 1, which the original wraps to
   31 bits (`Rect.new(2**30, ..)` is -1073741824, `Rect.new(2**31 - 1, ..)` is -1) and vkxp
   keeps.

7. **Comparison follows 3.0.1.** `docs/decisions.md` decides it: vkxp follows 3.0.1.1, and
   `Color#==` of 3.0.0.1 is not followed on any DLL. The samples of the API report show the
   same difference for `Tone` and `Rect` (`$t == nil` and `$r == nil` raise on 3.0.0), and
   the decision is applied to all three: false, never an error, whatever DLL the game
   ships. `equal` of `color.rs` and `rect.rs` does this today and is not changed. `hash` is
   that of `Object`, as on the original, where two equal colours have different hashes.

8. **Serialized forms** (API report 0 and 2.4).
   - `_dump`: unchanged. One argument, ignored; bytes.
   - `Color._load` in RGSS3: a string of 32 bytes as today (clamped, see below); a string
     of any other length gives (0, 0, 0, 0), which is what the report states and what its
     two samples, both of three bytes, show. `Tone._load` takes the same function, as it
     does today; no sample of a short tone exists and no scenario holds it.
   - `Rect._load` in RGSS3: a string shorter than 16 bytes is read as if zeros followed
     it. `"ab"` gives (25185, 0, 0, 0), which is what RGSS2 gave and what RGSS3 gave for
     the first three values; its fourth, 8199, was memory behind the string. A longer
     string stays the `ArgumentError` of today (not measured).
   - `Table._load`: unchanged. Malformed data raises `ArgumentError` where the original
     ends with a page fault.
   - Not followed: `Color._load` of 32 bytes does not clamp on the original
     (`Color._load([1000.0, ..])` reads 1000.0); the core clamps, and a dump the original
     wrote holds no such value. `to_s` of a component like 1e300 or NaN ends the original;
     vkxp cannot hold such a component.

9. **Text forms.** `to_s` unchanged. `inspect` is not defined by the binding (the method
   list has none); over Ruby 1.9.2 `Object#inspect` of a data object calls `to_s`, so
   `Color#inspect` is the text of `to_s` and `Table#inspect` is `#<Table:0x...>`, both as
   measured.

10. **`Table`.** Its behaviour in RGSS3 is that of RGSS2 line for line but for the wording
    of the messages (API report 0, the comparison file). The only code is decision 5.
    `Table.new` and `resize` with no size or more than three keep `check_count`: the
    original was not asked.

11. **`Font` is not here.** What of `Font` could count as plain value behaviour (the
    Float kept in `size`, the truth of `bold=`, `color=` taking a `Color` only and copying
    it, the `default_` writers keeping the object, `dup`) is one table of the bitmap
    report with `outline`, `out_color` and the defaults of RGSS3, and its tests need
    `FontDefaults::rgss3()`. Splitting it would put half of `font.rs` into each of two
    changes. All of it stays with `bind-font-and-text-of-rgss3`.

12. **The method list of RGSS3 is committed whole.**
    `crates/vkxp-binding/tests/data/rgss3_api.txt`, in the form of `rgss1_api.txt`, holds
    every entry of the method list for the sixteen classes and modules that have methods:
    `Graphics`, `Audio`, `Input`, `Sprite`, `Plane`, `Viewport`, `Tilemap`,
    `Tilemap::BitmapArray`, `Window`, `Bitmap`, `Font`, `Color`, `Tone`, `Rect`, `Table`,
    `Win32API`. A property stands with its reader (arity 0) and its writer (arity 1).
    `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs` checks the classes of one list in the
    file, `BOUND`, which this change sets to the four value classes; each later change
    adds its classes to that list and edits nothing else of the two files. The file is
    names and arities read from the original by reflection, as the two lists that are
    committed.

13. **Tests.** `crates/vkxp-binding-rgss3/tests/values_rgss3.rs` holds the scenarios of
    RGSS3, one evaluation per measured line or group of lines, through `testing::eval`
    after `testing::start_as(Runtime::Rgss3)`; the scenario "Seen by the element" loads
    `tests/scene_support` of `vkxp-binding` through `#[path]`, as `rgss3_script.rs` does.
    The existing tests of both packages are not edited: they run as games of RGSS1 and
    RGSS2 and hold that those do not change. The scenarios "XP and VX" are one new test in
    `crates/vkxp-binding/tests/ruby.rs`, whose harness is a game of RGSS1, and one in
    `tests/scene_rgss2.rs`, a binary of RGSS2 that exists; both run over both
    interpreters, and no new shared binary is made for them. No entry of `ruby192.allow` is expected to
    start passing; if one does, the mechanism reports it, and the entry and its marker are
    removed.

14. **Documents.** `docs/compatibility.md` gets "The value classes of VX Ace" as a
    subsection of "RPG Maker VX Ace". That section is created by
    `run-rgss3-games-in-a-guest-of-their-own`; when it is not there yet, this change
    creates the heading with one sentence (VX Ace is not supported yet; what follows is
    what its classes do) and the subsection under it. The opening of the document and the
    state of VX Ace in it are not touched. `CHANGELOG.md` is not touched: no game of VX
    Ace shows anything new to a user by this change.

15. **Delta specifications add and do not modify.** The requirements "Rect" and "Marshal
    forms" of `rgss-values` state RGSS1 and RGSS2; the added requirements say so, in the
    words "In RGSS1 and RGSS2 ... SHALL stay as it is", which is the form the changes of
    VX used.

## Implementation notes

- A flow reads `../../ref/` and copies nothing from it but names, arities, rules and
  numbers. No Wine and no original runtime is run.
- Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding` and of
  `vkxp-binding-rgss3` under `local/`: every name must be there at the end.
- Every form of `set` ends in one call of `Owned::set`, so that the owner of the object
  (a sprite's `src_rect`, a window's `cursor_rect`) sees exactly one change per call, also
  when the values are the ones it has. `bind-the-window-of-rgss3` relies on it.
- The functions of RGSS1 and RGSS2 call `wrap::intern_owner_names` where they do today.
  Those of RGSS3 need not: the flag is set for RGSS2 only.
- The numbers of `rgss3_api.txt`, counted from the method list on 2026-10-13: 344 entries,
  63 of the class itself, 30 with arity -1; `Color` 17, `Tone` 17, `Rect` 18, `Table` 10,
  `Window` 53. If the count of the flow differs, the method list is right; write the
  difference under Follow-ups.
- `arity_rgss3.rs` loads `tests/method_list/mod.rs` of `vkxp-binding` through `#[path]`.
  `Module#public_instance_methods` returns Symbols over Ruby 1.9.
- A scenario that names a message asks for that message; one that names only a class asks
  for the class.
- `values_rgss3.rs` defines a class for the scenario "A subclass and instance variables"
  under a name of its own, so that it meets no other test of the binary.

## Follow-ups

Left out, with the measured source. Sections are those of the API report unless the
bitmap report is named.

- `Font`, all of it: bitmap report 4, 5 and 7; `bind-font-and-text-of-rgss3`.
- `==` of 3.0.0 (`TypeError: can't convert NilClass into Color`, the same for `Tone` and
  `Rect`): 0 and 0.2; the samples `.300.txt`, lines 20 to 22, 47, 66 and 67.
- RGSS1 and RGSS2: the original of RGSS2 says `wrong # of arguments (E for G)`, raises
  `TypeError` for `==` with another class, and gives zeros for `Color._load("abc")`,
  (25185, 0, 0, 0) for `Rect._load("ab")` and an empty table for `Table._load("ab")`: the
  comparison file. vkxp words Ruby's order, returns false and raises `ArgumentError` in
  all three.
- `Color._load` of 32 bytes without clamping: 0. `to_s` of a component out of every range
  ending the process: 2.4.
- Raw copies between native classes by `set(other)`: bitmap report 3 and 14, point 5.
  `Color#set(rect)` was not tried on the original.
- The word size: the message of `RangeError`, the wrap of integers from 2^30
  (`Rect.new 2**30`, `Rect.new 2**31-1` of the bitmap samples), `t[1, 1, 1] = 2**31`,
  which raises `RangeError` on the original and stores the low 16 bits in vkxp: 0; bitmap
  samples, lines 243 to 247.
- `Table#[]=` with one index too few: `t[0, 0] = 5` on a table of three dimensions
  returned 5 on the original and, by a later read, wrote the element (0, 0, 0). One sample,
  no rule; vkxp raises `ArgumentError`, with `(4 for 3)` there, the number of dimensions
  plus one first, as for `[]=` with more arguments than that: not measured. `Table.new` and
  `resize` with no size or with four: not measured.
- `Rect._load` of more than 16 bytes, `Tone._load` of another length than 32: not
  measured.
- The class of what `_load` returns for a subclass; what a loaded object has of the
  instance variables of the dump: 2.4 states the dump only.
- `hash` of a `Rect` and a `Tone`: measured for `Color` alone.
- The label of `Table#inspect`, ASCII-8BIT on the original (3.3): it is the interpreter's
  `Object#inspect`, and no scenario of this change holds its encoding.
- The names interned at the first change of a value object (`observer`): 4.1.
