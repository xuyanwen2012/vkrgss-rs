# Design

## Context

- `crates/vkxp/src/process.rs` starts the guest process, which connects to a Unix socket the
  parent listens on, and relays between that connection and the in-process link of the
  runners. The guest's script thread uses the socket directly (`GuestEnd::over_stream`).
- The guest pumps host messages in `Guest::receive` (`crates/vkxp-binding/src/runtime.rs`);
  a wait for a tick is a frame boundary. It remembers what the music and background sound
  channels play (`runtime.music`, `runtime.background_sound`).
- The guest sends bitmap and table changes as deltas against what it has sent before.
- The guest is deterministic: the same state and the same input give the same frames.

## Decisions

### A state is a parked copy of the guest process

    host -> running guest: SaveState { slot }
    running guest, at its next frame boundary where it may (see below): fork()
        copy:    closes the inherited connection, connects to the host's socket,
                 sends Attach { pid, parked: Some(slot) }, then waits for a message
        itself:  goes on as if nothing happened
    host, on that Attach: keeps the connection as the state of the slot (closing the one the
                 slot had, whose process sees the end of its connection and exits), and
                 tells the session StateSaved { slot }

    session -> relay: LoadState { slot }       (never reaches a guest)
    relay, when the slot has a parked copy:
        ends the running guest's process with SIGKILL (its pid is in its Attach) and drops
        its connection; sends Resume on the parked connection, which is the running one from
        now on; tells the session StateLoaded { slot }
    parked copy, on Resume: fork()
        copy:    parks again for the same slot, as above
        itself:  resumes (next section)
    relay, when the slot is empty: tells the session StateFailed { slot }

The running guest is killed, not asked to quit: nothing of the game may run after the
decision to load, neither scripts after the main loop nor `ensure` clauses that write files.

A parked copy that reads the end of its connection calls `_exit(0)` at once, without
unwinding, interpreter shutdown or buffers flushed. That is how a replaced state, and every
state when `vkxp` ends, goes away. `vkxp` also ends every process whose `Attach` it holds
when it exits, so a resumed guest caught in a script loop does not stay behind.

Every guest process, the first included, opens its connection with `Attach`; the first has
`parked: None`.

### Where the fork happens

`fork` is called on the script thread, in the wait for a tick, when the frame of that wait
was sent while the display was not frozen and no modal message is shown; a request that
arrives at another time waits for the first such frame. (A copy made between a freeze and
its transition could not show the frozen picture again.)

Before the fork the C library's output streams are flushed, so that buffered output is not
written twice. In the copy only the forking thread exists. The guest process has one other
thread, which only waits for the script thread and holds no lock while it does; the copy
never returns to it: when its game ends it reports the outcome on its connection
(`Outcome { ok }`, which the relay keeps for the exit status and does not hand on) and exits
from the script thread. The first guest process reports its outcome the same way, so the
exit status of `vkxp` does not depend on which process ended the game.

Guest processes ignore `SIGCHLD`, so the copies they forked leave no zombies, and a copy
clears the parent-death signal it inherited: its parent is a guest process that a load ends
on purpose. With `--allow-process-functions` the original disposition of `SIGCHLD` is needed
by `system` and the like; with that option states are not offered (the save request is
answered with `StateFailed`).

The fork and the calls around it live in `crates/vkxp` (platform code, `libc`). The binding
gets them through a trait object in its run configuration (park for slot N: returns
"went on" in the process that asked and, in a copy that was resumed, the new link), so
`vkxp-binding` contains no process code and its tests give a fake.

### Resuming

A resumed copy holds the game exactly as it was in the wait for the tick. The host holds
nothing of it: the session, on `StateLoaded`, makes the display forget every bitmap and
table, ends a transition or freeze, drops a modal message, stops all four audio channels,
restarts the pacing as after a frame reset and sends the current input again. The resumed
guest then

1. marks every live bitmap and table as not yet sent, so the next packet carries them whole,
2. sends the title, and the play commands of the music and the background sound it
   remembers (they start from their beginning),
3. sends the frame of the interrupted wait again and waits for its tick as usual.

From there the run continues as any other. The frame counter of the session keeps counting
(dumps and `--frames` of a headless run are in host frames); `Graphics.frame_count` is part
of the game and comes back with the state.

### Session, keys, notices

Ten slots, 0 to 9, slot 0 chosen at start. F3 sends `SaveState` for the chosen slot, F4
`LoadState`; with Shift they choose the next and the previous slot (wrapping). The keys hold
no button of the game, like the other host keys. While a modal message is shown they do
nothing. The answer (`StateSaved`, `StateLoaded`, `StateFailed`) and a change of slot are
shown in the window title for two seconds, the way the frame rate is shown, and logged at
the level `info`.

The keys answer with a notice, and send nothing, when the guest is a thread, and when the
run records or replays input (a load would make the recording describe another run).

### Headless schedule

`--states "120:save 0, 200:load 0"`: at the host frame before the colon the session is
given the request, as if the key had been pressed. It exists for the tests. With the guest
as a thread it is an error at start that names `--guest process`.

### Default form

`--guest` keeps its values. Without a choice the guest is a process in a window and a thread
in a headless run.

## Notes from the implementation

- **When a request is served.** The guest notes a request when it arrives and has itself
  copied when it next *enters* a wait for a tick that qualifies, right after sending the
  frame of that wait. A request that arrives during the wait of frame 60 is so served in
  the wait of frame 61, and the copy sends frame 61 again when it is resumed. With the
  requests of a headless schedule given when their frame arrives, ahead of its tick like the
  input script's entries, this is what makes frame 100 + k of the scenario "Back to the
  saved frame" equal frame 60 + k.
- **No tick while a load is asked for.** From `LoadState` to its answer the session sends no
  tick (and drops a second `LoadState`). A tick sent in between would answer a frame of the
  guest that the load ends and reach the resumed copy, which would take it for the tick of
  the frame it sends again. On `StateLoaded` the pending tick is dropped.
- **Order at the relay.** A guest process that forks waits, through a pipe, until its copy
  has written its `Attach`, and the relay takes the processes that have connected before it
  reads a message of the running guest. So the relay knows a copy before anything its maker
  sent after making it, and the copy a resumed guest leaves of itself (which is not
  announced with `StateSaved`) fills the slot before that guest's first frame.
- **The outcome.** The binding sends `Outcome` before `Shutdown` in every form of the guest;
  a session ignores it. So that a guest whose host has stopped sending can still report it,
  the guest keeps its link when it reads the end of it and only takes that as a request to
  quit, and the relay closes just its sending side of the connection when the host has
  gone. The relay then waits for the guest process to end, as long as the program waited
  for it before.
- **Ending processes.** The relay ends the processes whose connections it holds, and not a
  process whose connection has ended: its number may belong to another process by then. A
  copy that is replaced in its slot exits by itself on the end of its connection. The relay
  waits up to two seconds for all of them to be gone.
- **The interpreter's thread timer.** Interval timers are not inherited through `fork`, so a
  copy calls `rb_thread_start_timer`, as the interpreter does after its own `fork`.
- **Edits outside the named files, a few lines each.** `crates/vkxp-core/src/bitmap.rs`: an
  iterator over the live bitmaps, which `delta.rs` cannot reach otherwise (the table
  bookkeeping already had `Scene::resend_tables`). `crates/vkxp-vk`: `forget_resources` of
  the compositor, built from the deltas that destroy each bitmap and table, and the two
  iterators over their ids. `crates/vkxp-host/src/report.rs` and `crates/vkxp/src/log.rs`:
  the severity `Info` for the notices. `crates/vkxp-binding/src/graphics.rs`: the flag that
  the display holds a picture.

## Limits (stated in the README)

- States live in memory and end with the run.
- Files the game wrote after a state was saved stay as they are; a file a script holds open
  across frames shares its position with the copies.
- Music starts over after a load.
- Each state costs the memory pages the game has changed since.

## Follow-ups

- A notice leaves the title when the session next looks at its clock, which is at the next
  tick. A game that sends no frame for a while (a script that loads for seconds) keeps the
  notice that long. A timer of its own for the notice was not built.
- The time a save and a load take has not been measured; the proposal's "a few
  milliseconds" is an expectation.
- The state keys were exercised through the session's tests and the headless schedule, not
  by a person in a window.
- `StateFailed` does not say whether it answers a save or a load. The session takes it for
  the answer to the load it waits for when the slot is the same, which is wrong for a save
  into that very slot that fails at the same moment.
- A state whose copy died (for example ended by the kernel for lack of memory) is found out
  when it is loaded and answered with `StateFailed`; nothing reports it earlier.
