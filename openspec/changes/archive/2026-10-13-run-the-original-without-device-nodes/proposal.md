## Why

`cargo xtask oracle` runs the original runtime under Wine with `LIBGL_ALWAYS_SOFTWARE=1` and
`VK_LOADER_DRIVERS_SELECT=*lvp*`, so that Wine opens no GPU of the machine. That is not
enough. A game that plays Ogg or MP3 does it through quartz, `winegstreamer` and GStreamer,
whose hardware plugins probe VA-API: a `wine.log` of 2026-10-12 (The Witch's House) and a run
of a VX Ace game on 2026-10-13 show `libva` opening the video driver of a GPU. On a machine
where one GPU is reserved for other work, the comparison disturbs it; anywhere, the original
side depends on hardware it was meant not to touch.

## What Changes

- Every Wine process of the original side runs inside a `bwrap` sandbox in which `/dev/dri`
  is an empty directory and there is no network. No library of the run can open a GPU device
  node.
- `bwrap` joins the programs the original side needs; without it the command says so, as
  for Wine and Xvfb.
- VA-API is also pointed at a driver directory that does not exist, as a second line.
- `docs/development.md` says it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oracle-harness`: the original side has no GPU device node and no network; `bwrap` is a
  prerequisite of it.

## Impact

`xtask/src/oracle/original.rs` (and `mod.rs` where the prerequisites are named),
`docs/development.md`. The frames of the original do not change: it rendered in software
before. The vkxp side is untouched.
