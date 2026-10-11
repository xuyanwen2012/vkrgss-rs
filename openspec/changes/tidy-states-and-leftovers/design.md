# Design

## Context

See the proposal. All five items are local; none changes a frame.

## Decisions

### The session names its next deadline

`Session` has one method that returns the earliest of the pending tick's due time and the end
of the notice, and the window loop uses it for `ControlFlow::WaitUntil` where it uses the
pending tick today. `poll` already clears a notice that is over. While a load waits for its
answer the pending tick is not due (as today), but the end of a notice still is.

### `StateFailed { slot, load }`

A `bool` field `load`: true for a failed load, false for a failed save. Senders: the guest
runtime (`false` after a failed copy; for a guest that cannot be copied, by the request it
answers), the relay in `vkxp` (`true`: the slot holds nothing), the demo guest and the test
guests. The session: a failed load ends the wait for that slot and shows "slot N holds no
state"; a failed save shows "no state could be saved into slot N" and leaves a waiting load
alone. A failed load for a slot the session does not wait for is shown and otherwise ignored.

The version becomes 9; whatever carries the version (fixtures, expected files, README) is
regenerated or edited as it was for version 8.

### Stale socket directories are removed at the next start

`GuestProcess::start`, before it creates its directory, looks at the entries of the base
directory named `vkxp-<pid>-<8 hex digits>`, and removes those that are directories owned by
the user running `vkxp` and whose `<pid>` names no existing process (the `exists` helper).
Errors are ignored: this is housekeeping.

Not a signal handler: a handler would need a blocked signal set shared with the winit loop,
the interpreter and the child after `exec`, for a directory that holds one socket file. The
sweep also covers `SIGKILL` and crashes. A process id that was given to another process only
delays the removal.

### The two leftovers in `vkxp-core`

`Tilemap::strips(&self)`; its caller in `extract.rs` and the test drop the argument, and
whatever only existed to compute it goes with it. The comment on `Sprite::bush_depth` says:
the lowest screen rows of the box the sprite covers, as many as the depth, are drawn with
half the opacity; see the requirements "Sprite bush depth" and "Bush rows of zoomed and
turned sprites".

## Risks

- The version bump invalidates streams of version 8; nothing stores such streams between
  runs except fixtures in the repository.

## Follow-ups

None yet.
