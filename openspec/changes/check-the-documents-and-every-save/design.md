# Design

## Context

- Reviews: `target/review-2026-10-26/fable-review.md` section 5 (18 items) and
  `codex-review.md` section 5 (8 rows) and defect 10. Not in the repository; the items are
  listed under "Items" below.
- This change runs after the other changes of the round (`verify-guests-and-deny-signals`,
  `end-the-session-without-the-guest`, `confine-loading-and-process-functions`,
  `call-rgss-from-script-threads`, `keep-the-audio-worker-sound`,
  `bound-the-compositor-and-survive-device-loss`, `bound-image-decoding-and-uploads`,
  `share-the-interpreter-glue`). Several items may already be fixed by them; check each against
  the code at the start of this change and correct only what still disagrees.
- `share-the-interpreter-glue` modifies the requirement "Crate layout"; this change does not.

## Decisions

- **Which files are saves.** A file below `<out>/<game>/vkrgss/game` is compared when the run
  wrote it: it is absent from the game's source directory (`dir` of `xarch.toml`) or its bytes
  differ from the source's. The oracle keeps file times but times are not trusted for this.
  The extension rule goes away. Frames are compared as now.
- **Expected saves.** A game in `xarch.toml` may set `saves = N`; `compare` fails for that game
  when fewer than N saves were compared on either side. `xarch.example.toml` and
  `docs/development.md` show it; the local list of this machine sets it for the games whose
  input saves (the owner's `local/xarch.toml` is not committed; the flow sets it there and says
  so in the run's report, and `xarch check` then shows the counts).
- **Report.** The per-game line already prints `saves X same, Y different`; it stays, and a
  game with an expectation prints `of N expected` too.
- **Document test.** `xtask/tests/architecture.rs`, beside `doc_links.rs`: parses the crate
  table of `docs/architecture.md` (first column) and compares it with the workspace members of
  `cargo metadata`; parses the `unsafe` paragraph's list per crate and compares it with
  `#![forbid(unsafe_code)]` / `#![deny(unsafe_code)]` plus `#[allow(unsafe_code)]` on modules
  in each crate's `lib.rs`/`main.rs`; reads the target lists of `vkrgss-ruby-sys/targets.rs`
  and `vkrgss-ruby192-sys/targets.rs` and checks that the sentences of the crate table and of
  the crates' top doc comments that name targets name exactly those. Keep the parsing simple
  and the document's format stable; a document change that breaks the parser fails loudly.
- **`VKRGSS` module.** The documents describe what exists (private `vkrgss_*` methods of
  `Kernel`, and `Graphics._reset`); moving them into a module is a behaviour change and goes
  under Follow-ups.
- **Corrections follow the code.** Where a document and the code disagree, the code is taken
  as the decision unless the document records a measurement of the original; such a case is a
  Follow-up, not a code change here.

## Items

Each is checked at the start and corrected if it still disagrees:

1. `docs/architecture.md`: guest `unsafe` modules (copier, exit, heap, parent_watch, seccomp,
   signals); `vkrgss-ruby192-sys` targets (crate table, `:19`), also `vkrgss-ruby192-sys/src/lib.rs`,
   `ffi.rs` of both sys crates and `vkrgss-guest-rgss3/src/main.rs`; "no `cfg`" (`:41`, and
   `docs/development.md` near 303) against `cfg(vkrgss_ruby_19)`; "the binding sends
   `TransitionRule::Rgss1` and sets no brightness" (`:83`); `vkrgss-host` "configuration" (it
   is `crates/vkrgss/src/config.rs`).
2. `docs/decisions.md`: the `VKRGSS` module (and `vkrgss-binding/src/lib.rs` near 240); the
   bundled SoundFont (`midi.rs` searches system paths; `args.rs` USAGE lacks
   `/usr/share/sounds/sf2/FluidR3_GM.sf2`); encoding "auto-detection" of `Game.ini`; the JPEG
   decision (hand-written IJG 6b decoder plus `jpeg-decoder` fallback); "Vulkan 1.3 core plus
   descriptor indexing is the only hard requirement" (65536 update-after-bind sampled images,
   256-invocation workgroups); bitmap slots "follow device limits" (`:855`, fixed 65536);
   "open" items that are built, such as `Graphics.snap_to_bitmap` (keep the history, mark them
   done).
3. `docs/running.md`: the host keys and the key map (`:310`, which keys are swallowed); F3/F4
   during a modal are dropped, not deferred (`:675`); the sandbox paragraph and the
   process-function sentence only if the earlier changes left them wrong.
4. Doc comments: `vkrgss-host/src/lib.rs` ("`demo` ... until the Ruby side exists"),
   `session.rs` (`FAST_FORWARD_KEY` "placeholder"); `vkrgss-core/src/lib.rs` ("all modules
   private except test_support", `sound` is public); `Scene::clear` (resets the frame index and
   the table bookkeeping, emits no `Destroy`); `vkrgss-vk/src/compositor.rs` ("no Vulkan object
   is in use when a method returns", apart from the presentation engine); `vkrgss-audio`
   `output.rs` "never waits"; `vkrgss-core/src/bitmap.rs` "every bitmap can be uploaded";
   `vkrgss-image/src/lib.rs` on allocation; `render.rs` held frame; `GuestForm`'s `Default`
   against the real default.
5. `docs/compatibility.md`: script threads and the RGSS API, as left by
   `call-rgss-from-script-threads`.

## Implementation notes

### Every save (group 1)

- The oracle writes the probe into the script archive of the copy (`Game.rgss3a`, or the
  file `Scripts=` names), so that file differs from the game's directory although no run
  wrote it. Counted as a save it would make every game hold one, and `saves = N` could be
  met by a run that saved nothing. `xarch` leaves it out, and asks the oracle which file it
  was: `probed` in `xtask/src/oracle/game.rs` and `probed_file` in `xtask/src/oracle/mod.rs`,
  fourteen lines outside the scope of the proposal that the requirement needs.
- Telling a save needs the game's directory, which only the list names, so `xarch compare`
  reads the list as `check` does (`local/xarch.toml`, or `--list FILE`), and fails for a game
  of the first directory that the list lacks.
- A file that the run of one side wrote and the other side holds unchanged from the game is
  counted as a save that differs, not as one that is missing.
- The list of this machine: the owner's `vkxp/local/xarch.toml` was copied to `local/xarch.toml`
  of this clone (not committed), and `saves = 3` was set there for `isoi`, the one game whose
  input saves. The owner's file was left as it is, because `xarch` of `main` refuses the new
  key; it gets the same line once this change is merged (see Follow-ups).
- `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch baseline --at 12ae44dd` (the proposal)
  and then `cargo xtask xarch check`, with `XDG_RUNTIME_DIR` below `TMPDIR`: `all identical`,
  84 frames. Saves compared per game: `isoi` 3 of 3 expected (`AutoSave.rvdata2`,
  `Save01.rvdata2` and `playoffline.txt`, which a script of the game writes), every other
  game 0. The "17 saves" of earlier reports were files whose names said "save", most of
  them files of the games' directories that no run touched; `README.md` repeated the
  number and now says what is written.

### Document test (group 2)

- `xtask/tests/architecture.rs` takes the members and their crate roots from `cargo metadata`,
  the attributes from the roots, and for a crate that denies `unsafe` the modules whose `mod`
  line follows `#[allow(unsafe_code)]` and the modules whose file holds that attribute
  (`link` of `vkrgss-proto`, whose allow is on a function in it). The scenarios with a
  missing crate and an omitted module run the check on the document with the row of
  `vkrgss-audio` and the word `seccomp` taken out; a fourth test gives it the old row of
  `vkrgss-ruby192-sys`.
- `vkrgss-ruby-sys` has no `targets.rs`: its `TARGETS` are in its `build.rs`, which the test
  reads. Neither its row nor its top doc comment names a target today.
- A sentence is checked when it names a target triple (a word of the form `x86_64-...` or
  `aarch64-...` with two hyphens), and has to name every target of the list.
- What the test found and this group corrected: the table lacked the members
  `vkrgss-android` and `xtask`; the row and the top doc comment of `vkrgss-ruby192-sys` named
  one and two of its four targets; the paragraph on `unsafe` named `vkrgss-proto` as
  forbidding it (it denies it and allows it in `link`), omitted the modules `exit`, `heap`,
  `parent_watch` and `seccomp` of `vkrgss-guest`, and said nothing of `vkrgss-image`,
  `vkrgss-text`, `vkrgss-vfs`, `vkrgss-audio`, `vkrgss-host`, `vkrgss-vk`, `vkrgss`,
  `vkrgss-android` and `xtask`. It is now a list of three items under its own heading.

## Follow-ups

- Moving the `vkrgss_*` methods and `Graphics._reset` under a `VKRGSS` module (behaviour
  change; not done here).
- The owner's `local/xarch.toml` gets `saves = 3` in the table of `isoi` after the merge.
