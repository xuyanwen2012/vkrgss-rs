# End the session without the guest

## Why

The review of 2026-10-26 (`target/review-2026-10-26/merged.md`, findings C1, B3, B6, C6 and
Fable's L7, L10 and L16) found that the program depends on the guest's cooperation to end, and
that the relay trusts it to keep the pace:

- Closing the window only queues `Quit` (`vkrgss-host/src/session.rs` `close_requested`); the
  event loop exits only once the session has an end. A script in `loop {}` that calls no RGSS
  function never reads the request, and the window stays open for ever. With the guest on a
  thread, `run_windowed` then joins a thread that never returns.
- The relay polls for readability and then reads a whole message with a blocking read, and
  writes to the guest with a blocking write (`crates/vkrgss/src/process.rs`). A guest that
  stops inside a message, or stops reading while the socket buffer fills, holds the relay
  where its deadline cannot act, and the program's exit waits for it.
- A connection to the guest socket is read inline, with a timeout of 5 s, in the loop that
  hands on the game's messages; a process that connects and says nothing freezes the game for
  5 s each time, and a persistent `accept` error (`EMFILE`) makes the loop spin.
- Messages that are not frames (`Audio`, `SetTitle`, `Modal`) are not paced by ticks, and the
  link to the host is an unbounded channel: a guest can queue memory without limit. Every error
  of the framing becomes `Disconnected` without its reason.
- `GuestReceiver::try_recv` over a socket switches the whole socket to non-blocking mode and
  back, which a send on another thread through the split sender would see.
- Keys a script injects with `keybd_event` also cause the program's own shortcuts: save and
  load of states (F3, F4), fast-forward (Tab) and full screen (F11), which the original runtime
  does not have.
- A headless run moves its virtual clock to the end of every state notice, so the elapsed time
  it reports grows with the notices.
- `GuestForm` defaults to `Thread` in the code although a game runs in a process by default.

## What Changes

- After a request to quit (closing the window, a frame limit), a guest that has not ended
  within 5 s is given up on: the session ends, the window closes, the guest process is ended,
  and a guest thread is left behind as the program exits with a failure that says so.
- The relay reads and writes the guest without blocking, with the incremental reader of the
  framing; a guest that stops inside a message or stops reading is ended when the deadline of
  the relay passes. Connections are attached without blocking the relay, and `accept` errors
  back off.
- A guest process may queue at most a fixed number of bytes of messages that are not frames
  between two ticks; one that sends more is ended with a report that names the bound. Framing
  errors keep their reason in the reports.
- `try_recv` over a socket no longer changes the socket's mode.
- Injected keys cause the actions of the original runtime's keys (F12, Alt+Enter) but not the
  program's own shortcuts.
- A headless run's clock moves only to ticks; state notices do not move it.
- `GuestForm::default()` is the process form, and an unused start-up path is removed if no
  caller remains.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-session`: quit with a deadline, injected keys, the clock of a headless run.
- `guest-host-protocol`: the guest end over a stream socket, incremental reading of the
  framing.
- `game-launch`: the relay ends a stalled guest, attaches without blocking, bounds what a guest
  queues.

## Impact

`crates/vkrgss-host` (`session.rs`, `windowed.rs`, `headless.rs`, their tests),
`crates/vkrgss` (`process.rs`, `main.rs`, `args.rs`; `process.rs` is also compiled into
`vkrgss-android`), `crates/vkrgss-proto` (`framing.rs`, `link.rs`), `docs/running.md`
(closing a game, the host keys, injected keys), `docs/architecture.md` where it describes the
relay, `CHANGELOG.md`. This change starts after `verify-guests-and-deny-signals` has merged
and builds on its `process.rs`. No picture changes.
