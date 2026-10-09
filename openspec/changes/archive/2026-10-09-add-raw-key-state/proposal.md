# Proposal

## Why

Many RPG Maker XP games read the keyboard and the mouse directly through `Win32API`
(`GetAsyncKeyState`, `GetKeyState`, `GetKeyboardState`, `GetCursorPos`, `ScreenToClient`):
full-keyboard input scripts, mouse scripts, hot keys. The second test game calls
`GetAsyncKeyState`. vkxp answers 0, because the guest only receives the twenty RGSS buttons.

## What Changes

- **BREAKING** (protocol version 5): the input snapshot also carries the state of the 256
  Windows virtual keys, including the mouse buttons, and the pointer position in logical
  frame coordinates when it is over the picture.
- The host fills them from the keyboard and mouse of the window; headless runs and input
  scripts leave them empty; recordings store and replay them (format version 2, version 1
  files still load).
- `Win32API` emulates `GetAsyncKeyState`, `GetKeyState`, `GetKeyboardState`, `GetCursorPos`
  and `ScreenToClient` from the snapshot latched at `Input.update`.

Out of scope: text input, key names other than the Windows virtual-key codes, a mouse cursor
drawn by vkxp.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: raw key and pointer state, version 5.
- `host-session`: filling and recording it.
- `rgss-runtime`: the emulated functions.

## Impact

- Crates `vkxp-proto`, `vkxp-host` (key translation, recorder), `vkxp-core` (input state keeps
  the snapshot), `vkxp-binding` (`win32api.rs`).
