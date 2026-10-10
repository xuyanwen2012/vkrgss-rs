# Proposal

## Why

`docs/decisions.md` keeps the guest behind a plain-data boundary so that it can move into a
process of its own: that is what in-memory save states (a forked copy of the guest) and an
operating-system sandbox need, and it keeps a crash of the vendored interpreter from taking
the window with it. Today both sides are threads of one process and the link is a pair of
channels. This change adds the process form of the same link; save states are a later change
that builds on it.

## What Changes

- `vkxp-proto` gains a framing of the two message types for a byte stream, and a guest end
  of the link that works directly on a Unix stream socket.
- `vkxp` can run the guest in a child process: it starts itself again in a guest role, the
  child connects to a socket the parent listens on, and the parent relays between that
  connection and the unchanged in-process link of the runners.
- The choice is an option: `--guest thread|process`, the configuration key `guest`, and the
  environment variable `VKXP_GUEST`. The default stays `thread` in this change.
- What a run does and shows is the same in both forms.

Out of scope: save states; a sandbox for the guest process; shared memory for bitmap data;
platforms without Unix sockets; making `process` the default; the demo guest and the tests
of `vkxp-host`, which keep the in-process link.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: a stream framing and a stream transport of the guest end.
- `game-launch`: the guest option and what a run in a guest process does.

## Impact

- `crates/vkxp-proto` (`link.rs`, a framing module), `crates/vkxp` (arguments,
  configuration, a module for the guest process and the relay, `main.rs`),
  `crates/vkxp-binding` only where the guest's own reports (frame statistics, the outcome of
  the game) have to leave from the guest process, `README.md`, `docs/decisions.md` (the
  paragraph on the guest/host split).
- A dependency for process and socket calls that the standard library lacks (`libc`, already
  in the workspace, or `rustix`) in `vkxp` only.
