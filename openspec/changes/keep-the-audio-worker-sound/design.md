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
- **Remember right after play.** The loader tells "the voice ended" from "the voice has not
  started" with a state the two share (for example an atomic that the voice sets when it is
  dropped or ends), instead of the reference count. A voice that has not started yet is
  remembered at the position the loader opened it at.
- **MIDI at any pitch.** The synthesiser runs at the requested rate clamped into
  16 000..=192 000 Hz, and the MIDI source resamples its output to the requested rate by
  linear interpolation, as sampled data is resampled. What the original does at such
  pitches is not measured (its MIDI goes through DirectMusic); the pitch then changes speed
  and pitch together, as it does for sampled data. Recorded in `docs/compatibility.md`.
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

None yet.
