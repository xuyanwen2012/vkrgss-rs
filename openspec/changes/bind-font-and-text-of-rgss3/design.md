## Context

Row 6 of the table "the changes that follow" of `run-rgss3-games-in-a-guest-of-their-own`:
`Font` with `outline` and `out_color`, the defaults, the fonts of `<RTP>/Fonts` and
`<game>/Fonts`, `draw_text` and `text_size` by the rules of RGSS3, the glyph rule and
`RGSS_VERSION` by the DLL of the game, the largest bitmap; over `FontDefaults::rgss3()`,
`TextRules::Rgss3`, `squeeze_width_rgss3` and `LibraryVersion`; after
`bind-the-values-of-rgss3`.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{font,bitmap,text,testing,guest,rb,runtime}.rs`,
  `prelude_rgss3.rb`; `crates/vkxp-guest-main/src/{font,lib}.rs`;
  `crates/vkxp-vfs/src/library_version.rs`; `crates/vkxp-core/src/{font,text}.rs`;
  `crates/vkxp-text/src/engine.rs` (`has_family_exactly`, `first_existing`,
  `set_mask_rules`); `crates/vkxp-guest/src/sandbox.rs`.
- The design of `draw-text-and-bitmaps-as-rgss3` (archived), its rules 1 to 11 and its
  Follow-ups, which list what this change binds; `docs/decisions.md`, "VX Ace", the part
  "Text and bitmaps of RGSS3".
- The designs of `bind-the-values-of-rgss3` (decisions 5 and 12: `rb::check_count_rgss3`,
  `arity_rgss3.rs` and `BOUND`; fact 3: a test of RGSS3 is a file of
  `crates/vkxp-binding-rgss3/tests/` that calls `testing::start_as(Runtime::Rgss3)`) and of
  `run-rgss3-games-in-a-guest-of-their-own` (decisions 9 and 17: `prelude_rgss3.rb`, the
  tests of a run of RGSS3 in the binding and through both executables).
- The measurements, outside the repository: `../../ref/tools/oracle/measurements/
  ace-v1-bitmap-REPORT.md` ("the bitmap report": sections 2, 4, 5, 6a, 6g, 7, 12, 13, 14)
  with `ace-v1-bitmap-samples.txt` ("the samples": lines 3 to 169 on `Font`, `font=` and
  name lookup, 270 to 400 on `Bitmap.new` and the errors of the methods, 1229 to 1243 on
  the calls without arguments); `ace-v0-REPORT.md` ("the V0 report": the table of DLLs,
  "Fonts", section 8); `ace-v1-api-REPORT.md` section 3.3 (encodings);
  `../../ref/RGSS301_api.md` ("the method list").

What the binding does in a game of RGSS3 today, and what was measured:

| | Today (the branch of RGSS2) | RGSS3, measured |
|---|---|---|
| defaults | `["Verdana", "Arial", "Courier New"]`, 20, shadow on | `"VL Gothic"` (a String, ASCII-8BIT), 24, shadow off, outline on, out colour (0, 0, 0, 128) |
| `outline`, `out_color`, their defaults | not defined | defined |
| `size=` | 6 to 96 by `rb::float`, the fraction dropped | 6 to 96 on the truncated value by `NUM2INT`, the number kept |
| `name=` | a String or an Array of Strings, read back rebuilt | any object, kept |
| `default_bold=` and the other flags | read back true or false | read back the object given |
| `default_color=` | copied into the class's own object | the object given is kept and stays live |
| `Font.exist?` | the engine's lookup, without regard to case, substitutes included | exact and case-sensitive |
| `Font.new` | 0 to 2 arguments, message `(3 for 2)` | `(3 for 0..2)` |
| `Bitmap#font=` | copies five attributes, keeps the shadow | copies all eight |
| `draw_text`, `text_size` | `Squeeze::Rgss2` | `TextRules::Rgss3` |
| `Bitmap.new(w, h)` | at most 4194304 pixels; a negative height fails | no such limit; `(10, -5)` is 10 by 5 |
| messages | Ruby's order, `wrong argument type X (expected Font)` | see decision 9 |
| glyph rule | `MaskRules::Plain` | `Rgss3`, `Rgss3Early` for a DLL 3.0.0.x |
| `RGSS_VERSION` | `"3.0.1"` | `"3.0.0"` on a DLL 3.0.0.x, `"3.0.1"` on 3.0.1.x |
| fonts | `--font-dir`, `<game>/Fonts`, the system | also `<RTP>/Fonts` |

Three facts:

1. **The sandbox does not restrict reading.** Landlock confines a guest process to writing
   below the game directory and executing nothing; it "reads every file as before"
   (`sandbox.rs`). The guest reads `Game.ini` itself (`guest_inputs`) and opens the game's
   files in `guest_config`, before the first script. The Follow-up of the archived change
   ("the confined guest does not read the DLL"; the version "has to reach the guest process
   in its settings record") rests on a wrong premise, and the guest record of
   `run-rgss3-games-in-a-guest-of-their-own` does not carry the version.
2. **`TextBackend` has one question today**, `font_exists`, which `FontBackend` answers with
   `TextEngine::is_available` (case-insensitive, substitutes counted) and `CellBackend` of
   the harness by exact comparison. The exact question of RGSS3 is
   `TextEngine::has_family_exactly`, which no backend offers.
3. **The size of a core `Font` is an `i32`** and the binding of RGSS2 drops the fraction; the
   archived decision 10 left keeping the Float to the binding.

## Goals / Non-Goals

**Goals:**

- `Font` and `Bitmap` of a game of RGSS3 answer every form and error the measurement lists,
  and their method surface matches the method list.
- Text of RGSS3 is drawn by the rules of RGSS3, with the glyph rule of the DLL the game
  ships, from the fonts the original loads.
- `RGSS_VERSION` follows the DLL the game ships.
- RGSS1 and RGSS2 do not change.

**Non-Goals:**

- Any change of `vkxp-core` or `vkxp-text`: the rules below the binding are done.
- The glyph shapes of GDI; text wider than 640 pixels beyond what the core does (it ends at
  the work rectangle); the width a font gets for an `lfWidth` (archived Follow-ups).
- The message "Unable to find VL Gothic font." and a substitution entry for "VL Gothic".
- The count messages and type messages of `Bitmap` and `Font` in RGSS1 and RGSS2.
- `Bitmap.new(10, -5)` in RGSS1 and RGSS2 (Follow-ups).
- `Color`, `Tone`, `Rect`, `Table` (`bind-the-values-of-rgss3`); `Window#contents` and
  sprites (their changes).
- The image-file size limit, `Graphics.snap_to_bitmap` (sprite change).

## Decisions

1. **Selected by the runtime of the game.** `font::define` and `bitmap::define` take the
   runtime already. For `Runtime::Rgss3` they register the functions of RGSS3 for the
   methods of the table; every other method, and every method of RGSS1 and RGSS2, is
   registered as today. `rb::RUBY_19` is not asked.

   | Class | Registered for RGSS3 only |
   |---|---|
   | `Font` | `initialize`, `initialize_copy`, `name`, `name=`, `size`, `size=`, `color=`, `outline`, `outline=`, `out_color`, `out_color=`, `default_name`, `default_name=`, `default_size`, `default_size=`, the four flag defaults with their writers, `default_color=`, `default_out_color`, `default_out_color=`, `exist?` |
   | `Bitmap` | `initialize`, `initialize_copy`, `font=`, `draw_text`, `text_size`, `blt`, `stretch_blt`, `fill_rect`, `set_pixel`, `clear_rect`, `gradient_fill_rect` |

   `initial_defaults` gives `FontDefaults::rgss3()` for `Runtime::Rgss3`, and the guest
   installs the `Color` object of `Font.default_out_color` as it installs that of
   `Font.default_color`.

2. **What a font of RGSS3 keeps** (bitmap report 4; samples 3 to 104). The core `Font` holds
   what is drawn with; the Ruby objects a script gave are kept beside it, in instance
   variables without `@` as the colour is today (marked with the object, invisible to
   scripts, copied by `dup` and `clone`).

   | Attribute | Kept for the reader | Given to the core |
   |---|---|---|
   | `name` | the object given (a String, an Array, `nil`, an Integer, anything) | decision 3 |
   | `size` | the number given, after the check | its integer part |
   | `bold`, `italic`, `shadow`, `outline` | true or false | the same |
   | `color`, `out_color` | the font's own `Color` object, the same at every call | its value |

   The class keeps, the same way: the objects given to `default_name=`, `default_size=` and
   the four flag writers (the reader returns that object; before any assignment it returns
   the initial value, a String, an Integer, true or false), and the `Color` objects given to
   `default_color=` and `default_out_color=`, which are kept, not copied: a later change of
   such an object changes the default (samples 89 to 92). The core `FontDefaults` of the
   guest holds the derived values, the truth of a flag object among them, and `Font.new`
   and a new bitmap copy from it at the moment they are made. A new font's name and size
   read the default's objects; the colours are copies (sample 93).

3. **The name a font of RGSS3 is drawn with** (bitmap report 7, samples 138 to 158). An
   Array: its first element that exists (decision 4), else its first element; an empty
   Array: no name. Anything else, and the element chosen: its `to_s`, a String as it is.
   The result is cut to its first 31 characters; an empty result is no name. It is settled
   when the name is assigned (`name=`, `Font.new`, `default_name=`): the set of fonts does
   not change during a run. The core `Font` then holds this one name, or none. It goes to
   the engine's family lookup, which ignores case and replaces a missing name, as GDI's
   mapper did in the measurement. A font without a name draws nothing; its `text_size` is
   (0, 0, 0, 0), where the original returns uninitialised numbers (known difference). The
   choice is a function of the names and a predicate, unit-tested without Ruby.

4. **`Font.exist?` of RGSS3** (bitmap report 7). `TextBackend` gets
   `font_exists_exactly(&self, name) -> bool`: `FontBackend` answers with
   `TextEngine::has_family_exactly`, `CellBackend` by exact comparison with its families.
   A String is asked as it is, a Symbol by its name; `nil`, an Integer, an Array and every
   other object answer false without an error (other classes were not measured; false is
   the answer for anything that is no name). `font_exists` stays for RGSS1 and RGSS2.

5. **Sizes** (bitmap report 4; samples 12 to 38). `size=`, `default_size=` and the size of
   `Font.new` convert by `rb::int` (`NUM2INT`: a Float is truncated, `nil` raises
   `TypeError` "no implicit conversion from nil to integer", a String "can't convert String
   into Integer"), and a result outside 6 to 96 raises `ArgumentError` "bad value for size"
   (for `default_size=`: "bad value for default_size"). The number given is kept for the
   reader and the core gets the integer. That `Font.new` keeps a Float it is given is
   inferred from `size=`; only Integers, `nil` and out-of-range values of `Font.new` were
   measured.

6. **Flags and colours** (samples 39 to 98). `bold=`, `italic=`, `shadow=`, `outline=`:
   false for `nil` and `false`, true for anything else, and the writer returns its
   argument. `color=` and `out_color=` take a `Color` only and copy its value into the
   font's own object; `default_color=` and `default_out_color=` take a `Color` only and
   keep it. Anything else raises `TypeError` "can't convert <class> into Color" with the
   class name (`NilClass`, `Tone`), the form `gradient_color_arg` of `bitmap.rs` has today.

7. **`Font.new`, copies and comparison** (samples 10 to 18, 103 to 105). `Font.new` takes 0
   to 2 arguments; more raise `ArgumentError` "wrong number of arguments (3 for 0..2)", for
   which `rb.rs` gets `check_count_range(ruby, args, min, max)`, worded `wrong number of
   arguments (<given> for <min>..<max>)`. The name is not checked. `dup` and `clone` copy
   every attribute and give the copy colour objects of its own. `==` is `Object#==`.
   `Marshal.dump` of a font raises `TypeError` "no marshal_dump is defined for class Font",
   which is Ruby 1.9.2's own for a data object without `_dump`: no code.

8. **`Bitmap` of RGSS3** (bitmap report 2, 5, 6g; samples 159 to 169, 270 to 303,
   354 to 372).
   - `font=` takes a `Font` only ("can't convert <class> into Font") and copies name
     object, size object, bold, italic, shadow, outline and the values of both colours into
     the bitmap's own `Font` object, which keeps its own colour objects. `dup` and `clone`
     give the copy a font of its own with all eight.
   - `draw_text` calls `Bitmaps::draw_text_as` with `TextRules::Rgss3`; `text_size` calls
     `Bitmaps::text_size`, which counts neither outline nor shadow. The text is the `to_s`
     of any object (`text_of`); `align` goes through `rb::int` and 1 and 2 are centre and
     right, everything else left, as in RGSS2. With a font without a name `draw_text`
     returns `nil` after its arguments are checked and draws nothing.
   - `Bitmap.new(width, height)`: `rb::int` for both; a width of 0 or less, or a height of
     0, raises `RGSSError` "failed to create bitmap"; a negative height is taken by its size
     (`(10, -5)` is 10 by 5, as measured; one sample). No limit of 4194304: the arena's
     `MAX_BITMAP_PIXELS` decides (decision 7 of the archived change), and a size above it
     raises `RGSSError` "failed to create bitmap". `Bitmap.new(path)` is that of RGSS2.

9. **Argument errors of RGSS3** (bitmap report 1 and 2; samples 270 to 372, 1229 to 1243).
   The binding words what the original words itself; what Ruby 1.9.2 raises by itself
   (conversions by `rb::int` and `rb::text`, a wrong count of a method of fixed arity,
   `wrong number of arguments(1 for 2)` without the space) needs no code.

   | Call | Count accepted | Message of a wrong count |
   |---|---|---|
   | `Bitmap.new` | 1 or 2 | `(G for 1..2)` (decision 7's function) |
   | `blt` | 4 or 5 | `check_count_rgss3(4, 5)`: `(4 for 3)`, `(5 for 6)`, `(4 for 0)` |
   | `stretch_blt` | 3 or 4 | `check_count_rgss3(3, 4)`: `(3 for 2)`, `(3 for 0)` |
   | `fill_rect` | a `Rect` and a `Color`, or 5 | `(5 for G)` for every other call, `(rect, nil)` included |
   | `clear_rect` | a `Rect` first: 1; otherwise 4 | `check_count_rgss3` with those bounds: `(4 for 0)`, `(4 for 2)` |
   | `gradient_fill_rect` | a `Rect` first: 3 or 4; otherwise 6 or 7 | `check_count_rgss3` with those bounds: `(6 for 0)` |
   | `draw_text` | a `Rect` first: 2 or 3; otherwise 5 or 6 | `check_count_rgss3` with those bounds: `(2 for 1)`, `(3 for 4)`, `(5 for 4)`, `(6 for 7)`, `(5 for 0)` |

   Wrong types: a `Color`, `Rect` or `Bitmap` argument of `set_pixel`, `fill_rect`,
   `gradient_fill_rect`, `blt` and `stretch_blt`, and the `Font` of `font=`, raise
   `TypeError` "can't convert <class> into <Color|Rect|Bitmap|Font>". The bitmap report
   lists the counts without arguments in another order than its samples; the samples are
   followed (gradient 6, `draw_text` 5). The rule "a `Rect` first chooses the short form,
   the bound nearer the count given is named" is measured for `draw_text` (five counts) and
   fits every measured count of `clear_rect` and `gradient_fill_rect`; it is applied to
   their other counts. RGSS2's function of `gradient_fill_rect` says `(3 for 0)` and is not
   the model: its count without arguments was never measured (the original of RGSS2 ends).

10. **The version of the DLL is read by the guest** (fact 1). `guest_config` of
    `vkxp-guest-main`, for a game of RGSS3 only, resolves `ini.library` with
    `GameFiles::resolve(FileKind::Data, ..)` (the game directory, letter case and `\`
    accepted, never the archive), reads it and gives the bytes to `LibraryVersion::read`.
    An empty `Library`, a file that is not found or cannot be read, and bytes without a
    record give no version; the guest writes one line of the log with the path and the
    version or why there is none, and goes on. The version goes to two places:
    - the text backend: `FontBackend::for_game` takes the runtime and the version and sets
      the engine's mask rules: `Plain` for RGSS1 and RGSS2, `Rgss3Early` for RGSS3 with a
      version whose first three numbers are 3.0.0, `Rgss3` for RGSS3 otherwise;
    - the binding: `RunConfig` gets `library_version: Option<LibraryVersion>` (`None` by
      default), and the preparation of RGSS3 defines `RGSS_VERSION` as `"3.0.0"` for a
      version 3.0.0.x and `"3.0.1"` otherwise, ASCII-8BIT, before `prelude_rgss3.rb`,
      whose line becomes a comment line or goes so that the constant is defined once.
    `LibraryVersion` gets `is_3_0_0(&self) -> bool`, the one place of the rule "the first
    three numbers are 3.0.0", which both callers use. Nothing is added to the guest record,
    the protocol or the program: the program does not read the DLL. Rejected: a field of
    the record (the record carries the settled options of the command line; the facts of
    the game's files are read by the guest, as `Game.ini` is), and reading in the program
    (one more file read by a process that does not otherwise look into the game).
    A guest on a thread never runs a game of RGSS3, so nothing is read there.

11. **Fonts of a game of RGSS3** (V0 report, "Fonts"; bitmap report 7). The directories of
    the engine are, in this order: those of `--font-dir`; the game's `Fonts` directory
    when it exists; for a game of RGSS3 the `Fonts` directory of each run-time package of
    the game when it exists, in the order of the packages; then the system's fonts. The
    game comes before the packages as in the search of the game's files. The list is made
    by a function of `font.rs` that is unit-tested. RGSS1 and RGSS2 keep their list: their
    packages have no `Fonts` directory that the original reads (not measured). "VL Gothic"
    gets no entry of the substitution table: a game of VX Ace names the package or ships
    its fonts, and without either the engine's fallback families draw, where the original
    shows "Unable to find VL Gothic font." (known difference).

12. **Method surface.** `Font` and `Bitmap` join the list `BOUND` of
    `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs`; nothing else of that file or of
    `rgss3_api.txt` is edited. The two lists of the method list equal those of RGSS2 for
    `Bitmap`, and add `outline`, `out_color`, `default_outline`, `default_out_color` with
    their writers for `Font`.

13. **Tests.**
    - `crates/vkxp-binding-rgss3/tests/font_rgss3.rs` and `bitmap_rgss3.rs`: the scenarios
      of `Font` and `Bitmap` through `testing::eval` after
      `testing::start_as(Runtime::Rgss3)`, one evaluation per measured line or group. The
      harness has the families "Test Sans" and "Liberation Sans" (`CellBackend`), which
      stand for existing fonts; its rasteriser is the core's `CellRasterizer`.
    - `crates/vkxp-binding-rgss3/tests/version_rgss3.rs`: `RGSS_VERSION` by
      `RunConfig::library_version`, through a run of a game of RGSS3 in the form of
      `run_game_rgss3.rs`.
    - Unit tests: the name choice (decision 3) in `font.rs`; `is_3_0_0` in
      `library_version.rs`; the mask rules, the font directories and the version read from
      a game directory in `vkxp-guest-main` (they run in both glue packages), with DLL
      bytes built in the test and a font file copied from `crates/vkxp-text/tests/fonts`.
    - `crates/vkxp/tests/rgss3.rs`: two runs of a generated game of RGSS3 whose
      `Library=System\RGSS300.dll` is a file holding a version record, 3.0.0.1 and
      3.0.1.1; the game reports `RGSS_VERSION`.
    - XP and VX: one new test in `crates/vkxp-binding/tests/bitmap_rgss2.rs` (a binary of
      RGSS2) and one in `tests/ruby.rs` (RGSS1), both over both interpreters, that hold
      that the forms of RGSS3 are not there. No existing test is edited.
    - No entry of `ruby192.allow` is about `Font` or `Bitmap`; if one starts to pass, the
      mechanism reports it and the entry and its marker are removed.

14. **Documents.** `docs/compatibility.md`: a subsection "Font and text of VX Ace" under
    "RPG Maker VX Ace", in the style of "Bitmap and Font of VX", with the known differences
    of the Follow-ups below; the bullet that `RGSS_VERSION` is `"3.0.1"` whatever DLL the
    game ships goes, and `Font` and text leave the list of classes that are still those of
    VX in that section, if it names them. `docs/decisions.md`: in "Text and bitmaps of
    RGSS3" the sentence that applying the table waits for the binding becomes how it is
    applied (decision 10), with `RGSS_VERSION` by the same table; the sentence of the
    preparation of RGSS3 that `RGSS_VERSION` does not follow the DLL is brought up to date;
    the fonts of decision 11. `docs/running.md`: the row of `--font-dir` and the
    paragraph on VX Ace name `<RTP>/Fonts`. `CHANGELOG.md`: one entry under "Unreleased".
    The opening of the documents and the state of VX Ace in them are not touched.

## Implementation notes

- A flow reads `../../ref/` and copies nothing from it but rules, names, arities and
  numbers. No Wine and no original runtime is run. Scratch output goes under `target/` or
  `local/`.
- Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding`,
  `vkxp-binding-rgss3`, `vkxp-guest-main`, `vkxp-guest-main-rgss3` and `vkxp-vfs` under
  `local/`: every name must be there at the end.
- The objects kept in instance variables are reached through the object that holds them;
  nothing of this change holds a `VALUE` in Rust across a call into Ruby. The collector
  stress step of `cargo xtask stress` covers the new files by running the binding's tests.
- `font_arg` returns the core `Font` with both colours; `FontData::blank` already has the
  two fields.
- A scenario that names a message asks for that message; one that names only a class asks
  for the class. Colours are compared by `to_s`, which is
  `"(255.000000, 255.000000, 255.000000, 255.000000)"` in RGSS3.
- The pixel scenario of decision 8 uses a transparent bitmap of 40 by 40, the default font
  of RGSS3 (outline on, white) and the text `"x"`: by rule 7 of the archived design the
  last pass writes the text colour with alpha 0 over every pixel of the work rectangle that
  no glyph covers, inside its border of one pixel. If the harness's cell glyph reaches the
  pixel named, the flow names another pixel of that kind and says so.
- `bitmap_rgss3.rs` makes no bitmap above 2049 by 2048: 16384 by 16384, the largest size
  the measurement saw created, is a GiB of pixels.

## Risks

- `rb::int` over Ruby 1.9.2 of a Bignum raises `RangeError` with the 64-bit message; the
  measured `bignum too big to convert into 'long'` is a 32-bit one. Not followed, as in
  `bind-the-values-of-rgss3`.
- A shared test of RGSS2 that asserts the message of a wrong count of `Bitmap` over Ruby
  1.9.2 keeps passing: the functions of RGSS2 do not change.

## Edits beyond the files named

Made during the implementation because a requirement plainly needed them; each is a few
lines.

- `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs`: beside the line of `BOUND`, the line
  that asserts the number of entries of each bound class gets the counts of `Font` (35) and
  `Bitmap` (22); it is computed from `BOUND` and fails for any other length. The change
  `bind-the-window-of-rgss3` edited the same line when it added `Window`.
- `crates/vkxp-binding-rgss3/tests/strings.rs`, `a_font_name_is_bytes`: its second
  assertion expected `Font.new('Test Sans').name` of RGSS3 to be ASCII-8BIT, which held
  while the font of RGSS3 was that of RGSS2 and rebuilt its name. The requirement "Font
  names of RGSS3" keeps the object given, and the original was measured to keep a name as
  given (API report 3.3): the assertion now expects the encoding of the script's literal,
  UTF-8, with a comment. Its first assertion, the initial `"VL Gothic"` in ASCII-8BIT, is
  unchanged.

## Follow-ups

Left out, with the measured source. Sections are those of the bitmap report unless another
report is named.

- Known differences for `docs/compatibility.md` (decision 14): the largest bitmap (the
  original created 8192 by 44954 in one run, which vkxp refuses; 32768 by 32768 failed
  there by memory, not by a rule; 2); text wider than 640 pixels, which the original writes
  640 columns to the left and one row up and vkxp ends at the work rectangle (6g); a font
  without a name, whose `text_size` is uninitialised on the original and (0, 0, 0, 0) in
  vkxp (6g); `"@VL Gothic"` and the other vertical names, true on the original (7);
  substituted names (`Arial` on a machine with Liberation Sans) that count as existing in
  vkxp (7); "Unable to find VL Gothic font." not shown (7; V0 report, the share DLL); the
  other language's name of the two VL fonts ("VL ゴシック"), which the original adds and
  vkxp knows only if the font file declares it (7); everything GDI answers (14, point 1).
- `RGSS_VERSION` and the glyph rule for a DLL of another version than 3.0.0.1 and 3.0.1.1:
  none exists in the corpus; the table of `docs/decisions.md` decides.
- `Bitmap.new(10, -5)` in RGSS1 and RGSS2: the VX report (section 2) measured 10 by 5 on
  both runtimes; vkxp raises `RGSSError` there. Found while drafting this change; a
  change of RGSS1 and RGSS2 is outside its scope.
- `Font.exist?` of an object of another class than those measured (a Float, an object with
  `to_s`): answered false (decision 4).
- `Font.new` with a Float size, and whether `Font.new` shares the default's name and size
  objects or copies them: not measured (decisions 2 and 5).
- `draw_text` with two or three arguments whose first is no `Rect`, `fill_rect` with a
  first argument that is no `Rect`, `clear_rect` and `gradient_fill_rect` with counts other
  than 0 and 2: not measured; the rules of decision 9 decide.
- The `text_size` tables of "VL Gothic" (8; 28392 values): a test skipped without the font
  under `local/fonts`, as for Noto Sans CJK SC, can hold them (archived Follow-up).
- The width for an `lfWidth` for all three runtimes (archived Follow-up).
- The encoding of `Font#name` when the name came from `default_name=` with a literal: kept
  as given (API report 3.3); the initial `"VL Gothic"` is ASCII-8BIT and a scenario holds
  that.
