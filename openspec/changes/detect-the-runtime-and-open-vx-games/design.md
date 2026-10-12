## Context

- `GameIni` (`crates/vkxp-vfs/src/ini.rs`) returns `library`, `scripts`, `title` and the
  three names `RTP1` to `RTP3`. `resolve_rtps` maps the names to directories.
  `GameFiles` looks for `Game.rgssad` (`files.rs`).
- The program (`crates/vkxp/src/main.rs`) reads the configuration file, resolves the
  packages, finds the built-in scripts file (`--builtin-scripts`, default
  `$XDG_DATA_HOME/vkxp/rpg_builtin.rb`) and starts the guest. `RunConfig`
  (`crates/vkxp-binding/src/runtime.rs`) carries the configuration file and the debug and
  battle-test switches; the binding sets `$DEBUG` and `$BTEST`.
- Measured on the original runtime of VX on 2026-10-12 (reports
  `ref/tools/oracle/measurements/vx-v0-REPORT.md` and `vx-v1-REPORT.md`, outside the
  repository):
  - `Game.ini` of VX has one key `RTP=` (value `RPGVX` for the run-time package; both
    games of the corpus leave it empty).
  - `Game.rgss2a` is RGSSAD version 1, the format of `Game.rgssad`. With the archive
    present a loose `Data/Scripts.rvdata` is not used.
  - `Game.exe` without arguments: `$TEST`, `$BTEST` and `$DEBUG` are all `false`. With the
    argument `test`: `$TEST` is `true`, the other two `false`. With `btest`: `$TEST` and
    `$BTEST` are `true`, `$DEBUG` is `false`.
  - `$KCODE` is `"UTF8"`.
  - The built-in scripts of RGSS2 are one block that starts with `module RPG;` and defines
    the `RPG` data classes only. No VX DLL holds it as plain text; it was read from the
    memory of the running original.

## Goals / Non-Goals

**Goals:** the runtime of a game is known before anything else is decided; what differs
between XP and VX in starting a game follows it; XP is unchanged.

**Non-Goals:** anything a VX game needs after its first script starts (the frame size of
544 by 416, 60 frames per second, the functions of RGSS2); the symbol numbers of RGSS2; the
archive format of VX Ace; a configuration-file key for the runtime.

## Decisions

- The runtime is an enum `Runtime { Rgss1, Rgss2, Rgss3 }` in `vkxp-vfs`, with the names
  `rgss1`, `rgss2`, `rgss3` in options and messages and "RPG Maker XP", "RPG Maker VX",
  "RPG Maker VX Ace" for people. It lives in `vkxp-vfs` because detection reads the
  configuration file and the binding already depends on that crate.
- Detection, as decided by the owner: `Library=` is recognised when its file name (the part
  after the last `\` or `/`, letter case ignored) is `RGSS` followed by a digit 1, 2 or 3,
  two more digits, an optional letter and `.dll`; the first digit is the runtime.
  `Scripts=` is recognised by its extension `.rxdata`, `.rvdata` or `.rvdata2`. A value
  that is not recognised says nothing. One source alone decides; two that agree decide;
  two that disagree, or none, are an error that names both values and the option
  `--runtime`. No other file is looked at.
- `--runtime rgss1|rgss2|rgss3` skips detection entirely.
- `GameIni` gains the value `rtp` of the key `RTP`. The names of the run-time packages of a
  game are `RTP1` to `RTP3` for RGSS1 and the one name `RTP` for RGSS2; an empty name is
  skipped as before. `--rtp NAME=DIRECTORY` is unchanged (`--rtp RPGVX=<directory>`).
- The archive is `Game.rgssad` for RGSS1 and `Game.rgss2a` for RGSS2, the same reader. The
  archive of the other runtime is ignored if present.
- RGSS3: the program ends before the guest starts, with a message that names the runtime
  and says that VX Ace is planned and not supported. Nothing of its archive is read.
- The built-in scripts file: the option names it for the run's runtime; the default is
  `rpg_builtin.rb` for RGSS1 as before and `rpg_builtin_rgss2.rb` for RGSS2, both under
  `$XDG_DATA_HOME/vkxp/`. The message for a missing file names the file and, for RGSS2,
  the section of `docs/running.md` that says how it is made.
- `docs/running.md` says how the file of RGSS2 is made today: `cargo xtask extract-rpg`
  works on the image of the DLL as it lies in the memory of the running original, not on
  the DLL file, which is packed. A tool that does this for a user is not part of this
  change.
- Test mode: `--debug` sets `$DEBUG` for RGSS1 and `$TEST` for RGSS2; battle test sets
  `$BTEST` for both and, for RGSS2, `$TEST` as well. In RGSS2 `$DEBUG` stays `false`.
- A run of RGSS2 prints once, through the program's diagnostics at the level of a warning,
  that support for RPG Maker VX is experimental.
- The guest of a VX game is otherwise the guest of XP for now: the same prelude and
  classes. The game's scripts stop at the first thing that RGSS2 has and RGSS1 has not.
  That is what "run as far as the code goes" means until the next changes.

## Risks / Trade-offs

- A game with a renamed DLL and an unusual script file name is not detected. The message
  names the option.
- XP games whose `Library=` is empty are detected by `Scripts=` alone, as the tests'
  generated games are.

## Follow-ups

- The frame size and rate of RGSS2, `Graphics.wait` and the other functions, the symbol
  table chosen from the runtime at the start of the interpreter (the program passes it once
  `number-symbols-by-runtime` is merged).
- A way for a user to make the built-in scripts file of RGSS2 without the steps of
  `docs/running.md`.
- A key `runtime` in the configuration files.
