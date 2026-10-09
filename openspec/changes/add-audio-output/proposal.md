# Proposal

## Why

Games send audio commands and the host drops them: vkxp is silent. Milestone M3 is audio for
the formats the RTP and old games use: Ogg Vorbis, WAV, MP3 and MIDI.

## What Changes

- `vkxp-audio`: decoding (Ogg Vorbis, WAV, MP3 through `symphonia`; MIDI through `rustysynth`
  with a SoundFont), a mixer for the four RGSS channels (music, background sound, music effect,
  sound effects) with volume, pitch, fade, looping and the pause of music while a music effect
  plays, and an output stream through `cpal`.
- `vkxp-host`: the session hands audio commands to an audio sink instead of dropping them.
  Headless runs use a sink that plays nothing.
- `vkxp`: options `--no-audio` and `--soundfont FILE`.
- Failure to open a device, to find a SoundFont or to decode a file never stops the game: it is
  reported once and that sound is silent.

Out of scope: WMA, MIDI loop markers, the original's exact volume curve, positional or
per-effect processing, audio in headless runs.

## Capabilities

### New Capabilities

- `audio-playback`: decoding and mixing of the four channels.

### Modified Capabilities

- `host-session`: audio commands reach an audio sink.
- `game-launch`: audio options.

## Impact

- Crates `vkxp-audio` (new code), `vkxp-host` (the audio arm of the session and runner options),
  `vkxp` (two options, README section). New dependencies `symphonia`, `rustysynth`, `cpal`.
