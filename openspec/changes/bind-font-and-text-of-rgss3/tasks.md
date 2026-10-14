# Tasks

## 1. Font of RGSS3

- [ ] 1.1 Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-guest-main`, `vkxp-guest-main-rgss3` and `vkxp-vfs` under `local/`. Check that `bind-the-values-of-rgss3` and `run-rgss3-games-in-a-guest-of-their-own` are merged (`rb::check_count_rgss3`, `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs` with `BOUND`, `crates/vkxp-binding/src/prelude_rgss3.rb`, `crates/vkxp-guest-main-rgss3`); if not, stop and say so. Verify: the lists are kept and the four items exist.
- [ ] 1.2 `TextBackend::font_exists_exactly` with its two implementations (`CellBackend` in `testing.rs`, `FontBackend` in `crates/vkxp-guest-main/src/font.rs` over `has_family_exactly`), `rb::check_count_range`, `initial_defaults` of RGSS3, the `Color` object of `Font.default_out_color` in `guest.rs`, and the functions of RGSS3 of `font.rs` (design decisions 1 to 7) with the unit test of the name choice; `crates/vkxp-binding-rgss3/tests/font_rgss3.rs`; `Font` in `BOUND`. Verify: the scenarios of "Font of RGSS3" but the third, "Font sizes of RGSS3" but the last, "Font names of RGSS3" but the last two, "Font flags, colours and copies of RGSS3" and the second of "Method surface of Bitmap and Font in RGSS3"; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 -p vkxp-guest-main -p vkxp-guest-main-rgss3 --locked` passes.

## 2. Bitmap of RGSS3

- [ ] 2.1 The functions of RGSS3 of `bitmap.rs` (design decisions 8 and 9): `font=`, `initialize` and `initialize_copy`, `draw_text` and `text_size` with `TextRules::Rgss3` and the font without a name, the counts and types of `blt`, `stretch_blt`, `fill_rect`, `set_pixel`, `clear_rect` and `gradient_fill_rect`; `crates/vkxp-binding-rgss3/tests/bitmap_rgss3.rs`; `Bitmap` in `BOUND`. Verify: the scenarios of "Bitmap font and copies of RGSS3", "Text of a bitmap of RGSS3", "Creation from a size in RGSS3", "Argument errors of Bitmap in RGSS3", the first of "Method surface of Bitmap and Font in RGSS3", the third of "Font of RGSS3", the last of "Font sizes of RGSS3" and the last two of "Font names of RGSS3"; the same test command passes.
- [ ] 2.2 XP and VX: one new test in `crates/vkxp-binding/tests/bitmap_rgss2.rs` and one in `tests/ruby.rs` (design decision 13). Verify: the scenarios of "Font and Bitmap of RGSS1 and RGSS2 are unchanged by RGSS3"; `git diff` against the start of the change adds lines and removes none in those two files and in `arity_rgss3.rs` (but the line of `BOUND`), and is empty for `ruby192.allow` unless the mechanism reported an entry that now passes; the same test command passes.

## 3. The library and the fonts of a game of RGSS3

- [ ] 3.1 `LibraryVersion::is_3_0_0` with its unit test; `RunConfig::library_version` and `RGSS_VERSION` by it in the preparation of RGSS3 (design decision 10), `prelude_rgss3.rb` defining the constant no second time; `crates/vkxp-binding-rgss3/tests/version_rgss3.rs`. Verify: the first, second and last scenarios of "Version of RGSS3 by the library of the game"; `cargo test -p vkxp-vfs -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes.
- [ ] 3.2 In `vkxp-guest-main`: the version read in `guest_config` for a game of RGSS3, the mask rules and the font directories of `FontBackend::for_game` (design decisions 10 and 11), with their unit tests. Verify: the scenarios of "Fonts of a game of RGSS3" and "Glyph rule by the library of a game of RGSS3"; `cargo test -p vkxp-guest-main -p vkxp-guest-main-rgss3 --locked` passes; `git diff --stat` against the start of the change names no file of `crates/vkxp-guest`, `crates/vkxp-proto`, `crates/vkxp-host`, `crates/vkxp-core` or `crates/vkxp-text`.
- [ ] 3.3 Two tests in `crates/vkxp/tests/rgss3.rs` (design decision 13). Verify: the scenario "Through both executables"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp -p vkxp-guest-rgss3 --locked --test rgss3 --test guest_record` passes.

## 4. Documents

- [ ] 4.1 `docs/compatibility.md`, `docs/decisions.md`, `docs/running.md` and `CHANGELOG.md` (design decision 14); the documentation of `font.rs`, `bitmap.rs`, `text.rs`, `runtime.rs`, `crates/vkxp-guest-main/src/font.rs` and of `lib.rs` of the binding where it describes these classes. Verify: the scenarios of `specs/supported-runtimes/spec.md`; the text names nothing that the code does not do; `git diff --stat` against the start of the change names no file under `docs/` but those three; `cargo test -p xtask --locked` passes.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: every name of the lists kept in 1.1 is there again; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate bind-font-and-text-of-rgss3 --strict` passes.
