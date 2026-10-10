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

## Risks / Trade-offs

- [A public type of `vkxp-host` changes] → The only user outside the crate is `vkxp`.

## Follow-ups

- Severity for the mixer's reports, if one of them turns out to be a failure.
