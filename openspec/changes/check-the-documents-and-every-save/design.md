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

## Follow-ups

- Moving the `vkrgss_*` methods and `Graphics._reset` under a `VKRGSS` module (behaviour
  change; not done here).
