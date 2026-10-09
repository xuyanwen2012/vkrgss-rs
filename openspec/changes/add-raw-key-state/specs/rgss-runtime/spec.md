# Spec Delta

## ADDED Requirements

### Requirement: Keyboard and mouse through Win32API
`GetAsyncKeyState` and `GetKeyState` SHALL return a value with bit 15 set while the given
virtual key is held in the snapshot latched by the last `Input.update`, and
`GetAsyncKeyState` SHALL also set bit 0 when the key was not held in the snapshot before it.
`GetKeyboardState` SHALL fill a 256-byte buffer with 0x80 for each held key.

#### Scenario: Held key
- **WHEN** the latched snapshot has virtual key 0x0D held and `GetAsyncKeyState.call(0x0D) & 0x8000` is evaluated
- **THEN** the result is `0x8000`, and 0 for a key that is not held

#### Scenario: Keyboard state
- **WHEN** `GetKeyboardState` is called with a 256-byte buffer while 0x41 is held
- **THEN** byte 0x41 of the buffer is 0x80 and the others are 0

### Requirement: Pointer through Win32API
`GetCursorPos` SHALL write the pointer position as two 32-bit integers, and `ScreenToClient`
SHALL leave a point unchanged, so that the two together give frame coordinates; without a
pointer the position SHALL be (-1, -1).

#### Scenario: Pointer
- **WHEN** the snapshot's pointer is (100, 50) and `GetCursorPos` is called with an 8-byte buffer
- **THEN** the buffer unpacks with `"ll"` to `[100, 50]`
