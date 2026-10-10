# Proposal

## Why

`docs/decisions.md` reserves save states and names their mechanism: the guest runs in a
process of its own and a state is a forked copy of that process, kept in memory. The process
form of the guest exists since `run-the-guest-in-a-process`. This change adds the states: a
key saves the game as it is at this frame, another brings it back, in any game, without the
game's own save system and in a few milliseconds.

## What Changes

- Protocol version 8: the host can ask the guest to save a state into a slot; a guest
  process says on connecting whether it runs or is a parked copy; a parked copy can be told
  to resume.
- The guest process forks at a frame boundary. The copy connects to the host by itself and
  waits. To load, the host ends the running guest and resumes the copy, which first forks a
  new copy of itself, so a slot can be loaded any number of times, and then sends everything
  the host holds for it again: every bitmap and table in full, the title, the music.
- The session has ten slots and keys: F3 saves, F4 loads, Shift+F3 and Shift+F4 choose the
  slot. A headless run takes a schedule of saves and loads for tests.
- In a window the guest runs in a process by default, since the keys need it.

Out of scope: states that outlive the run or are written to disk; states with the guest as
a thread; rewind; restoring files the game wrote after the state was saved; restoring the
position of the music (it starts over); platforms without `fork`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: the state messages and version 8.
- `rgss-runtime`: saving at a frame boundary and resuming.
- `host-session`: slots, keys, and what a load resets.
- `game-launch`: the default form of the guest in a window, the schedule option, the parked
  copies.

## Impact

- `crates/vkxp-proto` (messages, version), `crates/vkxp-core` (a way to send every bitmap
  and table in full again), `crates/vkxp-binding` (`runtime.rs`: the pending save, the
  resume), `crates/vkxp-host` (`session.rs`, the display trait, the headless runner),
  `crates/vkxp` (`process.rs`: several connections, the fork, ending and resuming
  processes; arguments), `crates/vkxp-vk` and `crates/vkxp-compose-ref` only if a display
  cannot forget its bitmaps and tables without them, `README.md`, `docs/decisions.md`.
