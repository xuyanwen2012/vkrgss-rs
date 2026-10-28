//! The private-profile (INI) functions and the copy of a file, inside the game directory.

use std::path::PathBuf;

use vkrgss_vfs::{FileKind, Runtime};

use crate::guest::Guest;

use super::table::{Arg, buffer_mut, c_string, int};

/// What a line of a profile file is, read as `vkrgss-vfs` reads `Game.ini`.
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
pub(super) fn profile_lookup<'a>(
    contents: &'a [u8],
    section: &[u8],
    key: &[u8],
) -> Option<&'a [u8]> {
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
pub(super) fn profile_set(contents: &[u8], section: &[u8], key: &[u8], value: &[u8]) -> Vec<u8> {
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
pub(super) fn inside_game(name: &[u8]) -> Option<Vec<String>> {
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

/// Where the profile functions find the file `name`, as [`profile_path`] finds it. In a game
/// of RGSS3 a name without `/` or `\` is a file of the Windows directory, which vkrgss does not
/// have, and is not found: `GetPrivateProfileStringA` with the bare name `"Game.ini"` gives
/// the default.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, API report 5.8. The write was
/// not measured; it follows the rule that files stay in the game directory.
fn profile_file(guest: &Guest, name: &[u8]) -> Option<PathBuf> {
    let bare = !name.iter().any(|&byte| byte == b'/' || byte == b'\\');
    if bare && guest.runtime.run.runtime == Runtime::Rgss3 {
        return None;
    }
    profile_path(guest, name)
}

/// The value the section, key and file name arguments name: the first two arguments and the
/// one at `file`. `None` when the file is missing or outside the game directory, or has no
/// such key.
fn profile_value(guest: &Guest, args: &[Arg], file: usize) -> Option<Vec<u8>> {
    let section = c_string(args, 0)?;
    let key = c_string(args, 1)?;
    let contents = std::fs::read(profile_file(guest, c_string(args, file)?)?).ok()?;
    profile_lookup(&contents, section, key).map(<[u8]>::to_vec)
}

/// `GetPrivateProfileStringA(section, key, default, buffer, size, file)`: the value, or the
/// default, goes into the buffer with a zero byte after it, cut to fit; returns its length.
pub(super) fn profile_string(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
pub(super) fn profile_int(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
pub(super) fn write_profile_string(guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let (Some(section), Some(key), Some(value), Some(file)) = (
        c_string(args, 0),
        c_string(args, 1),
        c_string(args, 2),
        c_string(args, 3),
    ) else {
        return 0;
    };
    let Some(path) = profile_file(guest, file) else {
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
pub(super) fn copy_file(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
