# Design

## Context

- `crates/vkrgss-audio/src/output.rs`: a worker thread turns commands into `Prepared` ones
  (`Loader::prepare`) and sends them to the device callback, which applies them
  (`Channels::apply`) and mixes (`Channels::render`). The module comment says the callback
  never waits.
- `mixer.rs:376-386` `Voice::fade` scales the volume by `remaining / total` of the fade before;
  a fade of 0 ms is `Fade { remaining: 0, total: 0 }`. `mix` ends a voice whose fade has
  `remaining == 0` only when it next mixes, so two fades applied in one callback meet the
  `0 / 0`. `render` clamps with `f32::clamp`, which returns NaN for NaN.
- `decode.rs` `Report` is called from `Sampled::read`, that is from the callback; the report
  of `Mixer::new` locks `Reports` and writes to `errors`.
- `mixer.rs:148-156` `Loader::remember` uses `Arc::strong_count(&position) > 1`.
- `mixer.rs:176-177` opens a source at `device_rate * 100 / pitch`; `rustysynth` rejects
  synthesiser rates outside 16 000..=192 000.
- `decode.rs` `Source::start_from` renders and discards `position` frames; for MIDI the
  position counts every frame since the start, loops included (`midi.rs`).
- `output.rs:95-109`: the worker loop has no unwind guard.

## Decisions

- **Fade of no time.** `Voice::fade(0)` ends the voice at once (as a stop of that voice), so no
  later fade can see a fade of total 0; additionally the level of an earlier fade with
  `total == 0` counts as 0. `gain` returns 0 for a volume that is not finite, and `render`
  maps NaN to 0 before clamping. The fade of 0 ms in the original stops the channel; the
  existing requirement already says a fade ends by stopping the channel.
- **Callback without waiting.** The report a source calls while it plays does not lock or
  write: it hands the error to the worker (a message on the worker's request channel, which
  the worker reports through `Reports` as now). Voices that the callback replaces or that
  end are sent back to the worker to be dropped, over a channel the callback only sends on.
  The worker therefore also wakes for these messages; it may drain them each time it
  receives a request and on a short timeout. The callback still allocates its scratch buffer
  once when the device asks for a larger buffer than before; that stays.
  As built, the damage takes the same channel as the voices, not the request channel: the
  worker blocks on the request channel, and a send to a channel with a blocked receiver
  wakes it under a lock of the standard library. The worker only drains the channel of the
  callback, each time it receives a request and every 100 ms, so a send there wakes nobody.
- **Remember right after play.** The loader tells "the voice ended" from "the voice has not
  started" with a state the two share (for example an atomic that the voice sets when it is
  dropped or ends), instead of the reference count. A voice that has not started yet is
  remembered at the position the loader opened it at.
  As built, the shared `Place` holds the position and a flag that the channels set when they
  end a voice. The test of the scenario passes on the reference count too: the prepared
  command waiting in the channel to the callback held the second reference. The count
  stopped being usable with the change above, since a voice that ended is now alive until
  the worker drops it.
- **MIDI at any pitch.** The synthesiser runs at the requested rate clamped into
  16 000..=192 000 Hz, and the MIDI source resamples its output to the requested rate by
  linear interpolation, as sampled data is resampled. What the original does at such
  pitches is not measured (its MIDI goes through DirectMusic); the pitch then changes speed
  and pitch together, as it does for sampled data. Recorded in `docs/compatibility.md`.
  As built, the position of a MIDI source counts frames of the synthesiser, at the rate it
  runs at; at the pitches the synthesiser can render that is the requested rate, as before.
- **Recall of looping MIDI.** For a looping MIDI source, `start_from(position)` first reduces
  the position: positions past the end of the first pass map to the same offset within the
  loop (`intro + (position - intro) % loop_length`, in frames at the source's rate, from the
  file's event times and its controller 111). Then it renders up to the reduced position as
  now. Voices that sounded across the loop point may differ in their release tails from an
  uninterrupted playback; that is accepted and recorded. The source's reported position after
  the reduction keeps counting from the reduced value; remembered positions are only used by
  the same source kind, so this is consistent.
- **Worker panics.** Each `prepare`, `remember` and `recall` runs under `catch_unwind`. A
  panic in `prepare` turns the command into its silent form (a play command with no
  source); the panic is reported once through the reports as an undecodable sound.
  As built, the test-only source reaches the loader through a hidden field of
  `MixerOptions`, `open`, the function that opens the data of a play command
  (`decode::open` by default), which the test replaces with one that gives a source that
  panics while it is started at a frame.
- **Tests.** Mixer tests in `crates/vkrgss-audio/tests/mixer.rs` and `output.rs`, each named
  after the scenario: `fade_of_zero_then_another_fade_is_silent_not_nan`,
  `midi_at_pitch_400_and_10_plays`, `remember_right_after_play_keeps_the_file` (through `Output` with a device
  stand-in whose callback the test drives, so the remember request is handled before the first
  buffer),
  `recall_of_looping_midi_after_long_play_is_bounded` (counts frames rendered by the source
  through a test hook, not wall time), `panic_in_prepare_leaves_later_commands_playing`
  (a test-only source that panics), `callback_reports_damage_without_the_error_lock`
  (an error sink that blocks while a test holds it; the render call must return).

## Out of scope

Real-time scheduling of the callback thread; a different MIDI synthesiser; changing what
the binding sends.

## Follow-ups

- **Recall of looping MIDI is not bounded (task 3.2 open).** The reduction above needs the
  time of the loop marker and the length of the track as the sequencer counts them.
  `rustysynth` 1.3.8 keeps the event times of a `MidiFile` and the index of the loop start
  crate-private (`MidiFile::times` and `messages` are `pub(crate)`, the sequencer exposes no
  loop point), and `get_length` alone does not give the loop start. Getting them means one
  of: reading the MIDI file a second time beside `rustysynth` and merging its tracks as the
  library does (a second implementation, which this design does not name and the rules do
  not allow; one was built and withdrawn), a patched or forked `rustysynth`, an upstream
  change that exposes the times, or finding the loop by rendering, which costs more than
  the bound of the requirement. The owner decides which. Until then a recalled position in
  looping MIDI is rendered up to as before, and the requirement "Recall of looping MIDI is
  bounded" is not met; its test, `recall_of_looping_midi_after_long_play_is_bounded`, is
  not written, and `docs/compatibility.md` and `CHANGELOG.md` say nothing about the recall
  of looping MIDI, whose behaviour this change leaves as it was.
