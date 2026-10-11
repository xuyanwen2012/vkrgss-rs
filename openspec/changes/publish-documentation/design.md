# Design

## Context

Facts the documents may state (do not state more than these and what the repository itself
shows):

- vkxp runs RPG Maker XP (RGSS1) games on Linux x86_64: window, audio (Ogg Vorbis, WAV, MP3,
  MIDI with a SoundFont), keyboard and gamepads, encrypted archives, configuration files,
  fast-forward, input recording and replay, save states, a headless mode.
- Exactness: frames are compared with the original runtime (run under Wine by
  `cargo xtask oracle`). On the seven community-made games used as the regression corpus the
  frames compared are pixel-identical except for text glyphs, whose rasterisation differs
  from Windows GDI. Many rules were measured on the original runtime; the specifications
  under `openspec/specs/` record them. Do not name the games.
- The interpreter is Ruby 1.8.7-p374 built from vendored source, with Oniguruma 5.9.6.
- Not included and never to be included: the original runtime, its DLLs, the RTP assets,
  Enterbrain's `RPG` module source. A user supplies a game and extracts the built-in scripts
  from a DLL of their own (`cargo xtask extract-rpg`).
- Version 0.1.0 is the first public release. The owner is Yanwen Xu; the repository will be
  `https://github.com/xuyanwen2012/vkxp`.

## Decisions

### README as a landing page

At most 200 lines, in this order: title and one-paragraph description; a CI badge
(`https://github.com/xuyanwen2012/vkxp/actions/workflows/ci.yml/badge.svg`) and a licence
badge; the screenshot; "Status" (what works, the exactness statement, the known gap of text
glyphs, Linux x86_64 only); "Quick start" (prerequisites in one line with a link, build, the
built-in scripts, run a game, the main keys); "How it works" (guest and host, the crate
table, the two compositors, one paragraph); "Documentation" (links to the files below);
"Legal" (the notice below); "Licence" (MIT; `THIRD-PARTY.md` for the bundled source).

### The manual under `docs/`

The sections of the present README are moved, text kept apart from what the move needs
(heading levels, links, the removal of sentences about the owner's machine):

| File | Sections of the present README |
|---|---|
| `docs/running.md` | Running a game and everything under it; Running the demo |
| `docs/building.md` | Prerequisites |
| `docs/architecture.md` | Layout, the protocol paragraph, Headless proof, Dependency rules |
| `docs/development.md` | Merge gate, Checks outside the gate, Comparing with the original runtime, a short account of the OpenSpec workflow (`openspec/specs` are the specifications, `openspec/changes/archive` the history of changes) |
| `docs/compatibility.md` | new: what is exact and how that is established; known differences, one line for each group of the `PLACEHOLDER` notes that remain in `crates/` (text metrics and glyphs, Win32API shims), and the limits of save states |
| `docs/decisions.md` | unchanged |

Sentences that name the Intel Arc B580 or "the development machine" are rewritten to the
general fact: the Vulkan loader loads every installed driver, so the tests ask for
`VK_LOADER_DRIVERS_SELECT`, with lavapipe and RADV as the examples.

### Legal notice

"vkxp is an independent project. It is not affiliated with, endorsed by or sponsored by
Enterbrain, KADOKAWA or Gotcha Gotcha Games. RPG Maker is a trademark of its owner. The
repository contains no code, script or asset of the original runtime or of the run-time
package; to run a game you need the game and the files it requires."

### Other documents

- `THIRD-PARTY.md`: Ruby 1.8.7-p374 (`third_party/ruby-1.8.7`, the Ruby licence or GPL
  version 2, exceptions in its `LEGAL`; texts `COPYING`, `GPL`, `LGPL`, `LEGAL`) and
  Oniguruma 5.9.6 (`third_party/oniguruma-5.9.6`, BSD, text `COPYING`), what the paragraph
  "Third-party source" of the README says today; for the Rust crates a pointer to
  `Cargo.lock`, `deny.toml` and `cargo xtask audit`.
- `CHANGELOG.md` in the form of Keep a Changelog: one entry `0.1.0`, undated (the owner
  dates it at the release), a list of what the first release contains, at most 25 lines.
- `CONTRIBUTING.md`: build and gate, the OpenSpec workflow in five lines, the rule that no
  original DLL, RTP asset or Enterbrain source is ever committed and tests use generated
  images, how to report a game that misbehaves.
- `docs/images/demo.png`: frame 10 of
  `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo run -p vkxp-host --example demo-headless -- --frames 30 --dump 10=docs/images/demo.png`;
  if that frame is a plain picture, a frame and an `--input` that show more are chosen and
  the command is stated in `docs/running.md`.

### Agent files

`AGENTS.md` keeps "Rules" (without the line about Chinese) and "Scope and proportion" and
gains two lines: reference material of the original runtime lives outside the repository and
is never committed; the Vulkan tests need the loader restriction. The sections "Reference
material" and "Working inside an agent flow" are removed (the owner keeps them outside the
repository). `CLAUDE.md` is the single line `See [AGENTS.md](AGENTS.md).`. `.claude/` and
`.agents/` are removed from the index with `git rm -r --cached` (the files stay on disk) and
added to `.gitignore`.

### Link check

`xtask/tests/doc_links.rs`: for `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`,
`THIRD-PARTY.md`, `AGENTS.md` and every `docs/*.md`, every Markdown link or image whose
target is not a URL and not a pure `#fragment` must name an existing file or directory,
relative to the document (a fragment after the path is ignored). No new dependency: a small
scan for `](`.

## Risks

- A sentence may be lost in the move. The reviewer compares the section headings of the old
  README with those of the new files.

## Follow-ups

None yet.
