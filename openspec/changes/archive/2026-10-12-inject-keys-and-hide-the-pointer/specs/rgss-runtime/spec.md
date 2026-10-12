## MODIFIED Requirements

### Requirement: Keyboard and mouse through Win32API
`GetAsyncKeyState` and `GetKeyState` SHALL return a value with bit 15 set while the given
virtual key is held in the snapshot latched by the last `Input.update`, and
`GetAsyncKeyState` SHALL also set bit 0 when the key was not held in the snapshot before it.
`GetKeyboardState` SHALL fill a 256-byte buffer with 0x80 for each held key. `GetKeyState`
and `GetKeyboardState` SHALL also report in bit 0 a toggle of the key, 0 at the start, that
flips each time a latched snapshot has the key held and the one before had not.

#### Scenario: Held key
- **WHEN** the latched snapshot has virtual key 0x0D held and `GetAsyncKeyState.call(0x0D) & 0x8000` is evaluated
- **THEN** the result is `0x8000`, and 0 for a key that is not held

#### Scenario: Keyboard state
- **WHEN** `GetKeyboardState` is called with a 256-byte buffer while 0x41 is held for the first time
- **THEN** byte 0x41 of the buffer is 0x81 and the others are 0

#### Scenario: Toggle
- **WHEN** key 0x14 is held in one latched snapshot, released in the next and held again in a third
- **THEN** `GetKeyState.call(0x14) & 1` is 1 after the first, 1 after the second and 0 after the third

### Requirement: Key injection functions are accepted
`keybd_event` SHALL send the host a press of the given virtual key, or a release when bit 1
of its flags is set. `mouse_event` SHALL send a press or release of key 0x01, 0x02 or 0x04
for each button flag it is given. Both SHALL return 0. `SetKeyboardState` with a 256-byte
buffer SHALL replace what `GetKeyState` and `GetKeyboardState` report, the held bits until
the next `Input.update`, and SHALL return 1. None SHALL be reported as not emulated.

#### Scenario: keybd_event
- **WHEN** `keybd_event` is called with `(0x0D, 0, 0, 0)` and then with `(0x0D, 0, 2, 0)`
- **THEN** the guest has sent a press and then a release of key 0x0D, and nothing is written to the error stream

#### Scenario: mouse_event
- **WHEN** `mouse_event` is called with the flags `0x02 | 0x10`
- **THEN** the guest has sent a press of key 0x01 and a release of key 0x02

#### Scenario: SetKeyboardState
- **WHEN** `SetKeyboardState` is called with a buffer whose byte 0x41 is 0x80 and the others 0, while key 0x10 is held
- **THEN** `GetKeyState.call(0x41)` has bit 15 set and `GetKeyState.call(0x10)` is 0, until `Input.update` is called, after which it is the other way round

## ADDED Requirements

### Requirement: Pointer visibility through Win32API
`ShowCursor` SHALL keep its display count and return it, and SHALL tell the host that the
pointer is to be hidden when the count falls below 0 and that it is to be seen when it
comes back to 0.

#### Scenario: Hide and show
- **WHEN** `ShowCursor.call(0)` is called twice and then `ShowCursor.call(1)` twice
- **THEN** the returns are -1, -2, -1 and 0, and the guest has sent exactly two messages: hidden after the first call, to be seen after the last
