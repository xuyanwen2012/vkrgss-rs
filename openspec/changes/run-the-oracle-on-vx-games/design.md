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

## Notes on the implementation

- The directories of the run-time packages are written into the prefix with its registry
  script, the one that already held the fonts (now `vkxp-oracle.reg`), instead of one
  `reg add` per package. The script names the 32-bit view itself: `Software\Wow6432Node\...`
  when the prefix has a `syswow64` directory, `Software\...` otherwise.
- The values under the key have the names given with `--rtp`, as before. The harness does not
  read `RTP1` to `RTP3` or `RTP` from `Game.ini`; the runtime only decides the key.
- `summary.json` of a comparison has `totals` (equal, different, not comparable), so that a
  frame that is not comparable has something to stay out of. A frame whose two pictures
  differ in size is listed with `not_comparable` like a late frame, with the sizes as reason.
- The probe's record of a late frame is the request file of that frame: it holds the frame
  number followed by the word `late`, and the harness reads it before removing the file.
- The last frame of a transition is not late: the picture the transition ends with is the
  picture of that frame, so it is captured and compared like any other.
- The scenarios stated on the model of the probe of RGSS2 are tested by running that probe
  in vkxp, in a generated game of RGSS1 whose first script defines the functions of
  `Graphics` that only RGSS2 has; the side of the original is tested the same way, with the
  test answering the requests in the place of the harness.

## Measured

Task 3.1, 2026-10-12, Wine 11.0 Staging, through `cargo xtask oracle` only: Visions & Voices
with the DLL it ships (2.0.0.1 under the name `RGSS202E.dll`), `--original-only --no-threads
--rtp RPGVX=<run-time package> --input @vx-v0-vv-input.txt --locale en_US.UTF-8 --timeout 300`
and the nine frames, twice into the same output directory with the pictures copied aside
after each run. Both runs ended with status 0, and the nine pictures of the second run equal
those of the first byte for byte. No frame was late. Frame 260 lies inside the `fadeout(60)`
of the title and was captured. While the game ran, its Wine processes had mapped
`libvulkan_lvp.so` and no other Vulkan driver.

The files of V0 are the bytes b, g, r, x of the window, so the third column is the SHA-256
of a picture written that way (x is 255), next to that of the PNG file the harness wrote.

| frame | SHA-256 of the PNG file | SHA-256 as b g r x bytes | equals `vx-v0-vv-sha256.txt` |
|---|---|---|---|
| 100 | `24f2635c7ba740abed6266c3c136897d2411ab911b0e880b176d9adff4abb61f` | `46adcb8a305c8eb736892fffd3e30ff7e219ee200b6068c5435d014c5c70c048` | both lists |
| 260 | `32391230c883d8fcdfba4324e4a350f11600d14a9bbce01d1a8c236742f7b4be` | `57c7555060c95e480bac4c40c6bd4e874c87e037d5c3a440c633cb0e5ec417f9` | `vv-final-a` |
| 1000 | `0d1e6da9aa931d5c94a6e51fb5bb44061f3965bcd632a7fcf271581105204636` | `1d86771106095ea6db35cf02230e0863e7597fdcb6c8bc607137a8452a9af68c` | both lists |
| 3500 | `2a19ba5806ab87ea6c875c241680c1a47867c100731407362fdbb71a4efbdf99` | `8fc28d607efc91553faa42ea2cbff199b94efdc4e50c032aa572014b173605f4` | `vv-run-a` |
| 6600 | `6df7a477ee5f2045cc1c985831a2e736cd3c3c62c1e08e7a23f0a046b28c0213` | `3607e6cac9bf51ccb3f5ad6f0da2792db5bca9f3b06dae327516f33dc839a7d1` | `vv-run-a` |
| 11300 | `fdf591df2ee1aa2febac0dc626ff59b61c6286b5825faf2db4125ca42f3b086d` | `88309dfe14cf07ba78de1eb1366d428e4aba797711dc20aa60dced0f0d9a86ab` | `vv-run-a` |
| 11580 | `dc8d08654ffafa4f3bff15d95de7cfee52613830edb7df2978b17a42d3957651` | `6a057ddd1923508c09533e20a4f0caaf5d03cfa5ed97a3e3d96f79f4ba37d056` | `vv-run-a` |
| 11700 | `2a15f2c40dbe0664308e6fc5b7f1cfbd6ee7fe4f0b2214d54cdf212195daa590` | `85bdbf840155d34e8e3bd277ce2e7ea90433533df5e7a22a4de03bb74f1c6d49` | `vv-run-a` |
| 11900 | `b7739a1113b3fef78e4e205f4f5646dd694ab5369e5e93bbf54ed1d4bf1fdd22` | `fbb1fff08a80ef49b58d0c4c788fdee563d885d1e7675e9fc1cb3d606c8d90fa` | `vv-run-a` |

All nine equal a picture of V0. The eight frames that `vv-run-a` has (seed 20261009, the seed
of the harness, native `wait` and fades, real key presses) are equal to it, although the
harness answers `Input` from its model and runs `wait` and the fades as loops. Frame 260 is
not in `vv-run-a` and equals `vv-final-a` (seed 1, loops), as do frames 100 and 1000. From
frame 3500 on the pictures differ from `vv-final-a`; the reason is its seed, as V0 found
between its own runs (every frame from 2800 on differs between the two seeds).

Task 3.2, same day: `../../games/群战示范工程` with `RGSS103J.dll`, `--rtp Standard=<RTP>`,
frames 10, 60 and 200 and no input, once with the `xtask` of the commit before this change
(69648b3) and once with this change, each with a prefix of its own. The three pictures of the
original are equal byte for byte in both:

| frame | SHA-256 of the PNG file |
|---|---|
| 10 | `4efd73cfeb6c8094769ffcb91430cd955c079a37a26b6f77e135f7e1e376aa3f` |
| 60 | `de475792415f687e01d872e0cc12dbc6c324c341e2ee14a00dcf3d467ed469f0` |
| 200 | `adcde666313d37047c434633089457a6d6902ec0190b2bfc9d68a4cd5947326f` |

The registry of the new prefix holds `Standard` under
`Software\Wow6432Node\Enterbrain\RGSS\RTP`, and that of the prefix of task 3.1 holds `RPGVX`
under `...\RGSS2\RTP`, so the registry script reaches the 32-bit view.

## Follow-ups

- Windows that change size; `snap_to_bitmap` as a second capture that checks the grab.
- Whether another Wine build runs Ruby threads.
- Mask transitions of RGSS2 were recorded, not fitted.
