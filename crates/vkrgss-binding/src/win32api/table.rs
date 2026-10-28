//! The table of emulated functions and the plumbing of a call: the arguments, the lookup
//! and the run of an entry that puts changed buffers back.

use vkrgss_ruby_sys::ffi::VALUE;
use vkrgss_ruby_sys::{Raise, Ruby};

use crate::guest::Guest;
use crate::rb;

use super::input::{
    async_key_state, cursor_pos, key_state, keybd_event, keyboard_state, mouse_event,
    screen_to_client, set_keyboard_state, show_cursor,
};
use super::profile::{copy_file, profile_int, profile_string, write_profile_string};
use super::system::{
    move_memory, process_times, set_priority_class, system_metrics, tick_count, window, window_rect,
};
use super::text::{multi_byte_to_wide_char, wide_char_to_multi_byte};

/// One argument of a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Arg {
    /// An integer, or 0 for `nil`.
    Int(i64),
    /// The bytes of a Ruby string, of fixed length.
    Buffer(Box<[u8]>),
}

/// An emulated function: it reads its arguments, writes into buffers and returns the value
/// of the call.
pub(super) type Shim = fn(&mut Guest, &mut [Arg]) -> i64;

/// The emulated functions: library name in lower case without `.dll`, function name, entry.
pub(super) const TABLE: &[(&str, &str, Shim)] = &[
    ("kernel32", "GetPrivateProfileStringA", profile_string),
    ("kernel32", "GetPrivateProfileIntA", profile_int),
    (
        "kernel32",
        "WritePrivateProfileStringA",
        write_profile_string,
    ),
    ("kernel32", "CopyFileA", copy_file),
    ("kernel32", "SetPriorityClass", set_priority_class),
    ("kernel32", "GetProcessTimes", process_times),
    ("kernel32", "MultiByteToWideChar", multi_byte_to_wide_char),
    ("kernel32", "WideCharToMultiByte", wide_char_to_multi_byte),
    ("kernel32", "GetTickCount", tick_count),
    ("winmm", "timeGetTime", tick_count),
    ("kernel32", "RtlMoveMemory", move_memory),
    ("user32", "GetSystemMetrics", system_metrics),
    ("user32", "FindWindowA", window),
    ("user32", "GetActiveWindow", window),
    ("user32", "GetForegroundWindow", window),
    ("user32", "GetWindowRect", window_rect),
    ("user32", "GetClientRect", window_rect),
    ("user32", "ShowCursor", show_cursor),
    ("user32", "GetAsyncKeyState", async_key_state),
    ("user32", "GetKeyState", key_state),
    ("user32", "GetKeyboardState", keyboard_state),
    ("user32", "SetKeyboardState", set_keyboard_state),
    ("user32", "keybd_event", keybd_event),
    ("user32", "mouse_event", mouse_event),
    ("user32", "GetCursorPos", cursor_pos),
    ("user32", "ScreenToClient", screen_to_client),
];

/// The entry for a function of a library. The library name matches without regard to letter
/// case and with or without `.dll`; like the original `Win32API`, a function name also finds
/// the entry of its ANSI variant, whose name ends in `A`.
pub(super) fn find(library: &str, function: &str) -> Option<Shim> {
    let library = library.trim().to_ascii_lowercase();
    let library = library.strip_suffix(".dll").unwrap_or(&library);
    let entry = |name: &str| {
        TABLE
            .iter()
            .find(|(in_library, entry, _)| *in_library == library && *entry == name)
            .map(|&(_, _, shim)| shim)
    };
    entry(function).or_else(|| entry(&format!("{function}A")))
}

/// The integer argument at `index`; 0 for a buffer and for an argument that was not given.
pub(super) fn int(args: &[Arg], index: usize) -> i64 {
    match args.get(index) {
        Some(&Arg::Int(value)) => value,
        _ => 0,
    }
}

/// The buffer argument at `index`, or `None` for an integer such as the null pointer.
pub(super) fn buffer(args: &[Arg], index: usize) -> Option<&[u8]> {
    match args.get(index) {
        Some(Arg::Buffer(bytes)) => Some(bytes),
        _ => None,
    }
}

/// The buffer argument at `index`, to write into.
pub(super) fn buffer_mut(args: &mut [Arg], index: usize) -> Option<&mut [u8]> {
    match args.get_mut(index) {
        Some(Arg::Buffer(bytes)) => Some(bytes),
        _ => None,
    }
}

/// The text argument at `index`: the bytes of a buffer up to its first zero byte.
pub(super) fn c_string(args: &[Arg], index: usize) -> Option<&[u8]> {
    let bytes = buffer(args, index)?;
    let end = bytes.iter().position(|&byte| byte == 0);
    Some(&bytes[..end.unwrap_or(bytes.len())])
}

/// Runs `shim` on `args` and puts every buffer it changed back into the string at the same
/// index of `strings`, which must be live: in RGSS1 and RGSS2 by `String#replace`, which
/// labels the string ASCII-8BIT over Ruby 1.9; in RGSS3 by writing its bytes in place, with
/// the length and the encoding label the string has, as the original runtime of RGSS3
/// leaves it (API report 3.3).
pub(super) fn run_shim(
    ruby: &Ruby,
    shim: Shim,
    strings: &[Option<VALUE>],
    mut args: Vec<Arg>,
    in_place: bool,
) -> Result<i64, Raise> {
    let before = args.clone();
    let result = Guest::with(ruby, |guest| shim(guest, &mut args));
    for ((string, after), before) in strings.iter().zip(&args).zip(&before) {
        if let (Some(string), Arg::Buffer(bytes)) = (*string, after)
            && after != before
        {
            if in_place {
                rb::write_bytes(ruby, string, bytes)?;
                continue;
            }
            let contents = rb::bytes_new(ruby, bytes)?;
            // SAFETY: `string` is live by the condition of this function, and `contents`
            // was just returned and is in a local.
            unsafe { ruby.funcall(string, c"replace", &[contents]) }?;
        }
    }
    Ok(result)
}
