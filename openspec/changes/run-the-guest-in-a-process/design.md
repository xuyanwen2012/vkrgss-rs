# Design

## Context

- `vkxp_proto::link` is the in-process transport: `GuestEnd` and `HostEnd` over channels,
  with a wake hook. Both runners of `vkxp-host` take the guest as a closure
  `FnOnce(GuestEnd)` (windowed) or are handed a `HostLink` (headless).
- `vkxp_proto::codec` encodes a message with the protocol version in front.
- `crates/vkxp/src/main.rs` builds the guest's `Config`, starts the interpreter and runs
  `run_game(ruby, config, link)` on it.
- The guest already ends by itself when its link reports `Disconnected`.

## Decisions

### The runners do not change: the parent relays

In the process form the "guest" the runners see is a relay in the parent. It owns the
`GuestEnd` of the ordinary in-process link and a connection to the guest process: one thread
reads framed `GuestToHost` messages from the socket and sends them into the link, the other
takes `HostToGuest` messages from the link and writes them to the socket. `Session`, the
runners and the displays are untouched, and so are their tests.

When the connection ends the relay drops its `GuestEnd`, which the host sees as the guest
having gone; when the link ends (the host is done) the relay closes the connection.

### The guest process uses the socket directly

In the guest process the script thread reads and writes the socket itself: `GuestEnd` gets a
second transport, a Unix stream socket, with the same three operations (`send`, blocking
`recv`, `try_recv`). No helper thread exists in the guest process for the link. The later
save-state change forks this process, and a forked copy keeps only the thread that forked.

### Framing

A message on the stream is a length (u32, little endian) followed by that many bytes, which
are the message as `vkxp_proto::codec` encodes it, version included. A length above a fixed
bound (large enough for a frame packet that carries every pixel of a few screen-sized
bitmaps; the bound is a constant in the framing module) and a message that does not decode
are errors that end the connection. The framing functions take `Read` and `Write` and know
nothing of sockets, so they are tested on byte buffers.

### Starting the guest process

The parent creates a directory only the user can enter (below `$XDG_RUNTIME_DIR`, else
below the system's temporary directory), listens on a Unix stream socket in it, and starts
`std::env::current_exe()` again with its own arguments plus a hidden option that names the
socket and carries the seed the parent settled on. The parent accepts one connection, with a
time limit, and removes the directory when the run ends.

In the guest role the program settles its configuration from the same arguments and files as
the parent, does everything the guest side does today (game files, fonts, interpreter,
`run_game`), and nothing of the host side: no window, no Vulkan, no audio device, no input
recording or replay file. Its diagnostics go to the inherited error stream, as the game's
script errors do.

The guest process asks the kernel to end it when the parent dies (`PR_SET_PDEATHSIG`), so a
game caught in a script loop does not outlive a parent that was killed.

Listening on a socket, where a pair of connected sockets would do for one guest, is chosen
because the save-state change needs further processes to connect later.

### End of the run

The guest process exits with status 0 when the game ended without a failure and with a
non-zero status otherwise, after its last message. The parent waits for it as it waits for
the guest thread today (`GUEST_WAIT`), ends it when it does not exit in that time, and
reports a guest process that ended by a signal with the signal's name. The exit status of
`vkxp` follows the rules of `game-launch` "Exit status" in both forms.

With `--frame-stats` the rows that describe the guest's own time are written by the process
that measured them, the guest process, to the same error stream.

### Selecting the form

`--guest thread|process` on the command line, `guest = "thread"` or `"process"` in a
configuration file, `VKXP_GUEST` in the environment; the command line wins over the
environment, the environment over the files. Default `thread`. Another value is an error
that names the two.

## Risks

- A bitmap that changes is copied through the socket instead of moved through a channel.
  The frame time has a budget of 25 ms and measures under 3 ms today; task 3.2 measures the
  two forms side by side. Shared memory is not built here.
- Two processes write to one error stream; lines are written whole, so they interleave by
  line.

## Notes from the implementation

- The relay serves the two directions of one `GuestEnd` on two threads, and the host only
  sees the guest as gone when the sending direction is dropped while the other may still
  wait. `GuestEnd::split` in `vkxp_proto::link` gives the two directions (`GuestSender`,
  `GuestReceiver`) for that; the guest process does not use it.
- The receive that does not wait puts the socket into non-blocking mode for the time of the
  call, because `vkxp-proto` forbids `unsafe` and the standard library has no other way.
  The guest process uses its end from one thread, so no send meets that mode.
- The bound of the framing is 1 GiB. A reader allocates as bytes arrive, not what a length
  states.
- The hidden option is `--guest-role SEED=SOCKET`.
- The parent looks up what the guest needs (`Game.ini`, run-time packages, the built-in
  scripts file) before it starts the child, so those errors are reported once and before a
  window opens, as in the thread form. The child connects once the game's files are open and
  the interpreter runs; the parent's time limit for that is 30 seconds, and a child that
  exits before it connects ends the wait at once.
- The parent starts the child on the main thread: the parent-death signal follows the thread
  that started the process, and the windowed runner runs the relay on a thread of its own.
- `--frame-stats` needed no change in `vkxp-binding`: the guest already writes its own rows
  to the error stream of the process it runs in.
- The end-to-end tests set `XDG_RUNTIME_DIR` to their work directory, as they do with the
  other XDG directories.

## Checked

Headless runs of 600 frames on llvmpipe with `--frame-stats`, of two games generated for the
measurement (kept under `local/`, not committed), release build, mean times in milliseconds.
`full` fills a bitmap of 640 by 480 pixels with a new colour every frame, so every frame
carries 1.2 MiB of pixels; `small` moves a sprite of 32 by 32 pixels and carries none.

| Game | Form | `guest` (host's row) | `guest extraction` | `guest scripts` | `guest bitmap operations` | `submit` |
|---|---|---|---|---|---|---|
| `full` | thread | 0.039 | 0.021 | 0.002 | 0.013 | 1.120 |
| `full` | process | 1.542 | 0.826 | 0.003 | 0.014 | 1.138 |
| `small` | thread | 0.007 | 0.001 | 0.001 | 0.000 | 0.644 |
| `small` | process | 0.011 | 0.001 | 0.001 | 0.000 | 0.638 |

The largest `guest` time of `full` was 0.428 ms as a thread and 2.276 ms as a process. A
frame that replaces every pixel of the screen costs about 1.5 ms more in a process, against
the budget of 25 ms; a frame without bitmap changes costs a few microseconds more.

The development build (`cargo build` without `--release`) is slower at the same thing: `full`
as a process has a `guest` mean of 8.461 ms there (0.092 ms as a thread), nearly all of it in
`guest extraction` (7.558 ms), where the frame is encoded. Encoding that frame packet alone
takes 7.2 ms in the development build and 0.37 ms in the release build.

Not run: the process form in a window. The end-to-end tests and the measurement are headless;
the windowed runner is given the same relay as its guest.

## Follow-ups

- The pixels of a bitmap change are encoded byte by byte through `serde`. Encoding them as
  one byte string (the bytes on the stream stay the same in `postcard`) would take most of
  the 7 ms of a development build and part of the 0.8 ms of a release build away. It changes
  `vkxp-proto`'s frame types or adds a dependency, which this change does not name.
- When `$XDG_RUNTIME_DIR` is set but cannot be written to, the run with `--guest process`
  ends with an error that names the directory. Whether it should go on to the temporary
  directory instead is open; the design names that directory only for an unset variable.
