# Proposal

## Why

`--frame-stats` has one row for the guest: the time the host waits between frames. It cannot
tell the game's scripts from bitmap operations, text drawing or frame extraction, and in a
headless run the guest is often ahead of the host, so the row understates what the guest
spends. A decision on whether any guest-side work deserves a faster path needs the split.

## What Changes

- With `--frame-stats` the guest measures its own time per frame and splits it into: bitmap
  operations, text drawing, frame extraction, and the rest (scripts). The rows appear in the
  same summary, at the same moments.

Out of scope: making anything faster; a profiler; a protocol change (the guest writes its
own rows); timing when the option is off.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-session`: the statistics have rows for the guest's parts.

## Impact

- `crates/vkxp-binding` (the measuring and the rows), `crates/vkxp` (passing the option to
  the guest), `crates/vkxp-host` only if the summary's moment has to be signalled, `README.md`.
