# Design

## Decisions

- The evidence is the run of 2026-10-16 described in the proposal; the scripts that ran it
  (`wt/xarch.sh`, `wt/xarch-compare.py`) and the outputs stay outside the repository, like
  the oracle's. The games, frames and schedules are those of `ref/tools/oracle/regress.sh`,
  the V1-V3 route and the A1-A3 runs.
- What P1 does not cover, written in the row: the comparison ran on llvmpipe of aarch64, not
  on NVIDIA's driver of the Jetson (the frames are composed by the same compute shader; the
  Vulkan tests passed on both); audio and a windowed run were not checked on aarch64; glyphs
  are those of the same font file on both sides.

## Follow-ups

None yet.
