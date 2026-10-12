## Why

Milestones V1 to V3 of VX are reached by comparing frames with the original runtime
(`docs/decisions.md`, "Order and milestones"). Milestone V0 showed, with scripts kept outside
the repository, that the original of VX can be driven and captured exactly under Wine, and
what has to differ from the recipe of XP. `cargo xtask oracle` still assumes XP in every one
of those places: a window of 640 by 480, the registry key of the run-time package of XP,
`Game.rgssad`, a probe that counts one frame per `Graphics.update` and the length of every
transition, and the repeat rule of XP's input. It cannot run a VX game on either side.

## What Changes

- The harness finds the runtime of the game (RGSS1 or RGSS2) and follows it: frame size,
  registry key and package name of the original, the archive, the probe.
- A probe for RGSS2 that numbers frames as the original does, keeps the original drawing
  when it is held, and lets frames inside `wait`, `fadeout` and `fadein` be captured.
- A frame asked for inside a transition of RGSS2 is reported as not comparable instead of
  compared.
- `--original-only`, so that the original's side can be run and repeated before vkxp runs
  VX games.
- `--no-threads`, for games whose Ruby threads end the original under Wine.
- XP runs as before.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oracle-harness`: the runtime of the game; the conditions and the capture by runtime; the
  two options.

## Impact

- `xtask/src/oracle/` and `xtask/src/rgssad.rs`; `docs/development.md` (the section on the
  oracle).
- Not touched: every crate under `crates/`. The harness reads `Scripts=` itself, as it
  reads archives itself; it does not depend on the game crates.
- The vkxp side of a VX game fails until vkxp runs VX games; the harness reports that
  failure like any other of that side.
