# Design

## Context

- The original runs under Wine 11 in Xvfb on the development machine. The working recipe is the
  shell script `ref/tools/oracle/run-original.sh` outside the repository (a copy is in this
  clone's `local/`): a Wine prefix of its own; the registry value
  `HKLM\Software\Enterbrain\RGSS\RTP` (32-bit view) naming the RTP directory; a stand-in file
  `syswow64/d3d8.dll`, because the runtime only checks that `system32\D3D8.DLL` exists;
  `LC_ALL=zh_CN.UTF-8` so that Chinese file names resolve through the ANSI file functions;
  software rendering (`LIBGL_ALWAYS_SOFTWARE=1`, `VK_LOADER_DRIVERS_SELECT='*lvp*'`), because
  the Intel GPU of that machine must never be opened.
- Without a window manager the game window is an undecorated 640 by 480 top-level window.
- In a first screenshot the message window's text was missing: the prefix has no font for the
  game's font name.
- `vkxp --headless --frames N --input ... --dump FRAME=FILE` exists. Host frame numbers count
  every frame message, and `Graphics.transition(d)` sends `d` of them.
- The original has no frame dump and no input injection; `Input` lives in the DLL, but its
  module functions can be redefined from Ruby in both runtimes.
- The script archive is a Ruby `Marshal` array of `[Integer, String, String]` whose third
  element is zlib-deflated source.

## Goals / Non-Goals

**Goals:** one command from a game directory to a comparison report; exact frame
correspondence; nothing proprietary in the repository.

**Non-Goals:** closing differences; a threshold that fails the gate.

## Decisions

### The probe

One Ruby file, kept in `xtask` as a template, with the schedule, the capture frames and the
side (`original` or `vkxp`) filled in. It:

- calls `srand` with a fixed seed;
- wraps `Graphics.update` to count frames, and `Graphics.transition` so that the count stays
  equal to vkxp's host frame number (on the original side the count advances by the
  transition's duration);
- redefines the five `Input` queries from the schedule, keeping a held-frame counter per
  button (trigger at 1, repeat at 1 and from 16 every 4 frames, as `core-input` states);
  `Input.update` advances the counters;
- on the original side, after a capture frame: writes `capture-<n>.request` into the game copy
  and sleeps in a loop until the harness has removed it; after the last capture frame it exits;
- on the vkxp side captures are `--dump` options and the run ends by `--frames`.

### Capturing the original

The harness starts `Xvfb` on a free display itself (no `xvfb-run`), starts Wine with the
environment of the recipe, and polls for request files. For a request it connects with `x11rb`,
finds the 640 by 480 top-level window, reads it with `GetImage`, writes the PNG and removes the
request. A timeout ends a run that never reaches a capture frame; the Wine server is always
shut down at the end.

The prefix is created once below the output directory. Font replacements for common game font
names are registered in the prefix so that text is drawn (PLACEHOLDER list; text is compared
for layout only).

### Archive reader and writer

A small module in `xtask` that reads and writes exactly the `Marshal` shapes the archive uses
(array, fixnum, string with and without instance variables on reading) and uses `flate2`.

### Report

`<out>/frames/<n>-original.png`, `<n>-vkxp.png`, `<n>-diff.png` (absolute difference,
amplified) and `<out>/summary.json` plus a printed table.

### Tests

Unit tests: archive round trip, probe generation, schedule semantics by running the probe in
vkxp on a generated game (the `vkxp` executable's test support is the model), comparison
arithmetic. The original-side scenario is an ignored test documented in the README, run when
`VKXP_ORACLE_DLL`, `VKXP_ORACLE_RTP` are set and Wine and Xvfb exist. If the flow's sandbox
cannot start Xvfb or Wine, implement the original side against the recipe, say so in the
final report and leave that scenario's verification to the coordinator.

## Risks / Trade-offs

- [Redefining `Input` hides differences in the real input path] → Accepted: the harness compares
  rendering and script behaviour; input has unit tests.
- [A game that calls `Input` before the first frame or uses `Win32API` keys] → Not covered.

## Follow-ups

- Exact text comparison needs the original fonts in the prefix.
- The original side could not be run in the implementing flow's sandbox, which denies writing
  to `/tmp`: `Xvfb` cannot create its socket in `/tmp/.X11-unix` and `wineserver` cannot
  create `/tmp/.wine-<uid>`. It is implemented against the recipe (prefix, `reg add` with
  `/reg:32`, `d3d8.dll` stand-in, locale, software rendering), and the capture-in-the-original
  scenario is left to the coordinator: the ignored test
  `the_original_is_captured_at_the_frames_asked_for`, as the README describes.
- That test needs a third variable, `VKXP_ORACLE_GAME`: a generated game has no `Game.exe`,
  which is part of the original runtime and cannot be committed or generated.
- The X socket and the Wine server directory are in `/tmp`, where `Xvfb` and Wine put them;
  neither can be moved below the output directory. Should they be isolated (a private `/tmp`)?
- On the original side a capture frame inside a transition is taken when the transition has
  ended, because the probe cannot hold the game in the middle of one. Is a probe-side
  re-implementation of the transition wanted?
- The DLL is copied into the game copy under its own file name, as the recipe does. Should it
  take the name that `Library=` gives, so that any 1.0x DLL can stand in?
- The 200 ms wait before a window capture and the font replacement table are unverified
  guesses until the original side has run.
