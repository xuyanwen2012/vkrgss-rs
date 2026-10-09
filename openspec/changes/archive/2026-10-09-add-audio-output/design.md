# Design

## Context

- `vkxp_proto::AudioCommand` is `Play { channel, data, volume, pitch }`, `Stop { channel }`,
  `Fade { channel, duration_ms }`; the guest reads the file and sends its bytes.
- `vkxp-host`'s session currently reports "audio is not implemented" once and drops commands.
- `vkxp-audio` is an empty host-side crate.
- The RTP's music is MIDI, its background sounds, music effects and sound effects are Ogg
  Vorbis; games add MP3 and WAV. Fedora installs SoundFonts in `/usr/share/soundfonts/`.

## Goals / Non-Goals

**Goals:** sound for the common formats; a mixer that is tested without any audio device;
no stall of the session or of the audio callback.

**Non-Goals:** the original's exact loudness curve and resampler, MIDI loop markers
(follow-up), WMA.

## Decisions

### Three layers in `vkxp-audio`

1. `decode`: `Source` values that yield interleaved stereo `f32` frames at a requested sample
   rate. Sampled formats through `symphonia` (features `ogg`, `vorbis`, `wav`, `pcm`, `mp3`
   only); MIDI through `rustysynth`. Format detection by probing the bytes.
2. `Mixer`: owns the four channels and exposes `apply(AudioCommand)` and
   `render(&mut [f32])`. Pure, deterministic, no threads, no device: every scenario is tested
   here by rendering into a buffer.
3. `Output`: opens the default `cpal` device, owns a `Mixer` behind the stream callback, and
   receives commands through a channel. Decoding that could take long (opening a file, the
   first block) happens on a worker thread; the callback only mixes ready blocks.

Pitch is a resampling ratio (linear interpolation). Volume is a linear factor
(PLACEHOLDER: the original maps volume to decibels). Fades are linear. Looping restarts the
source from its beginning.

### Sink trait in the host

`vkxp-host` defines `AudioSink` (`fn command(&mut self, AudioCommand)`); the session holds a
boxed sink. The windowed runner builds one from `vkxp_audio::Output` unless audio is disabled;
the headless runner uses a null sink. The existing "not implemented" report is removed.

### SoundFont

`--soundfont` or, in order, `/usr/share/soundfonts/default.sf2`,
`/usr/share/soundfonts/FluidR3_GM.sf2`, `/usr/share/sounds/sf2/FluidR3_GM.sf2`. The file is
loaded once on first MIDI use.

### Tests

WAV and MIDI inputs are generated in the test. One Ogg Vorbis and one MP3 fixture of under
20 kB each, generated for this purpose (a sine tone; state the tool and command in a README
next to them), are committed under `crates/vkxp-audio/tests/data/`. No test opens a device.

## Risks / Trade-offs

- [A whole decoded music track held in memory] → Sampled sources are decoded incrementally.
- [`cpal` on a machine without a sound server] → Covered by the no-device rule.

## Follow-ups

- MIDI loop markers (controller 111), WMA, the original's volume curve.
- The callback decodes every block after the first of a sampled source, and reopens a
  looping source when it wraps, itself; only opening a file and its first block are on the
  worker thread. Should a slow file ever make the output stutter, blocks decoded ahead on
  the worker are the next step.
- The output stream is opened with `f32` samples. A device that only takes integer samples
  fails to open and is reported under the no-device rule; whether to convert is open.
- MIDI data inside a RIFF container (`.rmi`) is not recognised and is reported as unsupported.
- A pitch outside 50 to 150 is clamped to that range; what the original does with such values
  has not been looked at.
- `cpal` links ALSA on Linux, so building now needs the ALSA development files
  (`alsa-lib-devel`); the README lists them.
