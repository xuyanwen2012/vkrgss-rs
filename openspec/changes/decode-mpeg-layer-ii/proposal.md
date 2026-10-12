## Why

A game of the corpus (`XP-MBBS-7.0`) ships a music file named `.mp3` that holds MPEG-1 audio
layer II. The original plays it; vkxp reports it as unsupported and the scene stays silent,
because the decoder was built with layer III only. The owner noticed missing music on
2026-10-12.

## What Changes

- The decoder is built with layer II as well, so such data is decoded like MP3.
- A fixture of one second of layer II and a test beside those of Ogg and MP3.
- The documents name the format.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `audio-playback`: the requirement "Formats" names MPEG audio layer II and gets a scenario.

## Impact

- `Cargo.toml` (one more feature of `symphonia`, no new crate), `crates/vkxp-audio/tests`,
  `README.md`, `docs/running.md`.
