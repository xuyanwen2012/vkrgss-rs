## Why

The milestones V1 to V3 of RPG Maker VX are done (`docs/decisions.md`, milestone table), and
the decisions say that version 0.2.0 is released when V3 is reached. Two things stand between
`main` and that release: the decisions ask for frame times of representative VX scenes before
0.2.0, and the program and its documents still call VX experimental.

The frame times were measured on 2026-10-13 and are far inside a frame of 16.7 ms except for
single frames in which a scene changes (numbers in `design.md`).

## What Changes

- A run of a VX game no longer says that support for VX is experimental; the usage message
  names `rgss2` without that word.
- The README, the agent guide, `docs/compatibility.md` and `docs/running.md` say that XP and
  VX are supported and that VX Ace is planned.
- `docs/decisions.md` records the measured frame times of VX beside the risk that asked for
  them.
- The version of the workspace is 0.2.0, and `CHANGELOG.md` has a section for it dated
  2026-10-13.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: the requirement "Experimental runtime" is removed.
- `supported-runtimes`: the recorded frame times of VX are required.

## Impact

`crates/vkxp` (`main.rs`, `args.rs`, `tests/end_to_end.rs`), `Cargo.toml`, `Cargo.lock`,
`README.md`, `AGENTS.md`, `CHANGELOG.md`, `docs/`. No protocol, frame or save changes.
