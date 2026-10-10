# Design

## Context

- `crates/vkxp-host/src/stats.rs` collects rows (count, mean, smallest, largest) and the
  session reports them every 600 frames in a window and at the end of a headless run.
- The guest runs in `vkxp-binding`: `graphics.rs` (`Graphics.update` extracts the frame and
  waits for the tick), `bitmap.rs` and `text.rs` (the Ruby methods that change pixels).
  The executable builds the guest's options in `crates/vkxp/src/main.rs`.
- The guest is deterministic: nothing a script can observe may depend on the host's clock.
  Reading a monotonic clock for a report that scripts cannot see does not break that.
- `vkxp-core` does no I/O and stays free of clocks.

## Decisions

- An option of the guest, set from `--frame-stats`, turns the measuring on. Off: no clock is
  read.
- Measured in `vkxp-binding`, per frame (from the return of one tick wait to the start of
  the next): the time inside bitmap-changing methods (`blt`, `stretch_blt`, `fill_rect`,
  `clear`, `hue_change`, pixel access and the like), inside `draw_text` and `text_size`,
  inside frame extraction and encoding, and the whole frame. "scripts" is the whole minus
  the three. Nested calls are counted once (a depth counter).
- The guest reports its rows itself, as lines of the same form as the host's rows, with the
  names `guest scripts`, `guest bitmap operations`, `guest text`, `guest extraction`, every
  600 frames and when the guest ends. They go to the error stream directly, like the host's
  option-requested lines; no message crosses the boundary for them.
- The existing `guest` row of the host stays; the README says what each row means and that
  the guest's own rows are the ones to read for its cost.
- Tests: a guest run with the option produces the four rows with the frame count; without
  the option none; a frame dump is equal with and without it.

## Risks / Trade-offs

- [Clock reads around short bitmap calls cost time themselves] → Only with the option; two
  reads per call are tens of nanoseconds against calls that touch thousands of pixels.
- [Guest and host rows are printed at slightly different moments] → Accepted; each block is
  labelled.

## Follow-ups

- Per-method breakdown, if the bitmap row turns out to matter.
