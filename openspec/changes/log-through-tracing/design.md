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

## Implementation notes

- The filter is `tracing-subscriber`'s `EnvFilter`, built from the value of `VKXP_LOG`. Its
  parser rejects `=`; the lighter `Targets` filter accepts it, so it was not used.
- Every direct write left in `crates/vkxp/src/main.rs` is the usage message or the message
  of a failure that ends the program with a non-zero status (a host error of a headless run,
  a session that ended without the game shutting down, scripts that do not end). They stay
  direct, so that no level hides why the program failed.
- The executable passed no stream before: the session and the mixer of a windowed run took
  their default, the error stream, inside `vkxp-host`. `WindowedOptions` therefore has two
  new fields, `errors` and `audio_errors`, like `HeadlessOptions::errors`, and the executable
  fills all three with the adapter. `WindowedOptions` no longer derives `Debug`, which a
  boxed stream does not have and nothing used.
- The adapter cannot tell what a line is, so every line is a warning, with the target
  `vkxp_host` or `vkxp_audio`. The helpers keep their `vkxp: ` prefix, which tests of
  `vkxp-host` and `vkxp-audio` read; the adapter leaves it out and the formatter writes it.
  A report of several lines (the host's echo of a modal text) now has the prefix on each
  line instead of on the first only.
- Present statistics and the report that present-timing pacing stopped are warnings.
- The format with `--verbose` or `VKXP_LOG` is `vkxp: <LEVEL> <target>: <message>`.
- The archive is named by the executable, which looks for `Game.rgssad` in the game
  directory itself: `vkxp-vfs` does not say whether it opened one, and its interface does
  not change here.

## Follow-ups

- Spans and frame timings (`measure-frame-times`).
- Events from `vkxp-vk` and the guest crates.
- The two diagnostics of `vkxp-binding` (a process function that is turned off, a `Win32API`
  call that is not emulated) are not events yet, and the crate has no `tracing` dependency.
  The Ruby prelude writes them to the script's `$stderr` as lines `vkxp: <text>`, and the
  tests `tests/prelude.rs` and `tests/win32api.rs` read them from a `$stderr` they replaced.
  As events they would no longer reach a replaced `$stderr`, so those tests could not pass
  unchanged, which task 2.1 asks for; keeping both would need a second path beside the
  event, which this design does not name. Today they are shown at every level, with the
  default format's prefix. Open question: should they become warnings through a host
  function, with the tests changed to match?
- A line that comes through the adapter is a warning whatever it says. A rejected frame, or
  a display that cannot be frozen, would be an error if the session emitted events itself;
  that means the session's tests reading events instead of an injected stream.
