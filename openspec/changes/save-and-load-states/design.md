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

## Limits (stated in the README)

- States live in memory and end with the run.
- Files the game wrote after a state was saved stay as they are; a file a script holds open
  across frames shares its position with the copies.
- Music starts over after a load.
- Each state costs the memory pages the game has changed since.

## Follow-ups

(none yet)
