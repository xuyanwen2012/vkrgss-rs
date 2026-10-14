//! The emulated functions behind `Win32API`.
//!
//! The prelude's `Win32API#call` passes the library name, the function name and the arguments
//! to the host function `vkxp_win32api`. That looks the pair up in [`TABLE`] and runs the
//! entry on the guest state, or returns `nil` for a function outside the table, which the
//! prelude turns into 0 and one report.
//!
//! An integer argument reaches an entry as an `i64` and `nil` as 0, the null pointer. A Ruby
//! string reaches it as a copy of its bytes that the entry may overwrite but not resize; a
//! copy that was changed is put back into the very string object, so a script finds the
//! result in the buffer it passed.

use std::collections::HashSet;
use std::ffi::{CStr, c_long, c_void};
use std::path::PathBuf;

use encoding_rs::Encoding;
use vkxp_core::InputState;
use vkxp_proto::GuestToHost;
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Arity, HostResult, Raise, Raised, Ruby};
use vkxp_vfs::{FileKind, Runtime};

use crate::guest::Guest;
use crate::rb;

/// One argument of a call.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Arg {
    /// An integer, or 0 for `nil`.
    Int(i64),
    /// The bytes of a Ruby string, of fixed length.
    Buffer(Box<[u8]>),
}

/// An emulated function: it reads its arguments, writes into buffers and returns the value
/// of the call.
type Shim = fn(&mut Guest, &mut [Arg]) -> i64;

/// The emulated functions: library name in lower case without `.dll`, function name, entry.
const TABLE: &[(&str, &str, Shim)] = &[
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

/// The handle every window function gives for the game window.
const WINDOW: i64 = 0x0001_0001;

/// What the emulated functions keep between calls.
#[derive(Debug, Default)]
pub(crate) struct State {
    /// The tick count: the milliseconds of the frames shown so far.
    milliseconds: u64,
    /// What the frames shown so far leave over of a millisecond, in units of one over the
    /// frame rate.
    remainder: u64,
    /// The display count of `ShowCursor`.
    cursor: i64,
    /// The toggle bit of each virtual key, which `GetKeyState` and `GetKeyboardState` report.
    toggles: KeySet,
    /// The keys `SetKeyboardState` said are down, until the next `Input.update`.
    set_down: Option<KeySet>,
    /// The library and function pairs outside the table that a `Win32API` of RGSS3 has
    /// reported.
    reported: HashSet<String>,
}

/// One bit for each of the 256 virtual-key codes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct KeySet([u64; 4]);

impl KeySet {
    fn has(&self, key: u8) -> bool {
        self.0[usize::from(key >> 6)] & (1 << (key & 63)) != 0
    }

    fn set(&mut self, key: u8, on: bool) {
        let bit = 1 << (key & 63);
        let word = &mut self.0[usize::from(key >> 6)];
        *word = if on { *word | bit } else { *word & !bit };
    }
}

impl State {
    /// Advances the tick count by one frame of `frame_rate` frames per second. The count
    /// follows the frames, not the clock, so a run is the same every time; and it follows
    /// the frames shown, not `Graphics.frame_count`, which a script may set back.
    pub(crate) fn frame_shown(&mut self, frame_rate: u64) {
        self.remainder += 1000;
        self.milliseconds += self.remainder / frame_rate.max(1);
        self.remainder %= frame_rate.max(1);
    }

    /// `Input.update` has latched a snapshot into `input`: the toggle of every key that is
    /// down in it and was not in the one before flips, and what `SetKeyboardState` said is
    /// down is over.
    pub(crate) fn input_updated(&mut self, input: &InputState) {
        for key in 0..=255u8 {
            if input.is_key_new(key) {
                self.toggles.set(key, !self.toggles.has(key));
            }
        }
        self.set_down = None;
    }

    /// Whether the pointer is to be seen: while the display count is 0 or more.
    pub(crate) fn pointer_visible(&self) -> bool {
        self.cursor >= 0
    }
}

/// The entry for a function of a library. The library name matches without regard to letter
/// case and with or without `.dll`; like the original `Win32API`, a function name also finds
/// the entry of its ANSI variant, whose name ends in `A`.
fn find(library: &str, function: &str) -> Option<Shim> {
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
fn int(args: &[Arg], index: usize) -> i64 {
    match args.get(index) {
        Some(&Arg::Int(value)) => value,
        _ => 0,
    }
}

/// The buffer argument at `index`, or `None` for an integer such as the null pointer.
fn buffer(args: &[Arg], index: usize) -> Option<&[u8]> {
    match args.get(index) {
        Some(Arg::Buffer(bytes)) => Some(bytes),
        _ => None,
    }
}

/// The buffer argument at `index`, to write into.
fn buffer_mut(args: &mut [Arg], index: usize) -> Option<&mut [u8]> {
    match args.get_mut(index) {
        Some(Arg::Buffer(bytes)) => Some(bytes),
        _ => None,
    }
}

/// The text argument at `index`: the bytes of a buffer up to its first zero byte.
fn c_string(args: &[Arg], index: usize) -> Option<&[u8]> {
    let bytes = buffer(args, index)?;
    let end = bytes.iter().position(|&byte| byte == 0);
    Some(&bytes[..end.unwrap_or(bytes.len())])
}

// --- Profile files ----------------------------------------------------------------------------

/// What a line of a profile file is, read as `vkxp-vfs` reads `Game.ini`.
enum Line<'a> {
    /// `[name]`.
    Section(&'a [u8]),
    /// `key=value`, both without surrounding white space.
    Entry(&'a [u8], &'a [u8]),
    /// A comment, an empty line or anything else.
    Other,
}

fn classify(line: &[u8]) -> Line<'_> {
    let line = line.trim_ascii();
    if line.starts_with(b";") {
        return Line::Other;
    }
    if let Some(rest) = line.strip_prefix(b"[") {
        let end = rest.iter().position(|&byte| byte == b']');
        return Line::Section(rest[..end.unwrap_or(rest.len())].trim_ascii());
    }
    match line.iter().position(|&byte| byte == b'=') {
        Some(at) => Line::Entry(line[..at].trim_ascii(), line[at + 1..].trim_ascii()),
        None => Line::Other,
    }
}

/// A file's byte-order mark, if it has one, and the rest.
fn split_mark(contents: &[u8]) -> (&[u8], &[u8]) {
    const MARK: &[u8] = b"\xEF\xBB\xBF";
    match contents.strip_prefix(MARK) {
        Some(rest) => (MARK, rest),
        None => (b"", contents),
    }
}

/// The value of `key` in `section`, in the file's bytes. Names match without regard to
/// letter case, the first occurrence of a key wins, and a pair of quotes around the value is
/// dropped as Windows drops it.
fn profile_lookup<'a>(contents: &'a [u8], section: &[u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut inside = false;
    for line in split_mark(contents).1.split(|&byte| byte == b'\n') {
        match classify(line) {
            Line::Section(name) => inside = name.eq_ignore_ascii_case(section),
            Line::Entry(name, value) if inside && name.eq_ignore_ascii_case(key) => {
                let quoted = value.len() >= 2
                    && matches!(value[0], b'"' | b'\'')
                    && value[value.len() - 1] == value[0];
                return Some(if quoted {
                    &value[1..value.len() - 1]
                } else {
                    value
                });
            }
            _ => {}
        }
    }
    None
}

/// `contents` with `key` of `section` set to `value`: in place of the value of the key's
/// first entry, or after the last entry of the section, or in a new section at the end.
fn profile_set(contents: &[u8], section: &[u8], key: &[u8], value: &[u8]) -> Vec<u8> {
    let (mark, body) = split_mark(contents);
    let unix = body.contains(&b'\n') && !body.windows(2).any(|pair| pair == b"\r\n");
    let newline: &[u8] = if unix { b"\n" } else { b"\r\n" };
    let entry = |name: &[u8]| [name, b"=", value, newline].concat();
    let lines: Vec<&[u8]> = body.split_inclusive(|&byte| byte == b'\n').collect();
    let mut inside = false;
    let mut replace = None;
    let mut insert = None;
    for (index, line) in lines.iter().enumerate() {
        match classify(line) {
            Line::Section(name) => {
                inside = name.eq_ignore_ascii_case(section);
                if inside {
                    insert = Some(index + 1);
                }
            }
            Line::Entry(name, _) if inside => {
                if name.eq_ignore_ascii_case(key) {
                    replace = Some((index, name));
                    break;
                }
                insert = Some(index + 1);
            }
            _ => {}
        }
    }
    let mut out = mark.to_vec();
    for (index, line) in lines.iter().enumerate() {
        match replace {
            // The key keeps the spelling the file has.
            Some((at, name)) if at == index => out.extend_from_slice(&entry(name)),
            Some(_) => out.extend_from_slice(line),
            None => {
                if insert == Some(index) {
                    out.extend_from_slice(&entry(key));
                }
                out.extend_from_slice(line);
            }
        }
    }
    if replace.is_none() && insert.is_none_or(|at| at >= lines.len()) {
        if body.last().is_some_and(|&byte| byte != b'\n') {
            out.extend_from_slice(newline);
        }
        if insert.is_none() {
            out.extend_from_slice(&[b"[", section, b"]", newline].concat());
        }
        out.extend_from_slice(&entry(key));
    }
    out
}

/// The components of a file name given to a profile function, relative to the game
/// directory, or `None` for a name that leaves it: an absolute path, a drive, or `..` beyond
/// the top.
fn inside_game(name: &[u8]) -> Option<Vec<String>> {
    let name = String::from_utf8_lossy(name);
    if name.starts_with(['/', '\\']) || name.as_bytes().get(1) == Some(&b':') {
        return None;
    }
    let mut components = Vec::new();
    for component in name.split(['/', '\\']) {
        match component {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            name => components.push(name.to_owned()),
        }
    }
    (!components.is_empty()).then_some(components)
}

/// Where the profile file `name` is, or may be created, inside the game directory. An
/// existing file is found without regard to letter case. `None` for a name that leaves the
/// game directory, also through a symbolic link: the game's files pass such links over, and
/// a file is only created where nothing is yet, in a directory that really lies inside.
fn profile_path(guest: &Guest, name: &[u8]) -> Option<PathBuf> {
    let components = inside_game(name)?;
    // Scripts create files through Ruby's own file API, which the cached listings do not see.
    guest.files.refresh();
    let relative = components.join("/");
    if let Ok(existing) = guest.files.resolve(FileKind::Data, &relative) {
        return Some(existing);
    }
    let game_dir = guest.files.game_dir().canonicalize().ok()?;
    let new = guest.files.game_dir().join(relative);
    let parent = new.parent()?.canonicalize().ok()?;
    if !parent.starts_with(&game_dir) {
        return None;
    }
    // An entry of that name which the game's files did not accept is a link that leads
    // outside, also one whose target does not exist yet, or a directory.
    let leaf = new.file_name()?.to_str()?.to_lowercase();
    let taken = std::fs::read_dir(&parent).ok()?.any(|entry| {
        entry.is_ok_and(|entry| entry.file_name().to_string_lossy().to_lowercase() == leaf)
    });
    (!taken).then(|| parent.join(new.file_name().unwrap_or_default()))
}

/// The value the section, key and file name arguments name: the first two arguments and the
/// one at `file`. `None` when the file is missing or outside the game directory, or has no
/// such key.
fn profile_value(guest: &Guest, args: &[Arg], file: usize) -> Option<Vec<u8>> {
    let section = c_string(args, 0)?;
    let key = c_string(args, 1)?;
    let contents = std::fs::read(profile_path(guest, c_string(args, file)?)?).ok()?;
    profile_lookup(&contents, section, key).map(<[u8]>::to_vec)
}

/// `GetPrivateProfileStringA(section, key, default, buffer, size, file)`: the value, or the
/// default, goes into the buffer with a zero byte after it, cut to fit; returns its length.
fn profile_string(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let value = profile_value(guest, args, 5)
        .or_else(|| c_string(args, 2).map(<[u8]>::to_vec))
        .unwrap_or_default();
    let size = usize::try_from(int(args, 4)).unwrap_or(0);
    let Some(buffer) = buffer_mut(args, 3) else {
        return 0;
    };
    let Some(room) = size.min(buffer.len()).checked_sub(1) else {
        return 0;
    };
    let length = value.len().min(room);
    buffer[..length].copy_from_slice(&value[..length]);
    buffer[length] = 0;
    length as i64
}

/// `GetPrivateProfileIntA(section, key, default, file)`: the number the value starts with,
/// 0 for a value that starts with none, the default for a missing key.
fn profile_int(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(value) = profile_value(guest, args, 3) else {
        return int(args, 2);
    };
    let (negative, digits) = match value.split_first() {
        Some((b'-', rest)) => (true, rest),
        Some((b'+', rest)) => (false, rest),
        _ => (false, &value[..]),
    };
    let number = digits
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .fold(0u32, |number, byte| {
            number.wrapping_mul(10).wrapping_add(u32::from(byte - b'0'))
        });
    let number = if negative {
        number.wrapping_neg()
    } else {
        number
    };
    i64::from(number as i32)
}

/// `WritePrivateProfileStringA(section, key, value, file)`: returns 1 when the file was
/// written, 0 when it was not or lies outside the game directory.
fn write_profile_string(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let (Some(section), Some(key), Some(value), Some(file)) = (
        c_string(args, 0),
        c_string(args, 1),
        c_string(args, 2),
        c_string(args, 3),
    ) else {
        return 0;
    };
    let Some(path) = profile_path(guest, file) else {
        return 0;
    };
    let contents = std::fs::read(&path).unwrap_or_default();
    let written = std::fs::write(&path, profile_set(&contents, section, key, value)).is_ok();
    guest.files.refresh();
    i64::from(written)
}

/// `CopyFileA(source, destination, fail_if_exists)`: both names follow the rule of the
/// profile files. Returns 1 when the file was copied, 0 when it was not: a name outside the
/// game directory, a source that cannot be read, or a destination that exists while the third
/// argument is not 0.
fn copy_file(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let (Some(source), Some(destination)) = (c_string(args, 0), c_string(args, 1)) else {
        return 0;
    };
    let (Some(source), Some(destination)) = (
        profile_path(guest, source),
        profile_path(guest, destination),
    ) else {
        return 0;
    };
    if int(args, 2) != 0 && destination.exists() {
        return 0;
    }
    let Ok(contents) = std::fs::read(source) else {
        return 0;
    };
    let written = std::fs::write(destination, contents).is_ok();
    guest.files.refresh();
    i64::from(written)
}

// --- Text conversion --------------------------------------------------------------------------

/// The encoding of a configuration file with these contents, found as `vkxp-vfs` finds it:
/// UTF-8 if that is valid, otherwise the first legacy encoding that accepts every byte.
fn detect(contents: &[u8]) -> &'static Encoding {
    if std::str::from_utf8(contents).is_ok() {
        return encoding_rs::UTF_8;
    }
    vkxp_vfs::Encoding::LEGACY_ORDER
        .into_iter()
        .map(|legacy| match legacy {
            vkxp_vfs::Encoding::Utf8 => encoding_rs::UTF_8,
            vkxp_vfs::Encoding::Gbk => encoding_rs::GBK,
            vkxp_vfs::Encoding::ShiftJis => encoding_rs::SHIFT_JIS,
            vkxp_vfs::Encoding::Big5 => encoding_rs::BIG5,
        })
        .find(|encoding| {
            encoding
                .decode_without_bom_handling_and_without_replacement(contents)
                .is_some()
        })
        .unwrap_or(encoding_rs::UTF_8)
}

/// The encoding a code page number names. 0 and 1, the ANSI and OEM code pages of the
/// system, are the game's own: the encoding of its `Game.ini`.
fn code_page(guest: &Guest, number: i64) -> Option<&'static Encoding> {
    match number {
        65001 => Some(encoding_rs::UTF_8),
        936 => Some(encoding_rs::GBK),
        932 => Some(encoding_rs::SHIFT_JIS),
        950 => Some(encoding_rs::BIG5),
        0 | 1 => {
            let ini = guest.files.resolve(FileKind::Data, "Game.ini");
            let contents = ini.ok().and_then(|path| std::fs::read(path).ok());
            Some(detect(&contents.unwrap_or_default()))
        }
        _ => None,
    }
}

/// `text` in `encoding`, with `?` for what the encoding does not have.
fn encode(encoding: &'static Encoding, text: &str) -> Vec<u8> {
    let (bytes, _, unmappable) = encoding.encode(text);
    if !unmappable {
        return bytes.into_owned();
    }
    let mut out = Vec::new();
    let mut utf8 = [0; 4];
    for character in text.chars() {
        match encoding.encode(character.encode_utf8(&mut utf8)) {
            (_, _, true) => out.push(b'?'),
            (bytes, _, false) => out.extend_from_slice(&bytes),
        }
    }
    out
}

/// Stores the result of a conversion, `data` in units of `unit` bytes, in the buffer at
/// `index`, whose size in units is the argument after it. A size of 0 asks for the number
/// of units needed. Returns the number of units stored, or 0, as Windows does, when the
/// buffer is too small for all of them; what fits is stored all the same.
fn store(args: &mut [Arg], index: usize, data: &[u8], unit: usize) -> i64 {
    let size = usize::try_from(int(args, index + 1)).unwrap_or(0);
    if size == 0 {
        return (data.len() / unit) as i64;
    }
    let Some(buffer) = buffer_mut(args, index) else {
        return 0;
    };
    let room = size.min(buffer.len() / unit) * unit;
    let stored = data.len().min(room);
    buffer[..stored].copy_from_slice(&data[..stored]);
    if stored == data.len() {
        (stored / unit) as i64
    } else {
        0
    }
}

/// `MultiByteToWideChar(code_page, flags, source, length, buffer, size)`: UTF-16 in
/// little-endian byte order, the size counted in 16-bit units. A length of -1 takes the
/// source up to its zero byte and converts the terminator too.
fn multi_byte_to_wide_char(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(encoding) = code_page(guest, int(args, 0)) else {
        return 0;
    };
    let length = int(args, 3);
    let source = match usize::try_from(length) {
        Ok(length) => buffer(args, 2).map(|bytes| &bytes[..length.min(bytes.len())]),
        Err(_) => c_string(args, 2),
    };
    let Some(source) = source else {
        return 0;
    };
    let (text, _) = encoding.decode_without_bom_handling(source);
    let terminator = (length < 0).then_some(0);
    let wide: Vec<u8> = text
        .encode_utf16()
        .chain(terminator)
        .flat_map(u16::to_le_bytes)
        .collect();
    store(args, 4, &wide, 2)
}

/// `WideCharToMultiByte(code_page, flags, source, length, buffer, size, default, used)`: the
/// reverse, the length counted in 16-bit units and the size in bytes. The last two arguments
/// are not used.
fn wide_char_to_multi_byte(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(encoding) = code_page(guest, int(args, 0)) else {
        return 0;
    };
    let Some(source) = buffer(args, 2) else {
        return 0;
    };
    let units = source
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair));
    let length = int(args, 3);
    let units: Vec<u16> = match usize::try_from(length) {
        Ok(length) => units.take(length).collect(),
        Err(_) => units.take_while(|&unit| unit != 0).collect(),
    };
    let mut bytes = encode(encoding, &String::from_utf16_lossy(&units));
    if length < 0 {
        bytes.push(0);
    }
    store(args, 4, &bytes, 1)
}

// --- Time, screen and window ------------------------------------------------------------------

/// `GetTickCount()` and `timeGetTime()`: the milliseconds of the frames shown so far, in 32
/// bits.
fn tick_count(guest: &mut Guest, _args: &mut [Arg]) -> i64 {
    i64::from(guest.win32api.milliseconds as u32)
}

/// `GetSystemMetrics(index)`: the logical size for the width (0) and the height (1) of the
/// screen, 0 for every other index.
fn system_metrics(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let size = guest.scene.logical_size();
    match int(args, 0) {
        0 => i64::from(size.width),
        1 => i64::from(size.height),
        _ => 0,
    }
}

/// `FindWindowA(class, title)`, `GetActiveWindow()` and `GetForegroundWindow()`: the game
/// window, whatever is asked for.
fn window(_guest: &mut Guest, _args: &mut [Arg]) -> i64 {
    WINDOW
}

/// `GetWindowRect(window, buffer)` and `GetClientRect(window, buffer)`: the rectangle of the
/// logical size at the origin, as left, top, right and bottom in 32 bits each.
fn window_rect(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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

/// `ShowCursor(show)`: the new display count. The pointer is to be seen while the count is
/// 0 or more, and the host is told when a call changes that.
// PLACEHOLDER: the count starts at 0 and follows the documentation of Windows; it was not
// measured on the original.
fn show_cursor(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let before = guest.win32api.pointer_visible();
    guest.win32api.cursor += if int(args, 0) != 0 { 1 } else { -1 };
    let visible = guest.win32api.pointer_visible();
    if visible != before {
        guest.send(GuestToHost::PointerVisible { visible });
    }
    guest.win32api.cursor
}

// --- Keyboard and mouse -----------------------------------------------------------------------

/// The virtual-key code argument at `index`, when it is one of the 256.
fn virtual_key(args: &[Arg], index: usize) -> Option<u8> {
    u8::try_from(int(args, index)).ok()
}

/// `GetAsyncKeyState(key)`: bit 15 while the key is held in the snapshot `Input.update`
/// latched, and bit 0 too when it was not held in the one before. The value is a 16-bit
/// integer with its sign, as on Windows.
fn async_key_state(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
fn key_state(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(key) = virtual_key(args, 0) else {
        return 0;
    };
    let down = if reported_down(guest, key) { -0x80 } else { 0 };
    down | i64::from(guest.win32api.toggles.has(key))
}

/// `GetKeyboardState(buffer)`: in one byte per virtual-key code, 0x80 for a held key and the
/// toggle of the key in bit 0.
fn keyboard_state(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
fn keybd_event(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
fn mouse_event(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
fn set_keyboard_state(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
fn cursor_pos(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
fn screen_to_client(_guest: &mut Guest, _args: &mut [Arg]) -> i64 {
    1
}

/// `SetPriorityClass(process, priority)`: nothing is done, and that succeeds.
fn set_priority_class(_guest: &mut Guest, _args: &mut [Arg]) -> i64 {
    1
}

/// `GetProcessTimes(process, creation, exit, kernel, user)`: every time is 0, in 8 bytes
/// each.
fn process_times(_guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
fn move_memory(_guest: &mut Guest, args: &mut [Arg]) -> i64 {
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

// --- The call ---------------------------------------------------------------------------------

/// Runs `shim` on `args` and puts every buffer it changed back into the string at the same
/// index of `strings`, which must be live: in RGSS1 and RGSS2 by `String#replace`, which
/// labels the string ASCII-8BIT over Ruby 1.9; in RGSS3 with the encoding label the string
/// had, as the original runtime of RGSS3 leaves it (API report 3.3). The length stays.
fn run_shim(
    ruby: &Ruby,
    shim: Shim,
    strings: &[Option<VALUE>],
    mut args: Vec<Arg>,
    keep_label: bool,
) -> Result<i64, Raise> {
    let before = args.clone();
    let result = Guest::with(|guest| shim(guest, &mut args));
    for ((string, after), before) in strings.iter().zip(&args).zip(&before) {
        if let (Some(string), Arg::Buffer(bytes)) = (*string, after)
            && after != before
        {
            let contents = rb::bytes_new(ruby, bytes)?;
            if keep_label && rb::RUBY_19 {
                // SAFETY: `string` is live by the condition of this function, and every
                // value here was just returned and is in a local.
                let encoding = unsafe { ruby.funcall(string, c"encoding", &[]) }?;
                // SAFETY: as above.
                unsafe { ruby.funcall(contents, c"force_encoding", &[encoding]) }?;
            }
            // SAFETY: as above.
            unsafe { ruby.funcall(string, c"replace", &[contents]) }?;
        }
    }
    Ok(result)
}

/// `vkxp_win32api(library, function, *arguments)`: the value of the emulated function, or
/// `nil` when the function is not in the table.
fn call(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::check_count(ruby, args, 2, usize::MAX)?;
    let library = rb::text(ruby, args[0])?;
    let function = rb::text(ruby, args[1])?;
    let Some(shim) = find(&library, &function) else {
        return Ok(ffi::Qnil);
    };
    let values = &args[2..];
    let mut converted = Vec::with_capacity(values.len());
    let mut strings = Vec::with_capacity(values.len());
    for &value in values {
        if ffi::nil_p(value) {
            converted.push(Arg::Int(0));
            strings.push(None);
        } else if let Some(bytes) = rb::string_bytes(ruby, value) {
            converted.push(Arg::Buffer(bytes.into_boxed_slice()));
            strings.push(Some(value));
        } else {
            converted.push(Arg::Int(rb::long(ruby, value)?));
            strings.push(None);
        }
    }
    let result = run_shim(ruby, shim, &strings, converted, false)?;
    Ok(ffi::long2fix(result as c_long))
}

/// The names of the instance variables in which the `Win32API` of the original runtime of VX
/// keeps the library, the function and the argument types, in the order it interns them.
const NAMES: [&CStr; 6] = [
    c"__hdll__",
    c"__dll__",
    c"__dllname__",
    c"__proc__",
    c"__import__",
    c"__export__",
];

/// `vkxp_win32api_names`: called by `Win32API#initialize`. In a game of RGSS2 it interns the
/// six names the original interns at the first `Win32API.new` that loads its library, where
/// they take the next numbers of the symbol sequence. vkxp loads no library and keeps what
/// it needs in variables of its own, so it has no use for any of the six; they are interned
/// only so that the names a game makes afterwards have the numbers of the original.
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E version 2.0.2.1, `vx-v3-save-REPORT.md`
/// section 2.
fn names(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    if Guest::with(|guest| guest.runtime.run.runtime) == Runtime::Rgss2 {
        for name in NAMES {
            rb::intern(ruby, name)?;
        }
    }
    Ok(ffi::Qnil)
}

/// Defines `Win32API` for a game of `runtime`: in RGSS1 and RGSS2 the host functions the
/// class of the prelude builds on, in RGSS3 the class itself.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<(), Raised> {
    if runtime == Runtime::Rgss3 {
        return define_rgss3(ruby);
    }
    ruby.define_global_function_with_arity(c"vkxp_win32api_names", Arity::Fixed(0), names)?;
    ruby.define_global_function_with_arity(c"vkxp_win32api", Arity::Any, call)
}

// --- Win32API of RGSS3 ------------------------------------------------------------------------

/// The system libraries of Windows that `Win32API.new` finds in a game of RGSS3, in lower
/// case without `.dll`. vkxp loads none of them; a function of one is emulated or reported.
/// The list grows when a game shows a need.
const SYSTEM_LIBRARIES: &[&str] = &[
    "kernel32",
    "kernelbase",
    "user32",
    "gdi32",
    "advapi32",
    "shell32",
    "shlwapi",
    "ole32",
    "oleaut32",
    "comctl32",
    "comdlg32",
    "winmm",
    "wininet",
    "winhttp",
    "urlmon",
    "ws2_32",
    "wsock32",
    "msvcrt",
    "ntdll",
    "version",
    "imm32",
    "psapi",
    "dwmapi",
    "uxtheme",
    "msimg32",
    "gdiplus",
    "dsound",
    "ddraw",
    "dinput",
    "dinput8",
    "xinput1_3",
    "opengl32",
    "d3d9",
    "setupapi",
    "crypt32",
    "iphlpapi",
    "secur32",
    "netapi32",
    "mpr",
    "rpcrt4",
    "hid",
    "powrprof",
];

/// The instance variable, without `@` and so out of a script's sight, in which a `Win32API`
/// object of RGSS3 keeps the `Data` object that holds its [`Api`].
const STATE: &CStr = c"win32api";

/// What one character of an import declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Param {
    /// `l`, `i` or `n`: a 32-bit integer.
    Int,
    /// `p`: a pointer.
    Pointer,
}

/// What the first character of an export declares the value to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Export {
    /// `v`: the value is 0.
    Void,
    /// `p`: a String copied from the address.
    Pointer,
    /// Anything else: a signed 32-bit Integer.
    Int,
}

/// The state of a `Win32API` object of RGSS3.
#[derive(Debug, Clone)]
struct Api {
    /// The library as given.
    library: String,
    /// The function as given.
    function: String,
    params: Vec<Param>,
    export: Export,
}

/// The parameter one character of an import declares, in either case; `v` and every other
/// character declare none.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, API report 5.8.
fn param(character: u8) -> Option<Param> {
    match character.to_ascii_lowercase() {
        b'p' => Some(Param::Pointer),
        b'l' | b'i' | b'n' => Some(Param::Int),
        _ => None,
    }
}

/// What an export declares, by its first character.
fn export(declaration: &[u8]) -> Export {
    match declaration.first().map(u8::to_ascii_lowercase) {
        Some(b'v') => Export::Void,
        Some(b'p') => Export::Pointer,
        _ => Export::Int,
    }
}

/// Whether a game of RGSS3 finds the library `name`, with letter case ignored and with or
/// without `.dll`: a file of the game directory, the runtime library `Game.ini` names (by
/// its path, or by its file name alone), or one of [`SYSTEM_LIBRARIES`].
fn library_found(guest: &Guest, name: &str) -> bool {
    let normal = |name: &str| {
        let name = name.trim().replace('\\', "/").to_ascii_lowercase();
        name.strip_suffix(".dll").unwrap_or(&name).to_owned()
    };
    let wanted = normal(name);
    if SYSTEM_LIBRARIES.contains(&wanted.as_str()) {
        return true;
    }
    let runtime = normal(&guest.runtime.run.ini.library);
    let runtime_file = runtime.rsplit('/').next().unwrap_or_default();
    if !runtime.is_empty() && (wanted == runtime || wanted == runtime_file) {
        return true;
    }
    [name.to_owned(), format!("{name}.dll")].iter().any(|file| {
        guest
            .files
            .resolve(FileKind::Data, file)
            .is_ok_and(|path| path.is_file())
    })
}

/// The parameters an import declares: a String, each of its characters; an Array of
/// Strings, the first character of each; `nil`, none.
fn import(ruby: &Ruby, value: VALUE) -> Result<Vec<Param>, Raise> {
    if ffi::nil_p(value) {
        return Ok(Vec::new());
    }
    let Some(elements) = rb::array_elements(ruby, value) else {
        let declaration = rb::string_value(ruby, value)?;
        return Ok(rb::bytes(ruby, declaration)?
            .into_iter()
            .filter_map(param)
            .collect());
    };
    let mut params = Vec::with_capacity(elements.len());
    for element in elements {
        let declaration = rb::bytes(ruby, rb::string_value(ruby, element)?)?;
        params.extend(declaration.first().copied().and_then(param));
    }
    Ok(params)
}

unsafe extern "C" fn free_api(data: *mut c_void) {
    if !data.is_null() {
        // SAFETY: the collector passes the data pointer of an object `initialize` made with
        // this function, once; it is the box made there.
        drop(unsafe { Box::from_raw(data.cast::<Api>()) });
    }
}

/// `Win32API#initialize(library, function, import, export)`: the library and the function
/// are Strings, the import a String, an Array of Strings or `nil`, the export a String or
/// `nil`. A library the game does not find raises `RuntimeError` with `LoadLibrary: <name>`
/// and a line feed, as the original does for a missing DLL.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, API report 5.8.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let library = rb::text(ruby, rb::string_value(ruby, args[0])?)?;
    let function = rb::text(ruby, rb::string_value(ruby, args[1])?)?;
    let params = import(ruby, args[2])?;
    let export = if ffi::nil_p(args[3]) {
        Export::Int
    } else {
        export(&rb::bytes(ruby, rb::string_value(ruby, args[3])?)?)
    };
    if !Guest::with(|guest| library_found(guest, &library)) {
        // SAFETY: the class globals are set once the interpreter runs.
        let class = unsafe { ffi::rb_eRuntimeError };
        return Err(Raise::new(class, format!("LoadLibrary: {library}\n")));
    }
    let api = Box::into_raw(Box::new(Api {
        library,
        function,
        params,
        export,
    }));
    let data = rb::data_new(ruby, api.cast(), free_api)?;
    rb::ivar_set(ruby, recv, STATE, data)?;
    Ok(ffi::Qnil)
}

/// The state of the `Win32API` object `recv`.
fn api(ruby: &Ruby, recv: VALUE) -> Result<Api, Raise> {
    let state = rb::ivar_get(ruby, recv, STATE)?;
    let Some(data) = rb::data_of(state, free_api) else {
        // SAFETY: the class globals are set once the interpreter runs.
        let class = unsafe { ffi::rb_eRuntimeError };
        return Err(Raise::new(class, "uninitialized Win32API"));
    };
    // SAFETY: `data_of` found the box of an `Api` that `initialize` made, held by `state`,
    // which is in a local; it is copied before anything else runs.
    Ok(unsafe { &*data.cast::<Api>() }.clone())
}

/// An integer parameter: as Ruby converts to an unsigned long, then its low 32 bits, which
/// an entry reads with their sign. The RangeErrors so come at the 64-bit bounds of the
/// guest, not at 2**32 and -2**31: an object id, which a game passes to a library of its
/// own, is above 2**32 in the 64-bit guest.
fn int_param(ruby: &Ruby, value: VALUE) -> Result<i64, Raise> {
    Ok(i64::from(rb::ulong(ruby, value)? as u32 as i32))
}

/// A pointer parameter: `nil` is the null pointer, an Integer an address, a String a
/// buffer that must not be frozen; anything else is converted as Ruby converts to a String.
/// Returns the argument and the String to write a changed buffer back into, when that is the
/// argument itself; a String that `to_str` returned is read and not written back.
fn pointer_param(ruby: &Ruby, value: VALUE) -> Result<(Arg, Option<VALUE>), Raise> {
    if ffi::nil_p(value) {
        return Ok((Arg::Int(0), None));
    }
    // SAFETY: `value` is an argument of the call, and so live.
    let kind = unsafe { ffi::rb_type(value) };
    if kind == ffi::T_FIXNUM || kind == ffi::T_BIGNUM {
        return Ok((Arg::Int(int_param(ruby, value)?), None));
    }
    let string = rb::string_value(ruby, value)?;
    if rb::frozen(string) {
        // SAFETY: the class globals are set once the interpreter runs.
        let class = unsafe { ffi::rb_eRuntimeError };
        // The wording of the interpreter's check before it changes a string.
        return Err(Raise::new(class, "can't modify frozen string"));
    }
    let bytes = rb::bytes(ruby, string)?.into_boxed_slice();
    Ok((Arg::Buffer(bytes), (string == value).then_some(value)))
}

/// Writes the line that says a function is not emulated to `$stderr`, once per library and
/// function.
fn report(ruby: &Ruby, api: &Api) -> Result<(), Raise> {
    let name = format!("{} {}", api.library, api.function);
    if !Guest::with(|guest| guest.win32api.reported.insert(name.clone())) {
        return Ok(());
    }
    let line = format!("vkxp: Win32API {name} is not emulated and returns 0\n");
    let line = rb::text_new(ruby, line.as_bytes())?;
    let stderr = rb::global(ruby, c"$stderr")?;
    // SAFETY: both values were just returned and are in locals.
    unsafe { ruby.funcall(stderr, c"write", &[line]) }?;
    Ok(())
}

/// `Win32API#call(*arguments)` and `Call`: the count must be the declared one; the value
/// follows the export (decision 6 of the change that made this class).
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, API report 5.8.
fn call_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let api = api(ruby, recv)?;
    if args.len() != api.params.len() {
        // SAFETY: the class globals are set once the interpreter runs.
        let class = unsafe { ffi::rb_eRuntimeError };
        let message = format!(
            "wrong number of parameters: expected {}, got {}",
            api.params.len(),
            args.len()
        );
        return Err(Raise::new(class, message));
    }
    let mut converted = Vec::with_capacity(args.len());
    let mut strings = Vec::with_capacity(args.len());
    for (&value, &param) in args.iter().zip(&api.params) {
        let (arg, string) = match param {
            Param::Int => (Arg::Int(int_param(ruby, value)?), None),
            Param::Pointer => pointer_param(ruby, value)?,
        };
        converted.push(arg);
        strings.push(string);
    }
    let result = match find(&api.library, &api.function) {
        Some(shim) => Some(run_shim(ruby, shim, &strings, converted, true)?),
        None => {
            report(ruby, &api)?;
            None
        }
    };
    match (api.export, result) {
        (Export::Void, _) => Ok(rb::fix(0)),
        (Export::Pointer, Some(0)) => Err(Raise::new(rb::arg_error(ruby), "NULL pointer given")),
        // No value of an emulated function is an address the guest has.
        (Export::Pointer, _) => Ok(rb::bytes_new(ruby, b"")?),
        (Export::Int, result) => Ok(rb::fix(result.unwrap_or(0) as i32)),
    }
}

/// Defines `Win32API` of RGSS3 with the methods of the method list.
fn define_rgss3(ruby: &Ruby) -> Result<(), Raised> {
    // SAFETY: `Object` is a live class.
    let class = unsafe { ruby.define_class(c"Win32API", rb::object_class(ruby)) }?;
    // SAFETY: the class was just defined.
    unsafe {
        ruby.define_private_method_with_arity(class, c"initialize", Arity::Fixed(4), initialize)
    }?;
    rb::method(ruby, class, c"call", -1, call_rgss3)?;
    rb::method(ruby, class, c"Call", -1, call_rgss3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_is_read_as_the_configuration_file_is() {
        let file = b"\xEF\xBB\xBF; note\r\n[Other]\r\nTitle=no\r\n[ game ]\r\n title = A=B \r\n\
                     Title=later\r\nQuoted=\" x \"\r\nEmpty=\r\n";
        assert_eq!(profile_lookup(file, b"Game", b"TITLE"), Some(&b"A=B"[..]));
        assert_eq!(profile_lookup(file, b"Game", b"quoted"), Some(&b" x "[..]));
        assert_eq!(profile_lookup(file, b"Game", b"Empty"), Some(&b""[..]));
        assert_eq!(profile_lookup(file, b"Game", b"note"), None);
        assert_eq!(profile_lookup(file, b"Absent", b"Title"), None);
    }

    #[test]
    fn a_profile_is_changed_where_the_key_belongs() {
        let set = |contents: &str, key: &str| {
            String::from_utf8(profile_set(contents.as_bytes(), b"b", key.as_bytes(), b"9")).unwrap()
        };
        assert_eq!(set("", "k"), "[b]\r\nk=9\r\n");
        assert_eq!(set("[a]\nx=1", "k"), "[a]\nx=1\n[b]\nk=9\n");
        assert_eq!(
            set("[B]\r\nx=1\r\nK = 2\r\n[c]\r\nk=3\r\n", "k"),
            "[B]\r\nx=1\r\nK=9\r\n[c]\r\nk=3\r\n"
        );
        assert_eq!(
            set("[b]\nx=1\n\n[c]\ny=2\n", "k"),
            "[b]\nx=1\nk=9\n\n[c]\ny=2\n"
        );
        assert_eq!(set("[b]\nx=1", "k"), "[b]\nx=1\nk=9\n");
        assert_eq!(set("\u{feff}[b]\n", "k"), "\u{feff}[b]\nk=9\n");
    }

    #[test]
    fn the_code_page_of_a_game_is_the_encoding_of_its_configuration_file() {
        assert_eq!(detect(b""), encoding_rs::UTF_8);
        assert_eq!(detect("Title=测试".as_bytes()), encoding_rs::UTF_8);
        assert_eq!(detect(b"Title=\xB2\xE2\xCA\xD4"), encoding_rs::GBK);
        // Shift-JIS text that is not valid GBK: a lone half-width katakana.
        assert_eq!(detect(b"Title=\x83\x65\xB1"), encoding_rs::SHIFT_JIS);
    }

    #[test]
    fn what_an_encoding_does_not_have_becomes_a_question_mark() {
        assert_eq!(encode(encoding_rs::GBK, "a测b"), b"a\xB2\xE2b");
        assert_eq!(
            encode(encoding_rs::SHIFT_JIS, "a\u{1F600}測"),
            b"a?\x91\xAA"
        );
    }

    #[test]
    fn ticks_follow_the_frames_at_their_rate() {
        let mut state = State::default();
        for _ in 0..40 {
            state.frame_shown(40);
        }
        assert_eq!(state.milliseconds, 1000);
        for _ in 0..3 {
            state.frame_shown(60);
        }
        assert_eq!(state.milliseconds, 1050);
    }

    #[test]
    fn a_profile_name_stays_inside_the_game_directory() {
        let inside = |name: &str| inside_game(name.as_bytes()).map(|parts| parts.join("/"));
        assert_eq!(inside(".\\Game.ini").as_deref(), Some("Game.ini"));
        assert_eq!(inside("Data/../save.ini").as_deref(), Some("save.ini"));
        assert_eq!(
            inside("Data\\Config.ini").as_deref(),
            Some("Data/Config.ini")
        );
        for outside in [
            "C:\\Windows\\win.ini",
            "c:win.ini",
            "../other.ini",
            "/etc/x.ini",
            "\\\\host\\x.ini",
            "a/../../x.ini",
            ".",
            "",
        ] {
            assert_eq!(inside(outside), None, "{outside}");
        }
    }
}
