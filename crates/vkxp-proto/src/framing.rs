//! Framing of single messages for a byte stream.
//!
//! A message on the stream is:
//!
//! ```text
//! length   4 bytes  number of bytes that follow, little endian
//! message           one message as `codec` encodes a stream of it: header, then the message
//! ```
//!
//! The functions take [`Read`] and [`Write`] and know nothing of sockets. A length above
//! [`MAX_MESSAGE_LEN`], bytes that do not decode and a stream that ends inside a message are
//! errors, after which the stream is of no further use; a stream that ends between two
//! messages is its end.

use std::io::{self, ErrorKind, Read, Write};

use thiserror::Error;

use crate::codec::{self, DecodeError, EncodeError, Message};

/// The largest length a message may state. A frame packet that carries every pixel of a
/// screen-sized bitmap is 1.2 MiB; a game that loads its tilesets and pictures in one frame
/// sends a few dozen times that.
pub const MAX_MESSAGE_LEN: u32 = 1 << 30;

/// Number of bytes of the length in front of a message.
pub const LENGTH_LEN: usize = 4;

/// Why a message could not be written to or read from a stream.
#[derive(Debug, Error)]
pub enum FramingError {
    #[error("the stream failed: {0}")]
    Io(#[from] io::Error),
    #[error("the stream ended inside a message")]
    Truncated,
    #[error("a message of {len} bytes is above the bound of {MAX_MESSAGE_LEN} bytes")]
    TooLong { len: u64 },
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error("one length is followed by {count} messages instead of one")]
    NotOneMessage { count: usize },
}

/// The length a message states, if it is within the bound.
pub(crate) fn message_len(length: [u8; LENGTH_LEN]) -> Result<usize, FramingError> {
    let len = u32::from_le_bytes(length);
    if len > MAX_MESSAGE_LEN {
        return Err(FramingError::TooLong { len: len.into() });
    }
    Ok(len as usize)
}

/// The message that the bytes after a length hold.
pub(crate) fn decode_message<T: Message>(bytes: &[u8]) -> Result<T, FramingError> {
    let mut messages = codec::decode_stream::<T>(bytes)?;
    match messages.pop() {
        Some(message) if messages.is_empty() => Ok(message),
        last => Err(FramingError::NotOneMessage {
            count: messages.len() + usize::from(last.is_some()),
        }),
    }
}

/// Writes one message. The writer is not flushed.
pub fn write_message<T: Message>(writer: &mut impl Write, message: &T) -> Result<(), FramingError> {
    let bytes = codec::encode_stream([message])?;
    let len = u32::try_from(bytes.len())
        .ok()
        .filter(|len| *len <= MAX_MESSAGE_LEN)
        .ok_or(FramingError::TooLong {
            len: bytes.len() as u64,
        })?;
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(&bytes)?;
    Ok(())
}

/// Reads one message. `Ok(None)` is a stream that ended before the first byte of a length.
pub fn read_message<T: Message>(reader: &mut impl Read) -> Result<Option<T>, FramingError> {
    let mut length = [0; LENGTH_LEN];
    let mut filled = 0;
    while filled < LENGTH_LEN {
        match reader.read(&mut length[filled..]) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(FramingError::Truncated),
            Ok(count) => filled += count,
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    let len = message_len(length)?;
    // The buffer grows with the bytes that arrive, not with what the length states.
    let mut bytes = Vec::new();
    reader.take(len as u64).read_to_end(&mut bytes)?;
    if bytes.len() < len {
        return Err(FramingError::Truncated);
    }
    decode_message(&bytes).map(Some)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::codec::{HEADER_LEN, MAGIC, PROTOCOL_VERSION};
    use crate::{
        BitmapDelta, BitmapId, FramePacket, FrameRate, GuestToHost, HostToGuest, InputSnapshot,
        Rect, Size,
    };

    /// A frame packet that carries every pixel of a bitmap of 640 by 480.
    fn frame() -> GuestToHost {
        let rgba: Vec<u8> = (0..640 * 480 * 4)
            .map(|index| (index % 251) as u8)
            .collect();
        let id = BitmapId::new(1, 0);
        GuestToHost::Frame(FramePacket {
            frame_index: 3,
            logical_size: Size::new(640, 480),
            frame_rate: FrameRate::DEFAULT,
            bitmaps: vec![
                BitmapDelta::Create {
                    id,
                    width: 640,
                    height: 480,
                },
                BitmapDelta::Update {
                    id,
                    rect: Rect::new(0, 0, 640, 480),
                    rgba,
                },
            ],
            tables: Vec::new(),
            ops: Vec::new(),
        })
    }

    /// Scenario "Round trip".
    #[test]
    fn three_messages_are_read_back_as_written() {
        let packet = frame();
        let mut to_host = Vec::new();
        write_message(&mut to_host, &packet).unwrap();
        let mut to_guest = Vec::new();
        write_message(&mut to_guest, &HostToGuest::Input(InputSnapshot::EMPTY)).unwrap();
        write_message(&mut to_guest, &HostToGuest::Tick).unwrap();

        let mut reader = Cursor::new(to_host);
        assert_eq!(read_message(&mut reader).unwrap(), Some(packet));
        assert_eq!(read_message::<GuestToHost>(&mut reader).unwrap(), None);
        let mut reader = Cursor::new(to_guest);
        assert_eq!(
            read_message(&mut reader).unwrap(),
            Some(HostToGuest::Input(InputSnapshot::EMPTY))
        );
        assert_eq!(read_message(&mut reader).unwrap(), Some(HostToGuest::Tick));
        assert_eq!(read_message::<HostToGuest>(&mut reader).unwrap(), None);
    }

    /// Scenario "Truncated message".
    #[test]
    fn a_message_cut_short_is_an_error() {
        let mut bytes = Vec::new();
        write_message(&mut bytes, &HostToGuest::Input(InputSnapshot::EMPTY)).unwrap();
        bytes.pop();
        let error = read_message::<HostToGuest>(&mut bytes.as_slice()).unwrap_err();
        assert!(matches!(error, FramingError::Truncated), "{error}");

        // Cut inside the length.
        let error = read_message::<HostToGuest>(&mut &bytes[..2]).unwrap_err();
        assert!(matches!(error, FramingError::Truncated), "{error}");
    }

    /// Scenario "Oversized length".
    #[test]
    fn a_length_above_the_bound_is_an_error_and_nothing_after_it_is_read() {
        let mut bytes = (MAX_MESSAGE_LEN + 1).to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0; 16]);
        let mut reader = Cursor::new(bytes);
        let error = read_message::<HostToGuest>(&mut reader).unwrap_err();
        assert!(
            matches!(error, FramingError::TooLong { len } if len == u64::from(MAX_MESSAGE_LEN) + 1),
            "{error}"
        );
        assert_eq!(reader.position(), LENGTH_LEN as u64);
    }

    /// A length within the bound that the stream does not keep allocates only what arrives.
    #[test]
    fn a_length_the_stream_does_not_keep_is_a_truncated_message() {
        let mut bytes = MAX_MESSAGE_LEN.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0; 16]);
        let error = read_message::<HostToGuest>(&mut bytes.as_slice()).unwrap_err();
        assert!(matches!(error, FramingError::Truncated), "{error}");
    }

    /// Scenario "Other version".
    #[test]
    fn a_message_of_another_version_is_the_version_error_of_the_encoding() {
        let mut bytes = Vec::new();
        write_message(&mut bytes, &HostToGuest::Tick).unwrap();
        let version = LENGTH_LEN + MAGIC.len();
        bytes[version..version + 2].copy_from_slice(&(PROTOCOL_VERSION + 1).to_le_bytes());
        let error = read_message::<HostToGuest>(&mut bytes.as_slice()).unwrap_err();
        assert!(
            matches!(
                error,
                FramingError::Decode(DecodeError::UnsupportedVersion { found, supported })
                    if found == PROTOCOL_VERSION + 1 && supported == PROTOCOL_VERSION
            ),
            "{error}"
        );
    }

    #[test]
    fn bytes_that_do_not_decode_are_an_error() {
        // A message of the other direction.
        let mut bytes = Vec::new();
        write_message(&mut bytes, &HostToGuest::Tick).unwrap();
        let error = read_message::<GuestToHost>(&mut bytes.as_slice()).unwrap_err();
        assert!(
            matches!(
                error,
                FramingError::Decode(DecodeError::WrongStreamKind { .. })
            ),
            "{error}"
        );

        // A header with no message, and one with two.
        let mut empty = (HEADER_LEN as u32).to_le_bytes().to_vec();
        empty.extend_from_slice(&codec::encode_stream::<HostToGuest, _>([]).unwrap());
        let error = read_message::<HostToGuest>(&mut empty.as_slice()).unwrap_err();
        assert!(
            matches!(error, FramingError::NotOneMessage { count: 0 }),
            "{error}"
        );
        let two = codec::encode_stream([&HostToGuest::Tick, &HostToGuest::Quit]).unwrap();
        let mut bytes = (two.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(&two);
        let error = read_message::<HostToGuest>(&mut bytes.as_slice()).unwrap_err();
        assert!(
            matches!(error, FramingError::NotOneMessage { count: 2 }),
            "{error}"
        );

        // A variant that does not exist.
        let mut bytes = Vec::new();
        write_message(&mut bytes, &HostToGuest::Tick).unwrap();
        *bytes.last_mut().unwrap() = 0x7f;
        let error = read_message::<HostToGuest>(&mut bytes.as_slice()).unwrap_err();
        assert!(
            matches!(error, FramingError::Decode(DecodeError::Malformed { .. })),
            "{error}"
        );
    }
}
