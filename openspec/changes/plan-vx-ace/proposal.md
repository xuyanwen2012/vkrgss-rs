## Why

V3 is reached and 0.2.0 is released. `docs/decisions.md` leaves VX Ace open: "How it is
packaged and chosen, what `--guest thread` means for it, and everything else about VX Ace
is decided after V3." On 2026-10-13 the original runtime of VX Ace was measured under Wine
(the V0 of VX Ace) and the embedding of Ruby 1.9.2 was studied, and the owner decided the
open points. This change writes those decisions and the milestones down before any change
implements VX Ace.

## What Changes

- `docs/decisions.md`: the section "VX Ace" holds the decisions of 2026-10-13 (interpreter,
  guest executable, `--guest thread`, the shape of the binding, encodings, threads, hash
  seed, corpus, order of work); "Process" says the standing go-ahead covers VX Ace through
  its release; the milestone table gets the rows A0 (done, with its record) and A1 to A3
  (not started); the risks get those the study named.
- No program change. The README, the agent guide and `docs/compatibility.md` keep saying
  that VX Ace is planned.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-runtimes`: the order and milestones of VX Ace and its recorded decisions.

## Impact

`docs/decisions.md` only.
