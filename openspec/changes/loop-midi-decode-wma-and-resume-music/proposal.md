# Loop MIDI at its marker, decode WMA, resume music after a load

## Why

Three known differences of the audio side, all listed in `docs/compatibility.md` or under
the limits of the save states:

- A MIDI track loops from its beginning. Music written for RPG Maker marks the start of the
  loop with controller 111, so a track with an introduction plays the introduction again
  at every round.
- WMA data is unsupported. The original runtime plays it, and one of the games of the
  regression corpus ships three sound effects as WMA.
- After a state is loaded the music and the background sound start over, although the game
  goes on from the middle of a scene.

## What Changes

- A looping MIDI track goes back to its first controller 111 where it has one.
- WMA data is decoded by the `ffmpeg` program when it is installed.
- A state remembers where the music and the background sound were, and after it is loaded
  they go on from there.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `audio-playback`: the loop marker, WMA, positions that are remembered.
- `host-session`: saving and loading a state remembers and recalls the positions.

## Impact

`crates/vkxp-audio`, `crates/vkxp-host` (the audio sink and the session, with their tests
and test doubles), `docs/compatibility.md`, `docs/running.md`, `README.md` where it lists
the formats. The protocol does not change. No picture changes.
