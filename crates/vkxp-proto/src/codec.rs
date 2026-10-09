//! Versioned binary encoding of message streams.
//!
//! A stream is a header followed by any number of messages of one type:
//!
//! ```text
//! magic    4 bytes  "VKXP"
//! version  2 bytes  protocol version, little endian
//! kind     1 byte   which message type follows, see `StreamKind`
//! messages          each in postcard encoding, back to back
//! ```
//!
//! Encoding is deterministic: equal messages always give equal bytes. Decoding is all or
//! nothing: a stream with a foreign header or a malformed message yields an error and no
//! messages.

use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::{GuestToHost, HostToGuest, InputSnapshot};

/// First bytes of every encoded stream.
pub const MAGIC: [u8; 4] = *b"VKXP";

/// Version of the message types and their encoding. Any change to either increments it.
pub const PROTOCOL_VERSION: u16 = 1;

/// Length of the stream header in bytes.
pub const HEADER_LEN: usize = 7;

/// The message type a stream carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StreamKind {
    /// [`GuestToHost`] messages.
    GuestToHost = 1,
    /// [`HostToGuest`] messages.
    HostToGuest = 2,
    /// [`InputSnapshot`] values: an input recording.
    InputRecording = 3,
}

impl std::fmt::Display for StreamKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            StreamKind::GuestToHost => "guest-to-host",
            StreamKind::HostToGuest => "host-to-guest",
            StreamKind::InputRecording => "input recording",
        })
    }
}

/// A type that can be the element of an encoded stream.
pub trait Message: Serialize + DeserializeOwned {
    /// The stream kind written to and expected in the header.
    const KIND: StreamKind;
}

impl Message for GuestToHost {
    const KIND: StreamKind = StreamKind::GuestToHost;
}

impl Message for HostToGuest {
    const KIND: StreamKind = StreamKind::HostToGuest;
}

impl Message for InputSnapshot {
    const KIND: StreamKind = StreamKind::InputRecording;
}

/// Why a stream could not be encoded.
#[derive(Debug, Error)]
#[error("message {index} could not be encoded: {reason}")]
pub struct EncodeError {
    /// Position of the message in the stream.
    pub index: usize,
    pub reason: postcard::Error,
}

/// Why a stream could not be decoded.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("stream is {len} bytes long, shorter than the {HEADER_LEN}-byte header")]
    TruncatedHeader { len: usize },
    #[error("stream does not start with the vkxp magic: found {found:02x?}, expected {MAGIC:02x?}")]
    BadMagic { found: [u8; 4] },
    #[error("unsupported protocol version: found {found}, supported {supported}")]
    UnsupportedVersion { found: u16, supported: u16 },
    #[error("wrong stream kind: found {found}, expected {expected} ({expected_kind})", expected = *.expected as u8, expected_kind = .expected)]
    WrongStreamKind { found: u8, expected: StreamKind },
    #[error("message {index} is malformed: {reason}")]
    Malformed {
        /// Position of the message in the stream.
        index: usize,
        reason: postcard::Error,
    },
}

/// Encodes messages as one stream, header included.
pub fn encode_stream<'a, T, I>(messages: I) -> Result<Vec<u8>, EncodeError>
where
    T: Message + 'a,
    I: IntoIterator<Item = &'a T>,
{
    let mut bytes = Vec::with_capacity(HEADER_LEN);
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
    bytes.push(T::KIND as u8);
    for (index, message) in messages.into_iter().enumerate() {
        bytes =
            postcard::to_extend(message, bytes).map_err(|reason| EncodeError { index, reason })?;
    }
    Ok(bytes)
}

/// Decodes a whole stream. Returns every message or, on any error, none.
pub fn decode_stream<T: Message>(bytes: &[u8]) -> Result<Vec<T>, DecodeError> {
    let Some((header, mut rest)) = bytes.split_first_chunk::<HEADER_LEN>() else {
        return Err(DecodeError::TruncatedHeader { len: bytes.len() });
    };
    let [m0, m1, m2, m3, v0, v1, kind] = *header;
    if [m0, m1, m2, m3] != MAGIC {
        return Err(DecodeError::BadMagic {
            found: [m0, m1, m2, m3],
        });
    }
    let version = u16::from_le_bytes([v0, v1]);
    if version != PROTOCOL_VERSION {
        return Err(DecodeError::UnsupportedVersion {
            found: version,
            supported: PROTOCOL_VERSION,
        });
    }
    if kind != T::KIND as u8 {
        return Err(DecodeError::WrongStreamKind {
            found: kind,
            expected: T::KIND,
        });
    }

    let mut messages = Vec::new();
    while !rest.is_empty() {
        let (message, remaining) =
            postcard::take_from_bytes(rest).map_err(|reason| DecodeError::Malformed {
                index: messages.len(),
                reason,
            })?;
        messages.push(message);
        rest = remaining;
    }
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Button, FramePacket, Size};

    fn sample() -> Vec<GuestToHost> {
        vec![
            GuestToHost::SetTitle {
                title: "vkxp".to_owned(),
            },
            GuestToHost::Frame(FramePacket {
                frame_index: 1,
                logical_size: Size::new(640, 480),
                bitmaps: vec![],
                ops: vec![],
            }),
            GuestToHost::Shutdown,
        ]
    }

    #[test]
    fn stream_starts_with_magic_version_and_kind() {
        let bytes = encode_stream(&sample()).unwrap();
        assert_eq!(&bytes[..4], b"VKXP");
        assert_eq!(&bytes[4..6], &PROTOCOL_VERSION.to_le_bytes());
        assert_eq!(bytes[6], StreamKind::GuestToHost as u8);
    }

    #[test]
    fn empty_stream_is_only_a_header() {
        let bytes = encode_stream::<HostToGuest, _>([]).unwrap();
        assert_eq!(bytes.len(), HEADER_LEN);
        assert_eq!(decode_stream::<HostToGuest>(&bytes), Ok(vec![]));
    }

    #[test]
    fn sample_round_trips() {
        let messages = sample();
        let bytes = encode_stream(&messages).unwrap();
        assert_eq!(decode_stream::<GuestToHost>(&bytes), Ok(messages));
    }

    #[test]
    fn unsupported_version_is_rejected_with_both_versions() {
        let mut bytes = encode_stream(&sample()).unwrap();
        let foreign = PROTOCOL_VERSION + 41;
        bytes[4..6].copy_from_slice(&foreign.to_le_bytes());

        let error = decode_stream::<GuestToHost>(&bytes).unwrap_err();
        assert_eq!(
            error,
            DecodeError::UnsupportedVersion {
                found: foreign,
                supported: PROTOCOL_VERSION,
            }
        );
        let text = error.to_string();
        assert!(text.contains(&format!("found {foreign}")), "{text}");
        assert!(
            text.contains(&format!("supported {PROTOCOL_VERSION}")),
            "{text}"
        );
    }

    #[test]
    fn older_version_is_rejected_too() {
        let mut bytes = encode_stream(&sample()).unwrap();
        bytes[4..6].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            decode_stream::<GuestToHost>(&bytes),
            Err(DecodeError::UnsupportedVersion {
                found: 0,
                supported: PROTOCOL_VERSION,
            })
        );
    }

    #[test]
    fn bad_magic_is_rejected() {
        let mut bytes = encode_stream(&sample()).unwrap();
        bytes[0] = b'X';
        assert_eq!(
            decode_stream::<GuestToHost>(&bytes),
            Err(DecodeError::BadMagic { found: *b"XKXP" })
        );
    }

    #[test]
    fn truncated_header_is_rejected() {
        assert_eq!(
            decode_stream::<GuestToHost>(b"VKXP"),
            Err(DecodeError::TruncatedHeader { len: 4 })
        );
        assert_eq!(
            decode_stream::<GuestToHost>(&[]),
            Err(DecodeError::TruncatedHeader { len: 0 })
        );
    }

    #[test]
    fn stream_of_another_kind_is_rejected() {
        let recording = [InputSnapshot::EMPTY.with(Button::C)];
        let bytes = encode_stream(&recording).unwrap();
        let error = decode_stream::<GuestToHost>(&bytes).unwrap_err();
        assert_eq!(
            error,
            DecodeError::WrongStreamKind {
                found: StreamKind::InputRecording as u8,
                expected: StreamKind::GuestToHost,
            }
        );
        assert_eq!(
            error.to_string(),
            "wrong stream kind: found 3, expected 1 (guest-to-host)"
        );
    }

    #[test]
    fn truncated_message_yields_no_messages() {
        let bytes = encode_stream(&sample()).unwrap();
        let cut = &bytes[..bytes.len() - 2];
        assert!(matches!(
            decode_stream::<GuestToHost>(cut),
            Err(DecodeError::Malformed { index: 1, .. })
        ));
    }

    #[test]
    fn unknown_variant_is_malformed() {
        let mut bytes = encode_stream::<HostToGuest, _>([]).unwrap();
        bytes.push(200);
        assert!(matches!(
            decode_stream::<HostToGuest>(&bytes),
            Err(DecodeError::Malformed { index: 0, .. })
        ));
    }
}
