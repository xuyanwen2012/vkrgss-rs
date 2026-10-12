# Design

## Context

- `crates/vkxp-binding/src/win32api.rs` has the shims; `guest.win32api.cursor` is the
  display count of `ShowCursor`. The guest reads keys from the snapshot that `Input.update`
  latched (`is_key_down`, `is_key_new`).
- The host builds each snapshot from the physical keys (`raw.rs` gives a key its Windows
  virtual-key codes, the key map its buttons) and the session sends it ahead of a tick. The
  driver answers a key with a `KeyAction`; the window loop acts on `ToggleFullscreen`
  (Alt+Enter and F11). A session with a replayer delivers recorded snapshots.
- The earlier measurements injected keys into the original runtime with `keybd_event`, and
  `Input.update` of the following frame saw them; nothing else about these four functions
  was measured, and under Wine `user32` is Wine's. The rules below follow the documentation
  of Windows.

## Decisions

### Messages

`GuestToHost::InjectKey { key: u8, down: bool }` and
`GuestToHost::PointerVisible { visible: bool }`. The version becomes 10; whatever carries
the version is regenerated or edited as for version 9. The requirement "State messages"
states the version, so its text changes with it.

### Injected keys

`keybd_event(key, scan, flags, extra)` sends `InjectKey` with `down` false when bit 1 of
`flags` (`KEYEVENTF_KEYUP`) is set and true otherwise; a code above 255 sends nothing.
`mouse_event(flags, ...)` sends one message for each of the button flags it has: left down
0x02 and up 0x04 as key 0x01, right down 0x08 and up 0x10 as key 0x02, middle down 0x20 and
up 0x40 as key 0x04. Movement and the wheel are still ignored. Both return 0.

The session keeps the set of injected keys that are down. A message changes the set and is
handled as a press or release of the physical key that has this code, by the inverse of the
table of `raw.rs` (a code shared by a left and a right key takes the left one; a code no key
has is only held in the raw keys). So an injected key shows in the raw keys and the buttons
of the snapshots from then on, until it is released by a message, and the actions of the
window keys follow: with Alt down, Enter switches full screen. A headless run has no window
and ignores the actions it has no use for, as for physical keys.

The guest sees an injected key in the snapshot of the next tick, which is one
`Input.update` later than on Windows when a script injects and updates in the same frame.

A session that replays a recording ignores `InjectKey`: the recording has the snapshots as
they were delivered, injected keys included. Loading a state and a reset of the game clear
the set.

### Pointer

The pointer is to be seen while the count of `ShowCursor` is 0 or more. `show_cursor` sends
`PointerVisible` when a call changes that. The window loop hides or shows the pointer over
its picture (`Window::set_cursor_visible`); a headless run ignores the message. A guest
that goes on from a loaded state sends its current value after the loaded answer, as it
sends its title.

### `SetKeyboardState` and the toggle bit

The guest keeps, for each of the 256 codes, a toggle bit that flips whenever `Input.update`
latches a snapshot in which the key is down and was not in the one before. It starts at 0:
the state of the lock keys of the machine is not read. `GetKeyState` returns the toggle in
bit 0 beside the sign bit and bit 7, and `GetKeyboardState` in bit 0 of each byte.

`SetKeyboardState(buffer)` with 256 bytes replaces what these two functions report: bit 7
of a byte as the key being down, bit 0 as its toggle. The down bits last until the next
`Input.update`, which reports the latched snapshot again; the toggles stay as set.
`GetAsyncKeyState` and `Input` are not affected. It returns 1, and 0 for a shorter buffer.

The comment of each of these keeps a `PLACEHOLDER` line where the rule is from the
documentation and not measured; the ones for "nothing is done" go.

## Risks

- [A script holds a key down by injection and never releases it] → as on Windows.
- [The version bump invalidates streams of version 9] → only fixtures store streams.

## Follow-ups

None yet.
