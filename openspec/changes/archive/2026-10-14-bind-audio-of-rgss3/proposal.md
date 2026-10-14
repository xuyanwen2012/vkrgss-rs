# Bind Audio of RGSS3

## Why

The standard scripts of RPG Maker VX Ace call `Audio.bgm_play` with four arguments before
their first frame (the built-in `RPG::BGM#play`), and read `Audio.bgm_pos` whenever the
music is stopped or a game is saved. With the module of RGSS2, which a game of RGSS3 sees
today, the first raises `ArgumentError` and the second does not exist, so no game made with
the standard scripts reaches its title. The position is more than a call that has to exist:
a save of a game holds two copies of it, and milestone A3 asks for a save that is byte for
byte the original's. The original's value follows a rule that was measured exactly (982
readings, none unexplained), but it advances with real time; vkxp has to compute it from
the frames the game has shown, or two runs of vkxp would not write the same save.

## What Changes

- In a game of RGSS3 `Audio.bgm_play` and `Audio.bgs_play` take a fourth argument `pos`,
  and `Audio.bgm_pos`, `Audio.bgs_pos` and `Audio.setup_midi` exist, with the argument
  forms and errors measured on the original. Volume and pitch follow the measured limits
  of RGSS3: no clamp to 50 to 150, a volume below 0 is full volume, a pitch below 1 or one
  that asks for more than 200000 Hz is the normal rate.
- `Audio.bgm_pos` and `Audio.bgs_pos` return the position by the measured rule (bytes of
  16-bit PCM, `max(0, decoder - 16 chunks)`, a chunk of 187.5 ms, `LOOPSTART` and
  `LOOPLENGTH` in upper case). The guest computes it, from the frames it has shown and the
  format of the file, and never asks the host: the value is the same with and without an
  audio device, in a window and headless, and in every run with the same input.
- The guest learns the format of a sound itself: a new probe in `vkxp-core` reads the
  header of a WAV file and of an Ogg Vorbis file (sample rate, channels, length, the two
  loop tags) from the bytes the guest already holds. No decoder moves to the guest.
- The same music played again with the same pitch does not restart: the volume is changed
  by a new audio command, and `pos` is ignored, as measured.
- While a music effect plays, the position of the music stands still; when the effect has
  ended the music returns to its volume over one second, as measured.
- A game of RGSS3 looks for a sound with the extensions in the order `ogg`, `mid`, `mp3`,
  `wav`, `wma` (measured for 17 sets of names).
- The play command of the protocol carries where the sound starts and over what time the
  music returns after a music effect, and a volume command is added. **BREAKING** for
  encoded streams: the protocol version rises by one, so an input recording or a stream of
  the version before is refused.
- The mixer starts a sound at the position of its command, raises the music over the
  return time after a music effect, changes a volume without a restart, and plays at every
  pitch from 1 up.
- `docs/compatibility.md` gets what of `Audio` works for VX Ace and the known differences;
  `docs/decisions.md` records that the position is the guest's and follows frames.
- Not in this change: `Audio.__reset__`, which the change
  `run-rgss3-games-in-a-guest-of-their-own` defines; a loop of the sound itself at the
  loop tags (the position follows the tags, the mixer still repeats the whole file); the
  position of a MIDI file, which was not measured; one sound effect per file and frame.
  XP and VX do not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-modules`: the functions of `Audio` in RGSS3, their limits, the same music again,
  the position and its model, the probe of a sound's header; "Audio playback" is qualified
  by runtime.
- `audio-playback`: the start position, the volume command, the return of the music after
  a music effect, the pitch from 1 up.
- `guest-host-protocol`: the two fields of the play command, the volume command, the
  version.
- `game-files`: the order of the audio extensions for a game of RGSS3.

## Impact

- `crates/vkxp-core`: a new module `sound` (`src/sound.rs`, tests beside it) and its line
  in `src/lib.rs`. Nothing else of the crate.
- `crates/vkxp-vfs`: `src/files.rs` (a kind for the audio of RGSS3 and its tests).
- `crates/vkxp-proto`: `src/audio.rs`, `src/codec.rs` (the version), `src/lib.rs` (the
  paragraph on versions), `tests/codec_props.rs`.
- `crates/vkxp-audio`: `src/mixer.rs`, `src/decode.rs`, `tests/mixer.rs`,
  `tests/output.rs`.
- `crates/vkxp-host`: the literals of the play command in `src/session/tests.rs` and
  `tests/headless.rs`; `crates/vkxp-vk`: the literal in `src/tests/compositing.rs`.
- `crates/vkxp-binding/src`: `audio.rs` (the functions by runtime, the state of the
  model), `guest.rs` (one field), `runtime.rs` (the two fields in the play commands a
  resumed copy sends), `graphics.rs` (one call where a frame is shown). Tests: the literals
  of the play command in `tests/modules.rs` and `tests/states.rs`, and a new file
  `crates/vkxp-binding-rgss3/tests/audio_rgss3.rs`.
- Documents: `docs/compatibility.md`, `docs/decisions.md`, `docs/running.md` ("Audio"),
  `docs/architecture.md` and `README.md` where they name the protocol version,
  `CHANGELOG.md`.
- Other changes in flight touch the same files: `draw-sprites-and-snapshots-as-rgss3`
  raises the protocol version too (`codec.rs`, `lib.rs` of `vkxp-proto`, `README.md`,
  `docs/architecture.md`) and edits `graphics.rs`;
  `run-rgss3-games-in-a-guest-of-their-own` edits `audio.rs` (`Audio.__reset__`),
  `graphics.rs`, `runtime.rs` and the section on VX Ace of `docs/compatibility.md`;
  `bind-the-values-of-rgss3` edits `guest.rs` and the same section.
- Nothing of the original is committed: the sounds of the tests are made by test code.
