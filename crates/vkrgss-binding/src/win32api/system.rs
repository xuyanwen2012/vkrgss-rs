//! The functions of the window, the screen metrics, the ticks, the process and memory.

use crate::guest::Guest;

use super::table::{Arg, buffer, buffer_mut, int};

/// The handle every window function gives for the game window.
const WINDOW: i64 = 0x0001_0001;

/// `GetTickCount()` and `timeGetTime()`: the milliseconds of the frames shown so far, in 32
/// bits.
pub(super) fn tick_count(guest: &mut Guest, _args: &mut [Arg]) -> i64 {
    i64::from(guest.win32api.milliseconds as u32)
}

/// `GetSystemMetrics(index)`: the logical size for the width (0) and the height (1) of the
/// screen, 0 for every other index.
pub(super) fn system_metrics(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let size = guest.scene.logical_size();
    match int(args, 0) {
        0 => i64::from(size.width),
        1 => i64::from(size.height),
        _ => 0,
    }
}

/// `FindWindowA(class, title)`, `GetActiveWindow()` and `GetForegroundWindow()`: the game
/// window, whatever is asked for.
pub(super) fn window(_guest: &mut Guest, _args: &mut [Arg]) -> i64 {
    WINDOW
}

/// `GetWindowRect(window, buffer)` and `GetClientRect(window, buffer)`: the rectangle of the
/// logical size at the origin, as left, top, right and bottom in 32 bits each.
pub(super) fn window_rect(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let size = guest.scene.logical_size();
    if int(args, 0) != WINDOW {
        return 0;
    }
    let Some(rect) = buffer_mut(args, 1).and_then(|buffer| buffer.get_mut(..16)) else {
        return 0;
    };
    let sides = rect.as_chunks_mut::<4>().0.iter_mut();
    for (side, value) in sides.zip([0, 0, size.width, size.height]) {
        *side = value.to_le_bytes();
    }
    1
}

/// `SetPriorityClass(process, priority)`: nothing is done, and that succeeds.
pub(super) fn set_priority_class(_guest: &mut Guest, _args: &mut [Arg]) -> i64 {
    1
}

/// `GetProcessTimes(process, creation, exit, kernel, user)`: every time is 0, in 8 bytes
/// each.
pub(super) fn process_times(_guest: &mut Guest, args: &mut [Arg]) -> i64 {
    for index in 1..=4 {
        if let Some(buffer) = buffer_mut(args, index) {
            let length = buffer.len().min(8);
            buffer[..length].fill(0);
        }
    }
    1
}

// --- Memory -----------------------------------------------------------------------------------

/// `RtlMoveMemory(destination, source, length)` between two strings. An address given as an
/// integer names memory the guest does not have, and nothing is copied.
pub(super) fn move_memory(_guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(source) = buffer(args, 1).map(<[u8]>::to_vec) else {
        return 0;
    };
    let length = usize::try_from(int(args, 2)).unwrap_or(0);
    if let Some(destination) = buffer_mut(args, 0) {
        let length = length.min(source.len()).min(destination.len());
        destination[..length].copy_from_slice(&source[..length]);
    }
    0
}
