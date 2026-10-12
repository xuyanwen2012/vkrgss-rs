# Design

## Context

- `crates/vkxp-audio/src/midi.rs` reads a file with `MidiFile::new` and plays it with
  `MidiFileSequencer::play(&file, false)`; a looping voice calls `Source::rewind` at the
  end. `rustysynth` 1.3 has `MidiFile::new_with_loop_type` with
  `MidiFileLoopType::RpgMaker`, which takes the first controller 111 as the loop start.
- `decode::open` recognises MIDI by its header and gives everything else to `symphonia`,
  which has no WMA decoder. Data nothing can play is `DecodeError::Unsupported`, which the
  mixer reports and which is not fatal.
- The session stops all four channels when a state is loaded (`Session::state_loaded`);
  the guest that goes on then sends its music again, which starts from the beginning.
  `StateSaved { slot }` and `StateLoaded { slot }` reach the session.
- That controller 111 marks the loop is the documented convention of RPG Maker. It was not
  measured on the original runtime, whose MIDI output under Wine cannot be compared sample
  by sample.

## Decisions

### MIDI loop

The file is read with `MidiFileLoopType::RpgMaker`. A voice that loops (music and
background sound) continues at the marker when the track ends; a file without a marker
loops from its beginning as today. A voice that does not loop (effects) ends at the end of
the track. How the source learns whether it loops is the implementer's choice inside
`vkxp-audio`; the `Source` trait may change.

### WMA through `ffmpeg`

Data that starts with the ASF header object identifier (the sixteen bytes
`30 26 B2 75 8E 66 CF 11 A6 D9 00 AA 00 62 CE 6C`) is decoded by running
`ffmpeg -v error -i pipe:0 -f f32le -ac 2 -ar <rate> pipe:1`, with the data on standard
input and the mixer's sample rate; what comes back is a sampled source like a decoded WAV.
This form was tried on one of those files with ffmpeg 8.1.3. The name of the program
is a field of `MixerOptions` with the default `ffmpeg`, looked up on the search path; there
is no command-line option or configuration key for it.

When the program cannot be started, ends with a failure or returns no frame, the data is
`Unsupported` as today, and the message says that WMA needs `ffmpeg`. Decoding happens
where files are decoded today, not where they are mixed. No result is cached; no
time limit is put on the program.

No WMA decoder is linked or written.

### Positions across states

`Source` gains the position of the next frame it will give, counted in frames from the
start of the data at the source's rate, and a way to start from a position. For sampled
data that is an index. For MIDI the position is the number of frames rendered since the
start, and starting from one renders and discards that many frames where the file is
decoded.

`AudioSink` gains `remember(slot)` and `recall(slot)`:

- `remember(slot)` stores, for the music channel and the background sound channel each,
  the name the play command gave and the position of the voice at that moment, or nothing
  when the channel is silent. It replaces what the slot held.
- `recall(slot)` arms both channels with what the slot holds. The next play command on an
  armed channel disarms it, and when it names the remembered file the voice starts at the
  remembered position instead of the beginning. Volume and pitch are those of the command.
  A fade that was running when the state was saved is not restored.

The session calls `remember` when `StateSaved { slot }` arrives and `recall` in
`state_loaded`, after it stopped the channels. The memory effect and sound effects are not
remembered. What is remembered lives as long as the mixer: states live in memory too.

The position is that of the mixer, which is ahead of what is heard by the delay of the
output device.

## Risks

- [`ffmpeg` reads data that comes from a game] → it is the user's installed program, run
  without a shell, with fixed arguments.
- [Starting a MIDI track from a late position takes time] → rendering is many times faster
  than playing; it happens off the audio thread.

## Follow-ups

- A track with more than one controller 111. `rustysynth` moves its loop start at every
  controller 111 it passes, so such a track continues at the last marker it played, not at
  the first as the requirement says. With one marker, or several at the same time (one for
  each MIDI channel), the two are the same. Following the first would need the file to be
  rewritten in front of the library, which this design does not name; it was not built.
