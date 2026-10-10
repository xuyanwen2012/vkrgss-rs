# Design

## Context

- `Session::report` (`crates/vkxp-host/src/session.rs`) writes `vkxp: <text>` lines to an
  injected `Write`; it has six callers: a script's message text, a transition mask that does
  not exist, a rejected frame, two freeze failures, and the frame statistics lines. The
  headless run (`headless.rs`) has a helper of the same kind.
- Tests inject `testing::ErrorBuffer` and read the lines.
- `crates/vkxp/src/log.rs`: `EventLines` implements `Write` and emits each line as
  `tracing::warn!` with the target of the side.
- `vkxp-host` already depends on `tracing` for its other events.

## Decisions

- A small trait in `vkxp-host`, `Reporter`, with one method taking a severity (`Error`,
  `Warning`, `Always`) and the text. The session and the headless run hold a boxed reporter
  instead of a boxed `Write`.
- `testing::ErrorBuffer` implements it by keeping the lines as today, so tests that read
  text need no change beyond construction; it also keeps the severity of each line so that a
  test can ask for it.
- The executable passes a reporter that emits `tracing::error!` or `tracing::warn!` with the
  host's target, and writes `Always` lines the way option-requested lines are written today.
  `EventLines` stays for the mixer.
- Severities: rejected frame and freeze failure are errors; a missing transition mask is a
  warning; a script's message text and statistics lines are `Always`. Classify the headless
  helper's callers by the same rule and list them under "Checked".
- Report texts do not change.

## Checked

- The headless run writes no report of its own. `headless.rs` has no helper that writes
  lines: the closure named `report` in `run_headless` builds the `HeadlessReport` the run
  returns, and its three callers (the frame limit of 0, the shutdown, the frame limit) write
  nothing. Every line of a headless run comes from its session, which is given the run's
  reporter, so the classification of the session is the classification of the run:
  - `frame <n> was rejected: <error>`: error.
  - `cannot freeze the display: <error>` (both callers): error.
  - `the transition's mask, bitmap <i> of generation <g>, does not exist; showing a plain
    cross-fade`: warning.
  - a script's message text: `Always`.
  - the lines of the frame statistics: `Always`.
  What `run_headless` returns as an error (`HeadlessError`) is written by the executable
  directly, as before.
- "The way option-requested lines are written today" is read with the proposal ("shown at
  every level"): the executable writes an `Always` line to the error stream directly, as
  `vkxp: <text>`, like the other text the game produces (see the module documentation of
  `log.rs`). Each line of a text of several lines has the prefix, an empty line too, as the
  line adapter wrote it before, so at the default level the output is the one written before. With `--verbose` or a
  filter it no longer has the level and the target in front of it, because it is not an
  event.
- A session that is given no reporter (the default of the options, used by the examples and
  the tests) writes every report to the error stream as `vkxp: <text>`, as before.
- Scenario "Rejected frame at error level" is covered in two halves, because no game can make
  the backend reject a frame from a script: `vkxp-host/tests/headless.rs` checks that the
  headless run reports the rejected frame as an error, and `vkxp/src/log.rs` checks that the
  executable's reporter shows an error and no warning under `VKXP_LOG=error`.

## Risks / Trade-offs

- [A public type of `vkxp-host` changes] → The only user outside the crate is `vkxp`.

## Follow-ups

- Severity for the mixer's reports, if one of them turns out to be a failure.
- The presentation statistics (`--present-stats`) and the pacing report of the windowed
  display are option-requested lines too, but they are `tracing::warn!` events of
  `windowed.rs` and do not go through the session's reporter, so they still vanish with
  `VKXP_LOG=error`. Whether they should be shown at every level is not decided here.
