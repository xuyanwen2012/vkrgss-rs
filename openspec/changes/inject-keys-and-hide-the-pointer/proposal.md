# Inject keys and hide the pointer

## Why

Four `Win32API` shims are accepted and do nothing, and are marked `PLACEHOLDER`:

- `keybd_event` and `mouse_event`. Scripts of the time use `keybd_event` to press Alt+Enter
  for the player, so that the game starts in full screen, and some press a key to read it
  back. In vkxp nothing happens.
- `SetKeyboardState`, which scripts use to clear or set the state that `GetKeyState` and
  `GetKeyboardState` report.
- `ShowCursor`, which mouse scripts call to hide the system pointer under their own cursor
  sprite. vkxp keeps the count and leaves the pointer visible, so two pointers are drawn.
- `GetKeyState` never reports the toggle bit of a key.

## What Changes

- **BREAKING** (protocol): two messages from the guest, a key that is pressed or released
  for the player and whether the pointer is to be seen; the protocol version becomes 10.
- `keybd_event` and the button flags of `mouse_event` send the first. The host treats such a
  key as it treats the physical key: it is held in the raw keys, presses the buttons the key
  map gives it, and Alt+Enter switches full screen.
- `ShowCursor` sends the second when the count crosses zero, and the window hides or shows
  the pointer over the picture.
- `SetKeyboardState` sets what `GetKeyState` and `GetKeyboardState` report until the next
  `Input.update`.
- `GetKeyState` reports in bit 0 a toggle that flips at each press.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: the shims.
- `guest-host-protocol`: the two messages; version 10.
- `host-session`: what the session does with them.

## Impact

`vkxp-proto` (messages, codec, fixtures that carry the version), `vkxp-binding`
(`win32api.rs`, the guest's input state), `vkxp-host` (session, window loop, test doubles),
`vkxp` and `vkxp-compose-ref` where a message is matched, `docs/compatibility.md`,
`docs/architecture.md` and `README.md` for the version. No picture changes.
