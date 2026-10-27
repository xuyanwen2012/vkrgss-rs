# Design

## Context

- `vkrgss-host/src/session.rs`: `close_requested` calls `request_quit`, which queues
  `HostToGuest::Quit`; the session ends when the guest sends `Outcome`/shuts down or its link
  disconnects. `windowed.rs` exits the event loop once `driver.end()` is set, then drops the
  app and joins the guest thread unconditionally. Headless, `main.rs` waits `GUEST_WAIT`
  (5 s) for the outcome and ends the process if the game does not end.
- `crates/vkrgss/src/process.rs` (after `verify-guests-and-deny-signals`): `Relay::run` polls
  the running guest, the host side and the listener; `guest_message` calls `read_message` on a
  blocking stream; `host_message` calls `write_message`; `accept_copies` runs `attached()`
  inline with `ATTACH_WAIT` (5 s); the host-gone deadline is `GUEST_WAIT` (5 s) and lives in
  the poll loop only.
- `vkrgss-proto/src/framing.rs`: `MAX_MESSAGE_LEN` 2^30, `read_message`/`write_message`.
  `link.rs`: `GuestReceiver` over a stream already keeps unread bytes (`buffered`,
  `take_message`, `read_more`); `try_recv` toggles `set_nonblocking` on the shared socket;
  `GuestSender::send` maps every framing error to `Disconnected`. `HostEnd` is an unbounded
  `mpsc` channel.
- `session.rs` `inject_key` routes through `key_pressed`, which handles F12 (reset), F11 and
  Alt+Enter (full screen), F2 (frame rate), F3/F4 (states), Tab (fast-forward). The original
  runtime has F12, Alt+Enter and F2 of its own.

## Decisions

- **Quit deadline in the session.** The session records when it first asked the guest to quit
  (window closed, frame limit). If the guest has not ended `QUIT_WAIT` = 5 s after that (the
  same as `GUEST_WAIT`), the session ends with a new end, `SessionEnd::GaveUp`, which
  `next_wake` schedules so a windowed loop wakes for it. The program treats it as a failure
  with the message the relay already prints ("did not end within 5 seconds of being asked to
  quit; giving up"). A guest that ends in time ends as before. The deadline is measured on
  the session's clock, so headless tests drive it with the manual clock.
- **Leaving a guest thread behind.** `run_windowed` joins the guest thread only when the session
  ended for a reason other than `GaveUp`; otherwise it returns at once and the program exits
  with `std::process::exit` after its report, as the headless path already does when the game
  does not end. A guest process is ended by the relay's own deadline (below).
- **Relay I/O without blocking.** The running guest's stream and each pending connection are
  non-blocking. Reading uses an incremental reader moved into `framing.rs` (a buffer plus
  "take a whole message if one is there", what `GuestReceiver` has now), shared by
  `GuestReceiver` and the relay. Writing to the guest appends to an outgoing buffer that is
  written when `poll` reports `POLLOUT`; the relay polls for writing while the buffer is not
  empty. A guest whose outgoing buffer exceeds `MAX_MESSAGE_LEN` or whose partial message or
  unwritten bytes make no progress for `GUEST_WAIT` while the host has gone is ended. The
  host-gone deadline is checked on every turn of the loop, so no read or write can hold it.
- **Attaching without blocking.** A new connection's credentials are checked at once (change
  `verify-guests-and-deny-signals`); its version statement and `Attach` are read as they
  arrive, through the same incremental reader, with `ATTACH_WAIT` as its deadline. The game's
  messages are handed on meanwhile. The ordering rule of `Relay::run` (a copy is known before
  what its maker sent after making it) is kept: a message of the running guest is held while a
  connection is pending whose attachment has not been read, up to `ATTACH_WAIT`. An `accept`
  error other than `WouldBlock` is reported once and the listener is left out of `poll` for
  100 ms.
- **Bound on what a guest queues.** The relay counts the bytes of messages other than frame
  packets it hands to the host since it last handed the guest a tick; above 256 MiB the guest
  is ended with a report that names the bound. Frame packets stay paced by ticks and bounded
  by `MAX_MESSAGE_LEN`. A guest on a thread is not bounded (it shares the process).
- **Reasons of framing errors.** `GuestSender::send` and the relay log the framing error's
  text once before the link reports `Disconnected` (e.g. "message of N bytes above the bound").
  `Disconnected` stays the error type of the link.
- **try_recv without changing the socket.** `read_more` in its non-waiting form uses
  `recv(…, MSG_DONTWAIT)` on the socket instead of `set_nonblocking`; the descriptor's mode
  never changes.
- **Injected keys.** An injected key holds its raw key and the buttons of the key map as now,
  and causes the actions of F12, Alt+Enter and F2, which the original runtime has; it causes
  no state request, no slot change, no fast-forward and no F11 full screen.
- **Headless clock.** The headless loop advances the virtual clock only to the time of a
  pending tick; a notice whose end comes first is ended without moving the clock (the session
  polls at the tick's time and finds the notice over).
- **Small ones.** `GuestForm`'s `Default` becomes `Process` or is removed if nothing needs it.
  If no caller starts the program with `--guest-role` any more (`vkrgss-guest/src/role.rs`),
  the branch that reads it in `main.rs` goes, with its tests; otherwise it is left and the
  reason written here.

## As built

- **The relay's deadline counts from the quit request.** The relay notes when it hands the
  guest a quit request; once the host has gone it gives up `GUEST_WAIT` after that request or
  after the host went, whichever came first. Counting from the host's going alone would add
  the relay's 5 s to the session's 5 s, and a windowed run of a guest process in `loop {}`
  would end only after about 10 s, not within the 10 s of the scenario "Script in an endless
  loop".
- **Reading and holding.** The relay reads the running guest only while no connection is
  pending, at most 1 MiB in one turn, and hands on what it has read at the start of the next
  turn: by then every connection that was made before those bytes were sent has been
  accepted, and its messages wait until it has attached or been dropped. A version statement
  is judged as soon as it is whole or departs from the form of the program's own statement,
  so a connection that sends no statement is refused at its first bytes, as before.
- **`MSG_DONTWAIT` without `unsafe`.** `vkrgss-proto` forbids `unsafe`, so the receive that
  does not wait calls `recv` through `rustix` (already in the dependency tree), with its
  features `net` and `fs`; `net` alone does not build in rustix 1.1.5. The crate also takes
  `tracing`, to log the reasons of framing errors.
- **Android.** The pinned toolchain 1.99.1 on the owner's machine has no standard library
  for `aarch64-linux-android`; `cargo +1.99.0 clippy -p vkrgss-android --target
  aarch64-linux-android -- -D warnings`, with the NDK's clang as linker and C compiler, passes.

## Out of scope

Peer credentials and the state answers of the relay (change `verify-guests-and-deny-signals`);
the size of a bitmap against `MAX_MESSAGE_LEN` (change `bound-image-decoding-and-uploads`);
device loss (change `bound-the-compositor-and-survive-device-loss`).

## Follow-ups

- The bound of 256 MiB counts messages other than frame packets only. A guest process that
  sends frame packets without waiting for their ticks is not bounded by the relay: the
  session sets aside one frame while a tick is pending, but the in-process link to the host
  holds the rest. Whether the relay should hand on one frame per tick is open.
