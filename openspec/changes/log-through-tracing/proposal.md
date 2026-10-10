# Proposal

## Why

`docs/decisions.md` names `tracing` for logs, and nothing uses it. Diagnostics are about
twenty bare `eprintln!` calls in `vkxp`, `vkxp-host` and `vkxp-audio`, plus three private
report helpers that write to an injected stream. There are no levels: what the program says
cannot be turned up to find out which device, pacing mode, RTP directory or font was chosen,
and the present statistics and pacing reports share one stream with errors.

## What Changes

- Diagnostics that the running program writes become `tracing` events with a level. The
  executable installs one subscriber that writes them to the error stream.
- By default warnings and errors are shown, as lines `vkxp: <text>`, so what a user sees today
  stays as it is. `--verbose` adds informational and debug events; the environment variable
  `VKXP_LOG` sets a level or a filter.
- A small number of informational events are added at the points where the program makes a
  choice a user may need to see: the GPU and the pacing mode, the RTP directories, the
  built-in scripts file, the archive, the configuration files read.

Out of scope: `vkxp-vk`, `vkxp-core` and the other guest crates below `vkxp-binding` (no
events are added there in this change); spans and timings (a later change); the text the game
itself produces (`p`, `print`, the script error report) and the program's usage and fatal
error message, which stay direct writes; a log file; a configuration-file key.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: a requirement on diagnostics and the two ways to set their level.

## Impact

- `crates/vkxp`, `crates/vkxp-host`, `crates/vkxp-audio`; `crates/vkxp-binding` only where it
  reports a diagnostic of its own (a stubbed process function, an unknown `Win32API` call).
- New dependencies: `tracing` in those crates, `tracing-subscriber` in `vkxp` only.
- `README.md`: the options table and a short paragraph on diagnostics.
