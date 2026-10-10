# Proposal

## Why

Since `log-through-tracing` the reports of a session, of a headless run and of the mixer
reach the log through an adapter that turns each written line into a warning, because the
writer of the line has no way to say how serious it is. A frame that was rejected and a
display that could not be frozen are failures that lose something the game asked for, and
they are shown at the same level as "no gamepad is read". With `VKXP_LOG=error` they vanish.

## What Changes

- The session and the headless run say how serious each report is. Failures that lose a
  frame or a freeze are errors; degradations the run continues with stay warnings; lines
  that an option asked for (statistics) are shown at every level as today.
- The text of each report stays as it is.

Out of scope: the mixer's reports (all of them are degradations and stay warnings); new
reports; the format of the log.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-session`: reports carry a severity.

## Impact

- `crates/vkxp-host` (`session.rs`, `headless.rs`, `testing.rs`, the tests that read
  reports), `crates/vkxp` (`log.rs`, where the session is given its reporter).
