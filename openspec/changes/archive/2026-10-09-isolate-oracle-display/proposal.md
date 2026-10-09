# Proposal

## Why

Two runs of `cargo xtask oracle` at the same time fail: the second reports that it cannot
connect to its display. Each run picks an X display number by itself, and the choice races with
other runs and with any other virtual display on the machine. Measurement work runs several
probes side by side, so runs must not disturb each other.

## What Changes

- The harness lets `Xvfb` choose a free display itself (`-displayfd`) and reads the number
  back, instead of probing for one.
- A run only ever stops the `Xvfb` and the Wine server it started.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oracle-harness`: concurrent runs.

## Impact

- `xtask` only.
