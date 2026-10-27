# Keep the audio worker sound

## Why

The project review of 2026-10-26 (`target/review-2026-10-26/merged.md`, F1, C5, F8 L4-L6, L9)
found six defects in `vkrgss-audio`:

- `Audio.bgm_fade(0)` followed by another fade before the callback mixes gives the voice a
  volume of `0 / 0`, NaN; `f32::clamp` lets NaN through, so NaN samples reach the device.
- The device callback is documented to never wait, but a source that meets damaged data
  while the callback reads it calls the shared report, which takes a mutex and writes to the
  error stream. The callback also drops whole decoded sources when a voice is replaced.
- A save state taken within one audio buffer of a play command remembers the channel as
  silent: the loader takes "the voice holds the other reference" as "the voice is alive",
  and the voice gets that reference only when the callback applies the command.
- A MIDI file played at a pitch whose synthesiser rate falls outside 16 000 to 192 000 Hz
  (pitch outside about 25 to 300 at 48 kHz) fails to open and is reported as "not in a
  supported format", which also uses up the one report of really undecodable data.
- Loading a state saved after an hour of looping MIDI music synthesises the whole hour on the
  worker before any later command is handled. This change leaves that open: see the
  follow-up in design.md.
- A panic while the worker prepares a command (in a decoder, on malformed game data) ends the
  worker, and every later command is dropped without a word.

## What Changes

- A fade of no time ends the channel at once, and a fade that follows one never makes NaN.
  The mix maps NaN to silence before the device sees it.
- The callback takes no lock, writes no report and frees no decoded source: damage reports and
  voices that end are handed to the worker, which reports and drops them.
- A save state taken right after a play command remembers the file and its start position.
- MIDI plays at every pitch of the range: the synthesiser runs at the nearest rate it allows
  and its output is resampled to the requested rate.
- A panic in the worker is caught per command: it is reported once, that command's channel is
  silent, and later commands play.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `audio-playback`: requirements on a fade of no time, MIDI pitch, a callback that does not
  wait, a state taken right after a play command, and a failure in the worker.

## Impact

`crates/vkrgss-audio` (mixer, output, decode, midi, tests). No other crate: the binding keeps
sending what it sends now. `docs/compatibility.md` where it says what MIDI pitch does, if it
says it; `CHANGELOG.md`. No picture changes; the regression games' frames do not depend on
audio.
