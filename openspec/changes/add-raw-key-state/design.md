# Design

## Context

- `vkxp_proto::InputSnapshot` is `{ buttons: u32 }`; the session builds one per frame from the
  key map and gamepads, records and replays them; the guest's input state latches the newest
  snapshot at `Input.update`.
- `crates/vkxp-binding/src/win32api.rs` has the table of emulated functions; unknown pairs
  return 0 with one report.
- Windows virtual-key codes: letters and digits are their ASCII codes, `VK_SHIFT` 0x10 with
  `VK_LSHIFT` 0xA0 and `VK_RSHIFT` 0xA1 (likewise control 0x11, 0xA2, 0xA3 and menu 0x12, 0xA4,
  0xA5), arrows 0x25 to 0x28, function keys 0x70 upwards, numeric pad 0x60 to 0x69, mouse
  buttons 0x01, 0x02, 0x04.
- A game's client area is the frame, so screen and client coordinates are treated as the same
  logical coordinates.

## Decisions

- `InputSnapshot` gains `keys: [u64; 4]` and `pointer: Option<(i16, i16)>`; version 5.
- The host translates physical key codes of the windowing library with one table, tested by
  itself; the pointer goes through the inverse of the letterbox mapping.
- Recording format version 2 adds the held virtual keys (hexadecimal list) and the pointer to
  each run-length line; the reader accepts versions 1 and 2.
- The guest keeps the previous latched snapshot too, for bit 0 of `GetAsyncKeyState`.
- The oracle harness's probe replaces `Input` only; raw keys are not scripted there.

## Follow-ups

- `GetKeyState`'s toggle bit for lock keys; mouse wheel; text input.
