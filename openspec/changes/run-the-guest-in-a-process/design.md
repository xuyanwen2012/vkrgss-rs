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

## Follow-ups

(none yet)
