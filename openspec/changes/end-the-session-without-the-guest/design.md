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
- **`MSG_DONTWAIT` and the reasons in `vkrgss-proto`.** `vkrgss-core` reaches
  `vkrgss-proto`, so everything `vkrgss-proto` depends on has to be on the core allow-list of
  `xtask/src/deps.rs` (pure crates, no I/O or FFI); neither `libc`, `rustix` nor `tracing` is.
  The receive that does not wait therefore declares `recv` itself and calls it with
  `MSG_DONTWAIT` (0x40 on Linux and Android, 0x80 on macOS) in one function under
  `#[allow(unsafe_code)]`, the crate going from `forbid` to `deny`: platform code of the
  stream transport, where `AGENTS.md` allows `unsafe`. The reason of a framing error is
  written to the error stream as a line of the program's diagnostics, `vkrgss: <reason>`,
  since the crate has no logger; the relay logs its own through `tracing`.
- **Android.** The pinned toolchain 1.99.1 on the owner's machine has no standard library
  for `aarch64-linux-android`; `cargo +1.99.0 clippy -p vkrgss-android --target
  aarch64-linux-android -- -D warnings`, with the NDK's clang as linker and C compiler, passes.

- **What the bound of 256 MiB counts.** The host holds a message as an entry of the
  in-process link and the heap blocks of its texts, not as its bytes on the wire: a title of
  one character is 14 bytes encoded and about 140 in the host. The relay therefore charges
  each message other than a frame packet its encoded length plus `HELD_PER_MESSAGE`, the size
  of a `GuestToHost` and 64 bytes for up to two heap blocks, so that what the host holds stays
  below the bound for short messages as for long ones. The test floods with titles of one
  character and of 1 MiB and measures the peak resident memory of the test process.
- **Giving up is said whatever the guest reported.** The relay's flag that it gave up means
  that the guest was still there when its time was up, whatever outcome it had reported. The
  relay says so on the error stream unless the guest had reported a game that went well; then
  a headless run stays a success, as before, and when a window gave up on the guest the
  program says it.
- **Program shortcuts follow the physical key.** A key is down as a whole while the physical
  key or the injected one is; the actions of the original runtime's keys follow the key as a
  whole, the program's own shortcuts (F3, F4, Tab, F11) the physical key alone, so an
  injected Tab neither keeps fast-forward on after the physical Tab is released nor keeps a
  physical press from engaging it.
- **A stream that ends inside a message** says so, through the same mapping of framing errors,
  for the receive that waits and for the one that does not.
- **The headless clock and loads.** Moving the clock to ticks alone is not enough for the
  scenario "Save and load": the frame at which a state is loaded never gets its tick, and a
  run with a load would report one period less per load. The headless loop therefore also
  moves the clock to the time of a tick that waits for the answer to a load, without sending
  it; the guest that goes on from the state is then paced as the one that went would have
  been.
- **Injected keys** keep the actions of the original runtime: F12, Alt+Enter, F2, and a key
  of C or B that answers a modal message, as it answers the message box of the original.
- **`GuestForm`** has no `Default` any more; nothing but a test used it.
- **`--guest-role` in `main.rs` stays.** No caller starts the program with it, but the
  requirement "Refused guest record" of `game-launch`, which this change does not modify,
  names `vkrgss --guest-role <path>` in its scenarios, and `crates/vkrgss/tests/guest_record.rs`
  tests them.
- **"Script in an endless loop".** The thread form is a test of the windowed driver with the
  test doubles of `windowed.rs` and of the join that `run_windowed` leaves out after giving
  up. A windowed run cannot be driven in the tests (they have no window system), so the
  process form is covered by the relay's tests of a guest that stops and by the end-to-end
  test of a headless guest process in `loop {}`, which now also asserts the 10 s, counted from
  a mark the script makes as it enters its loop, right after the tick that the quit request
  follows; the owner checks the windowed process form by hand.

- **Checks.** The gate passed with `TMPDIR=/var/tmp/vkrgss-end-the-session-without-the-guest`.
  `xarch check` ran against a baseline of the base revision (`xarch baseline --at e3a60d2f`)
  with `XDG_RUNTIME_DIR` set to that directory too, since the sandbox of the run cannot write
  `/run/user/1000`, where the games of VX Ace make the directory of their socket; all ten games
  were identical.

## Out of scope

Peer credentials and the state answers of the relay (change `verify-guests-and-deny-signals`);
the size of a bitmap against `MAX_MESSAGE_LEN` (change `bound-image-decoding-and-uploads`);
device loss (change `bound-the-compositor-and-survive-device-loss`).

## Follow-ups

- The bound of 256 MiB counts messages other than frame packets only. A guest process that
  sends frame packets without waiting for their ticks is not bounded by the relay: the
  session sets aside one frame while a tick is pending, but the in-process link to the host
  holds the rest. Whether the relay should hand on one frame per tick is open.
