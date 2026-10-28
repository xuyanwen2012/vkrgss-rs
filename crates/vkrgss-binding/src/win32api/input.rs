//! The keyboard, mouse and pointer functions.

use vkrgss_proto::GuestToHost;

use crate::guest::Guest;

use super::KeySet;
use super::table::{Arg, buffer, buffer_mut, int};

/// `ShowCursor(show)`: the new display count. The pointer is to be seen while the count is
/// 0 or more, and the host is told when a call changes that.
// PLACEHOLDER: the count follows the documentation of Windows and was not measured on the
// originals of XP and VX. Its start, 0, was measured on the original of VX Ace:
// `ShowCursor(0)` gives -1 at first (oracle measurement 2026-10-13, RGSS301 3.0.1.1, API
// report 5.8).
pub(super) fn show_cursor(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let before = guest.win32api.pointer_visible();
    guest.win32api.cursor += if int(args, 0) != 0 { 1 } else { -1 };
    let visible = guest.win32api.pointer_visible();
    if visible != before {
        guest.send(GuestToHost::PointerVisible { visible });
    }
    guest.win32api.cursor
}

/// The virtual-key code argument at `index`, when it is one of the 256.
fn virtual_key(args: &[Arg], index: usize) -> Option<u8> {
    u8::try_from(int(args, index)).ok()
}

/// `GetAsyncKeyState(key)`: bit 15 while the key is held in the snapshot `Input.update`
/// latched, and bit 0 too when it was not held in the one before. The value is a 16-bit
/// integer with its sign, as on Windows.
pub(super) fn async_key_state(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    match virtual_key(args, 0) {
        Some(key) if guest.input.is_key_down(key) => {
            i64::from(i16::MIN | i16::from(guest.input.is_key_new(key)))
        }
        _ => 0,
    }
}

/// Whether `GetKeyState` and `GetKeyboardState` report `key` as down: as the snapshot
/// `Input.update` latched has it, or as `SetKeyboardState` said since.
fn reported_down(guest: &Guest, key: u8) -> bool {
    match &guest.win32api.set_down {
        Some(down) => down.has(key),
        None => guest.input.is_key_down(key),
    }
}

/// `GetKeyState(key)`: the sign bit and bit 7 while the key is held and the toggle of the key
/// in bit 0, in a 16-bit integer with its sign, as on Windows.
// PLACEHOLDER: the toggle flips when `Input.update` latches a press, by the documentation of
// Windows, and starts at 0: the state of the lock keys of the machine is not read.
pub(super) fn key_state(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(key) = virtual_key(args, 0) else {
        return 0;
    };
    let down = if reported_down(guest, key) { -0x80 } else { 0 };
    down | i64::from(guest.win32api.toggles.has(key))
}

/// `GetKeyboardState(buffer)`: in one byte per virtual-key code, 0x80 for a held key and the
/// toggle of the key in bit 0.
pub(super) fn keyboard_state(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(buffer) = buffer_mut(args, 0) else {
        return 0;
    };
    for (key, state) in (0..=255u8).zip(buffer.iter_mut()) {
        let down = if reported_down(guest, key) { 0x80 } else { 0 };
        *state = down | u8::from(guest.win32api.toggles.has(key));
    }
    1
}

/// `keybd_event(key, scan, flags, extra)`: the host is told that the key is pressed for the
/// player, or released when bit 1 of the flags (`KEYEVENTF_KEYUP`) is set. A code above 255
/// sends nothing.
// PLACEHOLDER: the guest sees the key in the snapshot of the next tick, one `Input.update`
// later than on Windows when a script injects and updates in the same frame.
pub(super) fn keybd_event(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    if let Some(key) = virtual_key(args, 0) {
        let down = int(args, 2) & 0x02 == 0;
        guest.send(GuestToHost::InjectKey { key, down });
    }
    0
}

/// The button flags of `mouse_event`: the flag, the virtual key of its button and whether
/// the button goes down.
const MOUSE_BUTTON_FLAGS: [(i64, u8, bool); 6] = [
    (0x02, 0x01, true),
    (0x04, 0x01, false),
    (0x08, 0x02, true),
    (0x10, 0x02, false),
    (0x20, 0x04, true),
    (0x40, 0x04, false),
];

/// `mouse_event(flags, dx, dy, data, extra)`: the host is told of a press or release for
/// each button flag, as of a key.
// PLACEHOLDER: movement and the wheel are ignored.
pub(super) fn mouse_event(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let flags = int(args, 0);
    for (flag, key, down) in MOUSE_BUTTON_FLAGS {
        if flags & flag != 0 {
            guest.send(GuestToHost::InjectKey { key, down });
        }
    }
    0
}

/// `SetKeyboardState(buffer)`: the 256 bytes replace what `GetKeyState` and
/// `GetKeyboardState` report, bit 7 of a byte as the key being down, until the next
/// `Input.update`, and bit 0 as its toggle. `GetAsyncKeyState` and `Input` are not affected.
/// A shorter buffer changes nothing and fails.
// PLACEHOLDER: by the documentation of Windows, not measured on the original.
pub(super) fn set_keyboard_state(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(states) = buffer(args, 0).and_then(|buffer| buffer.get(..256)) else {
        return 0;
    };
    let mut down = KeySet::default();
    for (key, state) in (0..=255u8).zip(states) {
        down.set(key, state & 0x80 != 0);
        guest.win32api.toggles.set(key, state & 1 != 0);
    }
    guest.win32api.set_down = Some(down);
    1
}

/// `GetCursorPos(buffer)`: the pointer position in the frame as x and y in 32 bits each,
/// (-1, -1) when the pointer is not over the picture.
pub(super) fn cursor_pos(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let (x, y) = guest.input.pointer().unwrap_or((-1, -1));
    let Some(point) = buffer_mut(args, 0).and_then(|buffer| buffer.get_mut(..8)) else {
        return 0;
    };
    let coordinates = point.as_chunks_mut::<4>().0.iter_mut();
    for (coordinate, value) in coordinates.zip([x, y]) {
        *coordinate = i32::from(value).to_le_bytes();
    }
    1
}

/// `ScreenToClient(window, buffer)`: the point stays what it is, since the pointer position
/// is in the frame already.
pub(super) fn screen_to_client(_guest: &mut Guest, _args: &mut [Arg]) -> i64 {
    1
}
