# Proposal

## Why

Many RPG Maker XP games call a handful of Windows functions through `Win32API`: they read
`Game.ini` with `GetPrivateProfileString`, convert text between code pages, ask for the screen
size, the tick count or the game window. vkxp answers every call with 0, which makes such
scripts misbehave or crash later. Milestone M6 asks for shims.

## What Changes

- A table of emulated functions behind `Win32API`: profile strings (`GetPrivateProfileStringA`,
  `GetPrivateProfileIntA`, `WritePrivateProfileStringA`), text conversion
  (`MultiByteToWideChar`, `WideCharToMultiByte`), time (`GetTickCount`, `timeGetTime`), system
  metrics (`GetSystemMetrics` for the screen size), window queries that return a stable fake
  handle (`FindWindowA`, `GetActiveWindow`, `GetForegroundWindow`, `GetWindowRect`,
  `GetClientRect`), `ShowCursor`, and memory copies (`RtlMoveMemory`).
- Buffer arguments (`p`) are written in place, as scripts expect.
- Functions outside the table keep the present behaviour: return 0 and report once.

Out of scope: keyboard and mouse state (`GetAsyncKeyState`, `GetKeyState`,
`GetKeyboardState`, `GetCursorPos`), which need the host's raw input and come with a protocol
change; anything that would touch the real system.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: emulated `Win32API` functions.

## Impact

- Crate `vkxp-binding` only (a new module and the `Win32API` class).
