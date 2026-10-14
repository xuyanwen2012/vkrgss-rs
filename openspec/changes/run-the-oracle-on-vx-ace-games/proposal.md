## Why

A milestone of VX Ace (A1 title and menu, A2 map, A3 battle, saving and loading) is reached
only with a record of a comparison with the original runtime (`docs/decisions.md`, "VX Ace"
and "Order and milestones"). For XP and VX that comparison is `cargo xtask oracle`. Milestone
A0 showed, with scripts kept outside the repository, that the original of VX Ace can be driven
and captured exactly under Wine, and what differs from VX. `cargo xtask oracle` knows none of
it: it does not recognise `Scripts.rvdata2`, cannot read or write `Game.rgss3a` (RGSSAD
version 3, another format than that of XP and VX), registers the run-time package under the
key of XP or VX, copies the DLL beside `Game.exe` although a game of VX Ace loads it from the
path `Library=` names, looks for a window of a fixed size although such a game may resize its
screen at once, and lets GStreamer load its hardware plugins, the path through which the
first run of a VX Ace game opened a GPU of the machine. It cannot run a VX Ace game on either
side.

## What Changes

- The harness recognises RGSS3 (`.rvdata2`, or `--runtime rgss3`) and follows it: frame size
  at the start, registry key `RGSS3\RTP`, the archive `Game.rgss3a`, the probe.
- A reader and writer of RGSSAD version 3 in `xtask`, which keeps the seed, the keys and the
  layout of an archive, so that only the entry of the scripts changes when the probe is put
  into it.
- A third probe, for RGSS3, in the syntax of Ruby 1.9: frames counted by the rules of RGSS2
  (measured the same), `Input` answered from the schedule with buttons as Symbols or as the
  Integers of RGSS2, and on the original's side the size of the screen written with each
  request to hold.
- On the original's side: the DLL goes to the path `Library=` names (and `--dll` may be
  left out for such a game, which ships its DLL); the window is captured at the size the
  probe reports; GStreamer gets plugin directories without its hardware plugins and a
  registry of its own, for every runtime.
- On the vkxp side: the guest executable of RGSS3 is built with `vkxp`, which starts it.
- XP and VX run as before, except that the GStreamer environment applies to them as well.
- Not in this change: comparing a real VX Ace game (that needs the class bindings of RGSS3,
  which are changes of their own) and recording evidence of a milestone. The first run on a
  real game is made after the merge, outside the flow.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oracle-harness`: the runtime RGSS3; archives of RGSSAD version 3; the input of the probe
  of RGSS3; the size of a captured frame; frames inside a transition of RGSS3; the DLL of a
  game of RGSS3; the vkxp side of such a game; GStreamer without hardware plugins.

## Impact

- `xtask/src/oracle/` (`runtime.rs`, `game.rs`, `probe.rs`, the new `probe_rgss3.rb`,
  `original.rs`, `vkxp.rs`, `mod.rs` with its usage text and tests), `xtask/src/rgssad.rs`
  and a new module beside it for version 3; `docs/development.md` (the section on the
  oracle).
- Not touched: every crate under `crates/`, `docs/decisions.md` and the milestone table.
  The harness reads `Game.ini` and the archives itself, as it does today; it does not
  depend on the game crates.
- Depends on `run-rgss3-games-in-a-guest-of-their-own` (the executable
  `vkxp-guest-rgss3`, which `vkxp` starts for a game of RGSS3). Until the class bindings of
  RGSS3 are merged, the vkxp side of a real VX Ace game ends with the error of a missing
  class behaviour; the harness reports that like any failure of that side, and
  `--original-only` runs the original alone.
