# Design

## Context

- `docs/decisions.md`, "Goal and scope": vkxp is a runtime for RPG Maker XP; RGSS2 and RGSS3
  are out of scope; the target behaviour is RGSS 1.04; `Library=` of `Game.ini` only selects
  the default font and the language of error messages.
- The three runtimes: RGSS1 (XP) has Ruby 1.8.1, 640 by 480 and 40 frames per second; RGSS2
  (VX) has the same Ruby, 544 by 416 and 60, another tile system, another window layout and
  more operations on bitmaps and sprites; RGSS3 (VX Ace) has Ruby 1.9.2, another archive
  format and video playback.
- On the development machine: `RGSS200E.dll`, `RGSS200J.dll`, `RGSS202E.dll` and
  `RGSS300.dll`, one small VX Ace project, no VX game and neither run-time package.
- The decisions below were taken by the owner on 2026-10-12 in three rounds of questions.

## Decisions

These are recorded in `docs/decisions.md` as written here.

### Scope and standard

- In scope: RGSS1, RGSS2, RGSS3. Out of scope: RPG Maker MV and MZ (JavaScript on a browser
  engine) and 2000 and 2003 (not Ruby; EasyRPG Player exists).
- VX and VX Ace are held to the standard of XP: compared frames are pixel-identical to the
  original runtime except for text glyphs, a save written at the same frame is identical
  byte for byte, and runs are deterministic.
- Rules of VX are measured on `RGSS202E.dll`; a game of the corpus is compared with the DLL
  it ships. A difference between minor versions is recorded and gets no branch of its own.

### Order and milestones

- VX first, then VX Ace. The open items of XP (text with the fonts of Windows, a long game
  played through by a person) go on beside this and neither waits for the other.
- Milestones of VX:
  - V0: the original runtime of VX runs under Wine, frames are captured at chosen frame
    numbers and keys are injected; the oracle harness compares a VX game. The owner's
    coordinating session does this directly, and no change that implements VX is drafted
    before it has passed.
  - V1: title screen and menu.
  - V2: walking on a map.
  - V3: battle, saving and loading.
- VX is done when every game of the corpus is identical to the original on its compared
  frames and in a save written at the same frame. Audio, configuration, the sandbox and the
  save states are those of the host and are not milestones.
- Version 0.2.0 is released when V3 is reached; it does not wait for VX Ace.
- VX Ace: after VX; an own fork of Ruby 1.9.2 is vendored and patched to the behaviour
  measured on the original, as was done with Ruby 1.8.7; its guest is an executable of its
  own, because two interpreters cannot be linked into one. Everything else about VX Ace is
  decided after V3.

### Several runtimes in one program

- One repository, and the name stays for now.
- One program. The RGSS version is a value known at run time; XP and VX share the embedded
  interpreter.
- The code of VX goes into the existing crates, told apart by a version value. Only the
  bindings of another interpreter get a crate of their own.
- The version is found from `Library=` in `Game.ini` and from the extension of the data
  files (`.rxdata`, `.rvdata`, `.rvdata2`); an option of the command line names it against
  that.
- The tile system of VX is an op kind that the CPU reference and the compute shader both
  evaluate from the start; no cache of composed maps comes first.
- The logical size of the frame and the frame rate are those of the game's runtime.

### Corpus

- Both run-time packages are fetched from their publisher's pages into the reference
  directory outside the repository.
- At least three games for each runtime, by different authors: the owner's own first, then
  well-known free games, whose list the owner sees before anything is fetched. A game is
  kept outside the repository and only a copy of it is run, in the confined guest process.
- What is built beyond the default scripts follows the corpus: DLLs that games ship,
  `Win32API` calls and tricks that change the resolution are done when a game of the corpus
  needs them.

### Before VX is done

- VX work is merged to `main` change by change. A VX game is run as far as the code goes,
  and the run says once that support for VX is experimental.
- Until V3 the documents say: XP is supported; VX is experimental or planned, whichever is
  true at that time; VX Ace is planned.

### Process

- As before: OpenSpec changes implemented by flows, at most three in parallel. The owner's
  standing go-ahead covers drafting, launching, merging and archiving up to V3 and does not
  cover VX Ace. Pushing to the public repository is asked for each time.

## Risks

- [The original runtime of VX does not run under Wine, or its frames cannot be captured
  exactly] → V0 comes first and alone; without it the standard above cannot be checked.
- [No VX game by the owner exists and free games are few] → the list is made and shown
  before V0; a thin corpus is said in the milestone table, as for XP.
- [Rules measured for XP are assumed for VX] → each is measured again on `RGSS202E.dll`
  before it is relied on; the probes exist.

## Follow-ups

None yet.
