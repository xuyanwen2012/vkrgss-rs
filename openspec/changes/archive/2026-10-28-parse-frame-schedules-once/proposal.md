# Parse frame schedules once

## Why

A headless run reads two texts of the same shape: the input script (`--input`,
`vkrgss-host/src/script.rs`, `InputScript::parse` at `:47`) and the states schedule
(`--states`, `vkrgss-host/src/states.rs`, `StateSchedule::parse` at `:55`). Both split on
commas and line breaks, trim, skip empty entries, split each entry at its first `:`, parse
the frame as `u64`, and report `MissingColon` and `InvalidFrame` with the entry; only what
follows the colon differs. The two loops are copies, and so are two variants of each error
enum, so a fix to one (for example how blanks or a frame beyond `u64` are handled) has to be
made twice.

## What Changes

- One function of `vkrgss-host` reads the entries of a frame schedule: it splits, trims,
  skips empty entries, divides each at its first `:` and parses the frame, giving the frame,
  the rest and the entry, or the error of the entry.
- `InputScript::parse` and `StateSchedule::parse` use it and keep only what each reads after
  the colon.
- The error messages stay word for word as they are, and so does every result; the public
  types and their variants keep their names.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-session`: the frame entries of the input script and the states schedule are read by
  one rule.

## Impact

`crates/vkrgss-host/src` only (`script.rs`, `states.rs`, a new small module for the shared
reading, their tests). No other crate, no protocol, no picture; the command-line options and
their error messages are unchanged.
