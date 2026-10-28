//! The code pages and the conversions `MultiByteToWideChar` and `WideCharToMultiByte`.

use encoding_rs::Encoding;
use vkrgss_vfs::FileKind;

use crate::guest::Guest;

use super::table::{Arg, buffer, buffer_mut, c_string, int};

/// The encoding of a configuration file with these contents, found as `vkrgss-vfs` finds it:
/// UTF-8 if that is valid, otherwise the first legacy encoding that accepts every byte.
pub(super) fn detect(contents: &[u8]) -> &'static Encoding {
    if std::str::from_utf8(contents).is_ok() {
        return encoding_rs::UTF_8;
    }
    vkrgss_vfs::Encoding::LEGACY_ORDER
        .into_iter()
        .map(|legacy| match legacy {
            vkrgss_vfs::Encoding::Utf8 => encoding_rs::UTF_8,
            vkrgss_vfs::Encoding::Gbk => encoding_rs::GBK,
            vkrgss_vfs::Encoding::ShiftJis => encoding_rs::SHIFT_JIS,
            vkrgss_vfs::Encoding::Big5 => encoding_rs::BIG5,
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
pub(super) fn encode(encoding: &'static Encoding, text: &str) -> Vec<u8> {
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
pub(super) fn multi_byte_to_wide_char(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
pub(super) fn wide_char_to_multi_byte(guest: &mut Guest, args: &mut [Arg]) -> i64 {
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
