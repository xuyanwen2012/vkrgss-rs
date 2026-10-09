# Spec Delta

## ADDED Requirements

### Requirement: Key injection functions are accepted
`keybd_event`, `SetKeyboardState` and `mouse_event` called through `Win32API` SHALL do nothing,
return 0 for `keybd_event` and `mouse_event` and 1 for `SetKeyboardState`, and SHALL NOT be
reported as not emulated.

#### Scenario: keybd_event
- **WHEN** `Win32API.new("user32.dll", "keybd_event", "iiii", "v").call(0x0D, 0, 0, 0)` is evaluated
- **THEN** nothing is written to the error stream and the raw key state is unchanged
