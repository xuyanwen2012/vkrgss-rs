## Why

A game of RPG Maker VX Ace sets `Font.default_*` and draws text before its title screen. In a
game of RGSS3 the classes `Font` and `Bitmap` are those of RGSS2 today: no `outline` or
`out_color`, the defaults of VX, the text rules of VX, the size limit of VX, and the glyph
rule and `RGSS_VERSION` of 3.0.1 whatever DLL the game ships. Everything below the binding
exists (archived change `draw-text-and-bitmaps-as-rgss3`: `TextRules::Rgss3`,
`FontDefaults::rgss3()`, `MaskRules::{Rgss3, Rgss3Early}`, `LibraryVersion`,
`TextEngine::has_family_exactly`), and its Follow-ups list what the binding has to do, each
with the measured rule (`ace-v1-bitmap-REPORT.md` of 2026-10-13, with its samples, outside
the repository). This is row 6 of the table "the changes that follow" of
`run-rgss3-games-in-a-guest-of-their-own`, on the path to milestone A1.

## What Changes

- In a game of RGSS3, and only there:
  - `Font` has `outline`, `out_color`, `Font.default_outline` and `Font.default_out_color`,
    and the defaults of RGSS3: the String `"VL Gothic"`, 24, bold, italic and shadow off,
    outline on, white, out colour (0, 0, 0, 128).
  - `Font#size=`, `Font.default_size=` and the size of `Font.new` keep the number given and
    the font is made with its integer part; the limits 6 to 96 are on that integer part.
  - `Font#name=` and `Font.default_name=` keep the object given; the font is drawn with the
    first element of an Array that exists exactly, else its first element, else the `to_s`
    of the object, cut to 31 characters. A font without a name draws nothing.
  - `Font.exist?` is exact and case-sensitive over the family names the text engine has; a
    Symbol is converted, anything else that is no String is false.
  - The `default_` writers keep the object given; `color=` and `out_color=` take a `Color`
    only and copy it; `Font.new` takes at most two arguments.
  - `Bitmap#font=` copies all eight attributes, `shadow`, `outline` and `out_color`
    included; `dup` and `clone` copy them too.
  - `draw_text` and `text_size` follow the rules of RGSS3 (`TextRules::Rgss3`).
  - `Bitmap.new(width, height)` has no limit of 4194304 pixels; the limit is the one every
    bitmap of vkxp has, 268435456 pixels. A negative height is taken by its size.
  - The argument errors of `Bitmap` and `Font` have the classes and messages measured on
    the original.
- The guest reads the version of the DLL the game's `Library` names, from the game
  directory, when it starts a game of RGSS3. A version 3.0.0.x selects the glyph rule
  `MaskRules::Rgss3Early` and `RGSS_VERSION` `"3.0.0"`; every other version, a DLL without
  a version record and no DLL select `MaskRules::Rgss3` and `"3.0.1"`. The table is the one
  `docs/decisions.md` records. Nothing is added to the guest record: the guest reads
  `Game.ini` and the game's files itself, and the sandbox does not restrict reading.
- A game of RGSS3 takes fonts also from the `Fonts` directory of each of its run-time
  packages, after the game's own `Fonts` directory.
- `Font` and `Bitmap` join the classes the method list of RGSS3 checks.
- `docs/compatibility.md` gets the font and text of VX Ace with their known differences;
  `docs/decisions.md`, `docs/running.md` and `CHANGELOG.md` say what changed.
- XP and VX are unchanged: every existing test passes over both interpreters with its
  expected values unedited.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-bitmap`: `Font` and the text and creation of `Bitmap` in a game of RGSS3, with
  their argument errors and method surface.
- `game-launch`: the fonts of a game of RGSS3 and the glyph rule by the DLL of the game.
- `rgss-runtime`: `RGSS_VERSION` by the DLL of the game.
- `supported-runtimes`: the known differences of font and text of RGSS3 in
  `docs/compatibility.md`.

## Impact

- `crates/vkxp-binding/src`: `font.rs` (the functions of RGSS3 and the definition by
  runtime), `bitmap.rs` (`font=`, `draw_text`, `text_size`, `initialize` and the argument
  errors of RGSS3), `text.rs` (one method of `TextBackend`), `testing.rs` (that method for
  `CellBackend`), `guest.rs` (the `Color` object of `Font.default_out_color`), `rb.rs` (the
  count message with a range), `runtime.rs` (one field of `RunConfig`, `RGSS_VERSION`),
  `prelude_rgss3.rb` (the line of `RGSS_VERSION`), `lib.rs` (documentation only).
- `crates/vkxp-guest-main/src`: `font.rs` (the font directories and the mask rules of a
  game of RGSS3, the new method), `lib.rs` (`guest_config` reads the version of the DLL).
- `crates/vkxp-vfs/src/library_version.rs`: one predicate, the first three numbers 3.0.0.
- Tests: new `crates/vkxp-binding-rgss3/tests/font_rgss3.rs`, `bitmap_rgss3.rs` and
  `version_rgss3.rs`; one line of `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs` (the
  list `BOUND`); one new test each in `crates/vkxp-binding/tests/bitmap_rgss2.rs` and
  `tests/ruby.rs`; unit tests in the files named above; two tests in
  `crates/vkxp/tests/rgss3.rs`.
- Documents: `docs/compatibility.md`, `docs/decisions.md`, `docs/running.md`,
  `CHANGELOG.md`.
- Depends on `bind-the-values-of-rgss3` (`rb::check_count_rgss3`, `arity_rgss3.rs` with
  `BOUND`, `rgss3_api.txt`) and on `run-rgss3-games-in-a-guest-of-their-own`
  (`prelude_rgss3.rb`, `RunConfig` as it leaves it, `vkxp-guest-main-rgss3`,
  `crates/vkxp/tests/rgss3.rs`, the section "RPG Maker VX Ace" of `docs/compatibility.md`),
  both merged first.
- Other changes in flight that touch the same files: `run-rgss3-games-in-a-guest-of-their-own`
  (`runtime.rs`, `prelude_rgss3.rb`, `vkxp-guest-main/src/lib.rs`, `crates/vkxp/tests/rgss3.rs`,
  the four documents); `bind-the-values-of-rgss3` (`rb.rs`, `guest.rs`, `tests/ruby.rs`,
  `arity_rgss3.rs`, `docs/compatibility.md`); the draft `bind-the-window-of-rgss3`
  (`arity_rgss3.rs`, `docs/compatibility.md`); the draft `bind-audio-of-rgss3` (`guest.rs`,
  `runtime.rs`, `docs/compatibility.md`, `docs/decisions.md`, `docs/running.md`,
  `CHANGELOG.md`); `bind-the-tilemap-of-rgss3` (`docs/compatibility.md`, `CHANGELOG.md`).
  Every edit of this change to a shared file is an addition or a line it owns.
- Not touched: `vkxp-core`, `vkxp-text`, the protocol, the host crates, the guest record
  (`vkxp-guest`), `third_party/`, `ruby192.allow`, `tests/data/rgss3_api.txt`, every other
  file of the binding.
- Nothing of the original is committed: no DLL byte (the tests build a version record in
  the test), no font of the run-time package, no script text.
