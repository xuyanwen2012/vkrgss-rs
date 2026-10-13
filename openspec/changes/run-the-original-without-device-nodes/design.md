## Context

`Original::wine` in `xtask/src/oracle/original.rs` builds every Wine command of the run
(`wine`, `wineboot`, `wineserver`, `wine regedit`). The virtual display is an `Xvfb` that
xtask starts itself and reads the frames from.

Measured outside the repository on 2026-10-13: with `bwrap --dev-bind / / --tmpfs /dev/dri
--unshare-net --die-with-parent` around the script that runs Wine, no process of the run maps
a VA-API or GPU driver other than llvmpipe and none holds a file under `/dev/dri`; original
runtimes of XP's successor versions start, take input and give byte-identical captures in
repeated runs. An own process-id namespace (`--unshare-pid`) must not be used: an `Xvfb`
inside one takes the display lock files of other processes for stale.

## Goals / Non-Goals

Goals: no process of the original side can open a device node under `/dev/dri` or reach the
network.

Non-goals: a sandbox for the vkxp side (it has its own rules and its own Landlock sandbox);
confining what Wine may write; any change to what is compared.

## Decisions

- The sandbox is `bwrap`, called as a program, around each Wine command: `bwrap --dev-bind
  / / --tmpfs /dev/dri --unshare-net --die-with-parent -- <program> <arguments>`. The
  environment of the command is set as today, on the `bwrap` process, which hands it on.
- `Xvfb` stays outside the sandbox, started by xtask as today. Wine reaches it through the
  socket file under `/tmp/.X11-unix`, which a network namespace does not hide (only the
  abstract socket is hidden). If the first run after the merge shows that Wine does not reach
  the display this way, `--unshare-net` is dropped and that is recorded here; the device
  directory is the requirement, the network is the lesser one.
- No network: the games are downloads, and one game of the corpus of VX Ace carries a script
  that talks HTTP. A game that needs the network on the original side is not compared.
- `LIBVA_DRIVERS_PATH` is set to a directory that does not exist and `LIBVA_DRIVER_NAME` to
  a name no driver has, so that VA-API finds no driver even where the sandbox would fail.
- `bwrap` is checked with the other programs; no fallback runs Wine without it.

## Implementation notes

- A unit test holds the argument list of a built command: `bwrap` first, the four options,
  then the program. The test that needs Wine stays ignored, as today.
- Flows cannot run Wine. The run on a real game, with a look at `/proc/<pid>/maps` and
  `wine.log`, is done after the merge.
