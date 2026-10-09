# Design

## Context

- `vkxp-binding`'s run configuration has the game files, the built-in scripts file, fonts and
  flags; `run_game` evaluates the prelude, the built-in scripts and the archive's scripts, and
  starts over on reset.
- `vkxp-host` has a session that takes input from the window's key state or, headless, from an
  input script, and delivers one snapshot per frame.
- `vkxp-vfs` has `GameFiles::open`, which reads `Game.rgssad` and can fail; the executable still
  calls the infallible `GameFiles::new`.
- The design record lists the injectable seed, preload scripts and input record and replay as
  first-version features.

## Decisions

- Seed: a field of the run configuration; `run_game` calls `srand(seed)` through the
  interpreter at the points the spec names.
- Preload: a list of paths in the run configuration, read by the guest (they are the user's
  files, outside the game directory) and evaluated under their file names.
- Recording format: a text file, first line `vkxp-input 1 seed=<n>`, then one line per frame
  with the names of the held buttons, run-length compressed as `<count>x <buttons>`. Human
  readable and diffable; the existing input script syntax is not reused because it is sparse.
- Recorder and replayer are input sources in `vkxp-host` in a new module; the session asks its
  source for each frame's snapshot. Edits to `session.rs` stay small and additive, because a
  parallel change edits its transition code.
- The executable switches to `GameFiles::open` and maps its error to the start-up error path.
- Tests for the archive scenario build the archive with a helper local to the test (the format
  is in `vkxp-vfs`'s documentation).

## Risks / Trade-offs

- [Replay diverges when the game reads the clock] → Out of scope; the guest's time comes from
  frame ticks.

## Follow-ups

- `--input` together with `--replay` is refused with the usage message, so that after the last
  recorded frame of a headless replay no button is held, as the spec asks. Whether an input
  script should instead take over after the recording is open.
- With a recorder or replayer the session sends each frame's snapshot together with that
  frame's tick, so that what is recorded is exactly what the guest saw in that frame. A key
  pressed and released between two frames is therefore not seen while recording. Without a
  source, live input is sent as it changes, as before.
- A reset with F12 is not recorded (only input and seed are in scope), so a session with a
  reset does not replay exactly. Recording host requests would need a format version 2.
- A failure to write the recording during a run is reported once on the error stream and the
  run goes on; whether it should fail the run is open.
