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
  it ships. A difference between minor versions is recorded and gets no branch of its own;
  where one shows in a comparison, V3 is reached only after the owner has accepted that
  difference explicitly. Recording it is not a passing comparison.
- Text glyphs are excluded as for XP: layout and advance widths are compared, glyph pixels
  are not. Where an operation of VX spreads glyph pixels (a snapshot of the screen, a
  blur), the comparison says how the affected area is left out.

### Order and milestones

- VX first, then VX Ace. The open items of XP (text with the fonts of Windows, a long game
  played through by a person) go on beside this and neither waits for the other.
- Milestones of VX:
  - V0: the original runtime of VX starts under Wine, takes a schedule of input, and gives
    the same captures at the same frame numbers in repeated runs; how waits and fades of VX
    count as frames is established. This is a proof of the original's side only, done by
    the owner's coordinating session directly; the oracle tool may be adapted for it (frame
    size, the probe script). No change that implements VX in the guest is drafted before it
    has passed. Comparing with vkxp begins at V1.
  - V1: title screen and menu.
  - V2: walking on a map.
  - V3: battle, saving and loading.
- A milestone is reached when the games of the corpus are identical to the original on
  compared frames that show that milestone's outcome, and for V3 also in a save written at
  the same frame. Before a milestone is declared reached, the record names the games and
  their revisions, the DLL of each, the seeds, the input schedules, the compared frames and
  the points at which saves were written, and that a repeated run gave the same.
- Audio, configuration, the sandbox and the save states get no milestones of their own.
  Their contracts apply to VX as they are, and are checked where the runtime behaves
  differently or the guest holds new kinds of resources (a state must bring them back; the
  music resumes differently after a music effect in VX). A measured difference in audio is
  implemented or recorded as an accepted limit.
- Version 0.2.0 is released when V3 is reached; it does not wait for VX Ace.
- VX Ace: after VX; an own fork of Ruby 1.9.2 is vendored and patched to the behaviour
  measured on the original, as was done with Ruby 1.8.7; its guest is an executable of its
  own, which keeps the two interpreters' symbols and global state apart. How it is
  packaged and chosen, what `--guest thread` means for it, and everything else about
  VX Ace is decided after V3.

### Several runtimes in one program

- One repository, and the name stays for now.
- One program. The RGSS version is a value known at run time; XP and VX share one build of
  the embedded interpreter. Where the interpreter itself behaves by runtime (the symbol
  numbers at start-up, which decide the order of instance variables in a save, are those
  of XP today), the runtime is chosen before the interpreter is initialised. What is
  measured for VX does not replace the behaviour of XP. The first change that implements VX
  says which recorded patches are shared and which are chosen by runtime.
- The requirements of the main specifications that state behaviour of XP (the default frame
  rate of 40, a viewport that disposes of its elements, `$DEBUG`) are contracts of RGSS1.
  A change that implements VX qualifies the requirements it touches by runtime and adds
  scenarios for RGSS2; the scenarios of XP stay. The requirements on the protocol, the
  boundary and the purity of the core apply to every runtime.
- The code of VX goes into the existing crates, told apart by a version value. Only the
  bindings of another interpreter get a crate of their own.
- The runtime of a game: an option of the command line decides when given. Otherwise it is
  taken from a recognised `Library=` of `Game.ini` and from the extension of the file that
  `Scripts=` names (`.rxdata`, `.rvdata`, `.rvdata2`), in a directory or in an archive
  alike; other files are not looked at. When the two disagree, or neither says anything,
  the run ends with a message that asks for the option.
- Starting a game is by runtime where it differs: the key `RTP=` of VX beside `RTP1` to
  `RTP3`, the archive `Game.rgss2a` beside `Game.rgssad` (the same encoding; the archive of
  VX Ace is another), and a file of built-in scripts for each runtime, extracted from the
  user's own DLL and never committed. These are settled before V1.
- The tile system of VX is an op kind that the CPU reference and the compute shader both
  evaluate from the start; no cache of composed maps comes first.
- The runtime gives the logical size of the frame and the frame rate at the start. The
  setters of RGSS itself (`Graphics.resize_screen`, `Graphics.frame_rate=`) change them
  within the limits of that runtime, and a reset restores the runtime's values. These are
  part of VX, not tricks.

### Corpus

- Both run-time packages are fetched from their publisher's pages into the reference
  directory outside the repository.
- At least three games for each runtime, by different authors: the owner's own first, then
  well-known free games, whose list the owner sees before anything is fetched. A game is
  kept outside the repository and only a copy of it is run: by vkxp in the confined guest
  process, by the original under Wine, whose prefix is not that sandbox.
- What is built beyond the default scripts follows the corpus: it decides which further
  `Win32API` calls are emulated and which patches are written. The limit that games which
  depend heavily on Windows may need patches stays. A DLL that a game ships and that would
  have to be executed, a script that reads or writes the memory of the runtime, and a
  change of resolution made past the functions of RGSS each get a decision of their own on
  whether and how.

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

- [A change to shared arithmetic or to the interpreter breaks XP] → a change that touches
  shared code names the probes and games of XP it was checked against; the seven-game
  comparison is run at its merge.
- [At 60 frames per second a frame has 16.7 ms, and waves, blur and snapshots are new
  costs] → frame times of representative VX scenes are measured before 0.2.0.
- [Ruby 1.9.2 differs in encodings, in what it serialises and in how it is embedded] →
  these are checked for feasibility when VX Ace is planned; sharing the rest of vkxp does
  not settle them.

## Follow-ups

- To be decided before the change that implements it is drafted: how
  `Graphics.snap_to_bitmap` of VX gets the pixels of the scene as it is, also while the
  display is frozen, without a frame going by. Composition happens on the host today and no
  message brings pixels back to the guest. The decision keeps the plain-data boundary and
  the purity of the core, and names the messages it adds or its use of the reference
  compositor.
