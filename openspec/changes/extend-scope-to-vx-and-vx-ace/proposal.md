# Extend the scope to RPG Maker VX and VX Ace

## Why

`docs/decisions.md` says that vkxp is a runtime for RPG Maker XP and that RGSS2 and RGSS3 are
out of scope. With the milestones of XP closed and version 0.1.0 released, the owner decided
on 2026-10-12 to extend the project to the whole RGSS family. The three runtimes share most
of what vkxp has: the host side, the compositor, the audio, the files, the way rules are
measured on the original under Wine. What sets vkxp apart from the runtimes that already
play all three, pixel-identical frames and deterministic runs, is to hold for the two new
ones as well.

A settled decision is changed only by a change that says so. This is that change. It
records the decisions and changes no code.

## What Changes

- The scope becomes RGSS1, RGSS2 and RGSS3 (XP, VX, VX Ace). RPG Maker MV, MZ, 2000 and 2003
  stay out of scope.
- The fidelity bar of XP applies to VX and VX Ace.
- The order is VX, then VX Ace; VX has four milestones, V0 to V3, and its own release.
- The decisions on how several runtimes live in one program, on the regression corpus, and
  on what a user sees before VX is done are recorded.
- For VX Ace only the direction is recorded; its plan is made after V3.
- The public documents say what is supported today and what is planned.

## Capabilities

### New Capabilities

- `supported-runtimes`: which RPG Maker runtimes the project covers, to what standard, in
  what order, and how the documents state what works at a given time.

### Modified Capabilities

None.

## Impact

`docs/decisions.md`, `README.md`, `AGENTS.md` (its first sentence), `docs/compatibility.md`
(one sentence). No code, no test, no dependency. Later changes implement the milestones;
the first of them is the check that the original runtime of VX can serve as the oracle.
