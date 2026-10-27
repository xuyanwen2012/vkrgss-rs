# Call RGSS from script threads

## Why

The project review of 2026-10-26 (`target/review-2026-10-26/merged.md`, B4, F8 L3) found:

- The binding keeps its guest state, its class handles and its queues of collected objects in
  Rust thread-locals of the interpreter thread (`vkrgss-binding/src/guest.rs:68-73`,
  `element.rs:219-222`, `input.rs:21-25`, `stats.rs:131-134`). Ruby 1.9.2 runs a script thread
  on a native thread of its own, under the interpreter lock. In a game of VX Ace,
  `Thread.new { Bitmap.new(1, 1) }` panics in the host function and raises
  `panic in a host function`; `Input.press?` answers false there; and a collection that runs on
  such a thread queues the freed bitmaps and elements where the interpreter thread never looks,
  so they leak and collected sprites stay in the scene. Scripts of VX Ace that load or animate
  in a thread exist; `docs/decisions.md` says script threads "run".
- The trampolines of both interpreter crates state "Ruby only calls methods on the interpreter
  thread" as the reason a `Ruby` token may be made, which is false over Ruby 1.9.2.
- The names `tilemap` and `autotile0` to `autotile6` that the binding interns for a tilemap of
  RGSS1 after the scripts started were never compared with the original XP runtime; symbol
  numbers decide the order of instance variables in a save of Ruby 1.8, and for RGSS2 such names
  were measured and matched (`vx-v3-save-REPORT.md`).

## What Changes

- What the original VX Ace runtime does when a script thread calls RGSS functions is measured
  first. The binding then lets every RGSS class work from a script thread as from the main
  script, which is the expected result; if the original refuses some call from a thread, the
  binding raises the same error there instead.
- The guest state, class handles and queues of collected objects become one state of the
  process, reachable only by a thread that holds the interpreter; a collection on any thread
  queues where the next access looks.
- The safety comments of the trampolines say what is true: the caller holds the interpreter.
- The names a tilemap of RGSS1 interns are measured on the original XP runtime, and the binding
  interns the same names at the same moments, or names of the start-up table when the original
  interns none.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `binding-over-two-interpreters`: a requirement on RGSS classes called from script threads.
- `rgss-runtime`: a requirement on the names a tilemap of RGSS1 interns.

## Impact

`crates/vkrgss-binding` (`guest.rs`, `element.rs`, `input.rs`, `stats.rs`, `tilemap.rs`, tests),
`crates/vkrgss-binding-rgss3` (tests), `crates/vkrgss-ruby-sys` and `crates/vkrgss-ruby192-sys`
(`host.rs`: safety comments; no behaviour change expected). Measurement scripts and outputs go
under `local/` of the clone or `vkxp/target/`, never into the repository except the numbers in
tests and in `docs/compatibility.md`. `docs/decisions.md` (script threads), `docs/compatibility.md`,
`CHANGELOG.md`. Saves of XP may change only if the measurement says the original's differ.
