# Design

## Context

- Bare `eprintln!` in production code: `crates/vkxp/src/main.rs` (usage, fatal errors, a few
  run-time reports), `crates/vkxp-host/src/windowed.rs` (pacing report, present statistics,
  draw errors), `gamepad.rs`, `record.rs`, `crates/vkxp-audio/src/output.rs`.
- Three helpers write to an injected `Write`: `Session::report` (`session.rs`), the one in
  `headless.rs`, and `Reports` in `vkxp-audio/src/mixer.rs`. Tests inject a buffer and read it.
- `crates/vkxp/tests/end_to_end.rs` reads the error stream of the executable.
- `vkxp-binding/src/runtime.rs` prints the script error report; that is the game's output.
- Examples and `build.rs` files print too; they are not the program.

## Decisions

- Three kinds of text on the error stream, and only the third changes:
  1. What the game produces: `p` and `print` in log mode, the script error report. Direct
     write, shown at every level.
  2. The program's own result: usage, and the error that ends it. Direct write.
  3. Diagnostics of a running program. These become events.
- Levels: `error` for a failure that loses something the user asked for (a frame that cannot
  be drawn, the audio stream failing); `warn` for a degradation the program continues with
  (no gamepad, an undecodable sound, MIDI without a SoundFont, a stubbed call); `info` for the
  choices listed in the proposal and for present statistics and pacing reports when their
  option asked for them; `debug` for anything added beyond that. Present statistics are
  requested by an option, so they are shown whatever the level: emit them at `warn` or write
  them directly, whichever keeps `--present-stats` working without `--verbose`.
- The subscriber is installed once in `vkxp`'s `main`, before anything else runs, with
  `tracing-subscriber`'s `fmt` layer writing to the error stream. Its default format is the
  line `vkxp: <message>` with no time and no level, so warnings and errors read as they do
  today. With `--verbose` or `VKXP_LOG` the format adds the level and the target.
- Level selection: default `warn`; `--verbose` is `debug`; `VKXP_LOG` takes a level name or
  `tracing-subscriber` filter directives (`vkxp_host=trace`) and wins over `--verbose`. A
  value that does not parse ends the program like a malformed option.
- The injected streams stay, because tests depend on them. Where the executable passes the
  error stream to a session, a headless run or the mixer, it passes an adapter that emits
  each completed line as an event instead (one small type in `vkxp`). The helpers themselves
  keep writing lines; their `vkxp: ` prefix moves to the formatter if that keeps the output
  the same, otherwise it stays where it is. One line must not get the prefix twice.
- Informational events are emitted from `vkxp` and `vkxp-host` with data they already have
  (the device name and pacing mode that `vkxp-vk` returns, resolved paths). No interface of
  another crate changes for them.
- Library crates depend on `tracing` only. `vkxp-core`'s allow-list is untouched.

## Risks / Trade-offs

- [A test pins a line of the error stream] → The default format reproduces today's lines;
  where one differs, keep the old text.
- [Events before the subscriber exists are lost] → Install it first; argument errors are
  direct writes anyway.

## Follow-ups

- Spans and frame timings (`measure-frame-times`).
- Events from `vkxp-vk` and the guest crates.
