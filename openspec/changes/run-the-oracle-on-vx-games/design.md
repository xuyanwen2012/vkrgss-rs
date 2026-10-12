## Context

The facts below were measured on the original of VX on 2026-10-12 and are written up in
`ref/tools/oracle/measurements/vx-v0-REPORT.md` (outside the repository, readable from a
clone as `../../ref/...`), with the scripts that were used (`vx-v0-probe.rb`,
`vx-v0-expand.rb`, `vx-v0-oracle.sh`, `vx-v0-prefix.sh`). Its section "What in
`xtask/src/oracle` is XP-specific" lists the places. In short:

- Window 544 by 416. Run-time package: `HKLM\Software\Enterbrain\RGSS2\RTP`, 32-bit view,
  value named as `RTP=` of `Game.ini` names it (`RPGVX`). The `d3d8.dll` stand-in, the Wine
  environment and the locale option are as for XP.
- `Data/Scripts.rvdata` has the structure of `Scripts.rxdata`; `Game.rgss2a` is the format
  of `Game.rgssad`; with the archive present the loose script file is not used.
- Frames, by the change of `Graphics.frame_count`: `update` 1; `wait(n)`, `fadeout(n)`,
  `fadein(n)` n; `freeze` 0; `transition(n)` n after a freeze and 0 without one, default
  n 10; `snap_to_bitmap`, `frame_reset`, `resize_screen` 0. The game may assign
  `Graphics.frame_count`, so a probe keeps its own count.
- `wait`, `fadeout` and `fadein` do not call the Ruby-level `Graphics.update`. Written as
  loops of `update` by the rules of the report (brightness `b0 - b0 * k / n` and
  `b0 + (255 - b0) * k / n` for frame k), they show the same pictures frame for frame
  (331 frames compared).
- After a stall, such as a hold, the original returns from `Graphics.update` without
  drawing for up to five updates. `Graphics.frame_reset` before the update of a captured
  frame and after every hold makes every update draw.
- The frames inside a `transition` cannot be held; the first picture that can be captured
  is the one the call ends with.
- `Input.repeat?` with one key held is true at the 1st, 24th, 30th, 36th update (XP: 1st,
  16th, 20th, 24th). The button numbers are those of XP.
- RGSS2 does not seed the random numbers at start; `srand` in the probe makes runs equal.
- `Thread.new` ends the original under this Wine (XP's as well). A game of the corpus
  creates a thread at its first victory. Running the block at once in the caller, and
  skipping `sleep` inside it, let the game go on.

Today (`xtask/src/oracle`): `report.rs` has the size as constants and `original.rs` looks
for a window of that size and writes the registry key of XP; `game.rs` uses
`rgssad::FILE_NAME`; `probe.rb` is one script for both sides whose text must be the same on
both, because the names it interns shift the symbol numbers of the game.

## Goals / Non-Goals

**Goals:** `cargo xtask oracle` drives the original of VX as V0 did by hand, with the same
exactness, and compares with vkxp once vkxp runs VX.

**Non-Goals:** a window whose size changes during the run (`Graphics.resize_screen`, a
game's own resizer); capture through `Graphics.snap_to_bitmap`; real key presses instead of
the probe's input model; audio; anything in vkxp itself.

## Decisions

- Runtime of the game: from the extension of the file that `Scripts=` names (`.rxdata`
  RGSS1, `.rvdata` RGSS2), or from `--runtime rgss1|rgss2` when given. Another extension
  without the option ends the run with a message naming the option.
- By runtime: the frame size (640 by 480, 544 by 416) in capture, window search and report;
  the registry key (`RGSS\RTP`, `RGSS2\RTP`) and the names taken from `RTP1` to `RTP3` or
  from `RTP`; the archive name; the probe file (`probe.rb`, `probe_rgss2.rb`).
- When the two sides give pictures of different sizes, the report says so for that frame
  and gives no pixel counts; the run does not fail.
- The probe of RGSS2 is one text for both sides, like that of XP. It:
  - seeds with the fixed seed and replaces `Input` by the schedule model, with the repeat
    rule of RGSS2 (24, then every 6);
  - keeps its own frame number, advanced by the measured rule of each call;
  - on the original's side replaces `wait`, `fadeout` and `fadein` by the loops of the
    report, so that a frame inside them can be held, and calls `Graphics.frame_reset`
    before the update of a captured frame and after each hold; on the vkxp side it leaves
    those functions alone, so that vkxp's own are what is compared;
  - on the original's side, when a captured frame number falls inside a transition, holds
    after the call and writes the number down as late.
  How the probe tells the sides apart follows what `probe.rb` does today.
- A late frame: its picture of the original is kept, the report lists it as "not
  comparable: inside a transition" and it counts neither as equal nor as different.
- `--original-only`: only the original runs; the report lists the pictures without
  comparison. It is the counterpart of `--vkxp-only` and is valid for XP as well.
- `--no-threads`: the probe, on both sides, makes `Thread.new` run its block at once in the
  caller and makes `sleep` return at once inside such a block. Without the option nothing
  about threads changes. The report states that the option was used.
- Wine is only ever started with the environment `original.rs` builds today (own prefix,
  software rendering, `VK_LOADER_DRIVERS_SELECT=*lvp*`). No step of this change runs a
  `wine` or `wineserver` command outside that code, in tests or by hand: a bare `wine`
  command loads every Vulkan driver of the machine, which broke the GPU rule once.

## Risks / Trade-offs

- The loops that stand in for `wait` and the fades on the original's side are the
  measured rules, not the original's code. They were compared on grey levels only. A
  difference would show as a mismatch against vkxp's native functions and would have to be
  told apart by a run without captures inside them.
- `--no-threads` changes the game. It is the same on both sides, and the report says so.

## Follow-ups

- Windows that change size; `snap_to_bitmap` as a second capture that checks the grab.
- Whether another Wine build runs Ruby threads.
- Mask transitions of RGSS2 were recorded, not fitted.
