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
pub const PROTOCOL_VERSION: u16 = 8;

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
    use crate::{AudioChannel, AudioCommand, BitmapId, Button, FramePacket, FrameRate, Size};

    fn sample() -> Vec<GuestToHost> {
        vec![
            GuestToHost::SetTitle {
                title: "vkxp".to_owned(),
            },
            GuestToHost::Frame(FramePacket {
                frame_index: 1,
                logical_size: Size::new(640, 480),
                frame_rate: FrameRate::DEFAULT,
                tables: vec![],
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
    fn older_streams_are_rejected_naming_both_versions() {
        assert_eq!(PROTOCOL_VERSION, 8);
        for found in [1u16, 2, 3, 4, 5, 6, 7] {
            let mut bytes = encode_stream(&sample()).unwrap();
            bytes[4..6].copy_from_slice(&found.to_le_bytes());

            let error = decode_stream::<GuestToHost>(&bytes).unwrap_err();
            assert_eq!(
                error,
                DecodeError::UnsupportedVersion {
                    found,
                    supported: 8,
                }
            );
            assert_eq!(
                error.to_string(),
                format!("unsupported protocol version: found {found}, supported 8")
            );
        }
    }

    #[test]
    fn affine_and_stretch_quads_survive_a_round_trip() {
        use crate::{BlendMode, Color, Fixed, Op, Quad, Rect, Sampling, Tone, Transform};

        let blended = |sampling, blend| {
            Op::Quad(Quad {
                clip: Rect::new(0, 0, 640, 480),
                bitmap: BitmapId::new(7, 2),
                src: Rect::new(1, 2, 30, 40),
                sampling,
                tone: Tone::new(-10, 20, 30, 40),
                color: Color::new(1, 2, 3, 4),
                opacity: 200,
                blend,
            })
        };
        let quad = |sampling| blended(sampling, BlendMode::Add);
        let affine = Sampling::Affine(Transform {
            xx: Fixed::HALF,
            xy: Fixed::from_bits(-7),
            yx: Fixed::from_bits(9),
            yy: Fixed::ONE,
            tx: Fixed::from_bits(i32::MIN),
            ty: Fixed::from_bits(i32::MAX),
        });
        let stretch = Sampling::Stretch {
            dest: Rect::new(-100, i32::MIN, 75, u32::MAX),
            mirror: true,
        };
        let messages = vec![GuestToHost::Frame(FramePacket {
            frame_index: 3,
            logical_size: Size::new(640, 480),
            frame_rate: FrameRate::DEFAULT,
            tables: vec![],
            bitmaps: vec![],
            ops: vec![
                quad(affine),
                quad(stretch),
                blended(Sampling::place_at(3, -4), BlendMode::Window),
            ],
        })];
        let bytes = encode_stream(&messages).unwrap();
        assert_eq!(decode_stream::<GuestToHost>(&bytes).unwrap(), messages);
    }

    #[test]
    fn tables_and_tilemap_layers_survive_a_round_trip() {
        use crate::{Op, Rect, TableDelta, TableId, TilemapLayer};

        let (map, priorities) = (TableId::new(0), TableId::new(u32::MAX));
        let cells: Vec<i16> = (0..20 * 15 * 3)
            .map(|i| (i * 7 % 2000 - 5) as i16)
            .collect();
        let messages = vec![GuestToHost::Frame(FramePacket {
            frame_index: 9,
            logical_size: Size::new(640, 480),
            frame_rate: FrameRate::DEFAULT,
            tables: vec![
                TableDelta::Set {
                    id: map,
                    xsize: 20,
                    ysize: 15,
                    zsize: 3,
                    data: cells,
                },
                TableDelta::Set {
                    id: priorities,
                    xsize: 400,
                    ysize: 1,
                    zsize: 1,
                    data: (0..400).map(|i| i % 6).collect(),
                },
                TableDelta::Destroy {
                    id: TableId::new(5),
                },
            ],
            bitmaps: vec![],
            ops: vec![Op::TilemapLayer(TilemapLayer {
                clip: Rect::new(0, 64, 640, 32),
                tileset: Some(BitmapId::new(4, 1)),
                autotiles: [
                    Some(BitmapId::new(5, 0)),
                    None,
                    None,
                    Some(BitmapId::new(u32::MAX, u32::MAX)),
                    None,
                    None,
                    Some(BitmapId::new(6, 2)),
                ],
                map,
                priorities,
                origin: (i32::MIN, i32::MAX),
                layer: 2,
                priority: 5,
                animation_step: u64::MAX,
            })],
        })];
        let bytes = encode_stream(&messages).unwrap();
        assert_eq!(decode_stream::<GuestToHost>(&bytes).unwrap(), messages);
    }

    fn packet_at(frame_index: u64, frame_rate: FrameRate) -> GuestToHost {
        GuestToHost::Frame(FramePacket {
            frame_index,
            logical_size: Size::new(640, 480),
            frame_rate,
            tables: vec![],
            bitmaps: vec![],
            ops: vec![],
        })
    }

    #[test]
    fn frame_rate_travels_with_every_packet() {
        let messages = [
            packet_at(0, FrameRate::DEFAULT),
            packet_at(1, FrameRate::MAX),
            packet_at(2, FrameRate::MIN),
        ];
        let bytes = encode_stream(&messages).unwrap();
        let decoded = decode_stream::<GuestToHost>(&bytes).unwrap();
        let rates: Vec<u8> = decoded
            .iter()
            .map(|message| match message {
                GuestToHost::Frame(packet) => packet.frame_rate.get(),
                other => panic!("not a frame: {other:?}"),
            })
            .collect();
        assert_eq!(rates, [40, 120, 10]);
    }

    /// The stream of one empty packet whose frame-rate byte is replaced by `fps`.
    fn stream_with_frame_rate(fps: u8) -> Vec<u8> {
        let marker = FrameRate::new(77).unwrap();
        let mut bytes = encode_stream(&[packet_at(0, marker)]).unwrap();
        let positions: Vec<usize> = (HEADER_LEN..bytes.len())
            .filter(|&index| bytes[index] == 77)
            .collect();
        assert_eq!(positions.len(), 1, "the frame rate is one distinct byte");
        bytes[positions[0]] = fps;
        bytes
    }

    #[test]
    fn frame_rate_out_of_range_does_not_decode() {
        for fps in [0, 9, 121, 127] {
            assert!(
                matches!(
                    decode_stream::<GuestToHost>(&stream_with_frame_rate(fps)),
                    Err(DecodeError::Malformed { index: 0, .. })
                ),
                "frame rate {fps}"
            );
        }
        for fps in [10, 40, 120] {
            assert_eq!(
                decode_stream::<GuestToHost>(&stream_with_frame_rate(fps)),
                Ok(vec![packet_at(0, FrameRate::new(fps).unwrap())]),
                "frame rate {fps}"
            );
        }
    }

    #[test]
    fn every_kind_of_message_round_trips() {
        let guest = vec![
            packet_at(3, FrameRate::DEFAULT),
            GuestToHost::Freeze,
            GuestToHost::Transition {
                duration: 20,
                mask: Some(BitmapId::new(4, 1)),
                vague: 40,
            },
            GuestToHost::Audio(AudioCommand::Stop {
                channel: AudioChannel::Bgm,
            }),
            GuestToHost::Modal {
                text: "message".to_owned(),
            },
            GuestToHost::SetTitle {
                title: "title".to_owned(),
            },
            GuestToHost::Shutdown,
            GuestToHost::FrameReset,
            GuestToHost::Attach {
                pid: 4711,
                parked: Some(9),
            },
            GuestToHost::Outcome { ok: true },
            GuestToHost::StateSaved { slot: 0 },
            GuestToHost::StateLoaded { slot: 3 },
            GuestToHost::StateFailed { slot: 9 },
        ];
        // Fails to compile when a kind is added without extending the list above.
        for message in &guest {
            match message {
                GuestToHost::Frame(_)
                | GuestToHost::Freeze
                | GuestToHost::Transition { .. }
                | GuestToHost::Audio(_)
                | GuestToHost::Modal { .. }
                | GuestToHost::SetTitle { .. }
                | GuestToHost::Shutdown
                | GuestToHost::FrameReset
                | GuestToHost::Attach { .. }
                | GuestToHost::Outcome { .. }
                | GuestToHost::StateSaved { .. }
                | GuestToHost::StateLoaded { .. }
                | GuestToHost::StateFailed { .. } => {}
            }
        }
        let kinds: std::collections::HashSet<_> =
            guest.iter().map(std::mem::discriminant).collect();
        assert_eq!(kinds.len(), 13);
        let bytes = encode_stream(&guest).unwrap();
        assert_eq!(decode_stream::<GuestToHost>(&bytes), Ok(guest));

        let host = vec![
            HostToGuest::Input(InputSnapshot::EMPTY.with(Button::F9)),
            HostToGuest::ModalDismissed,
            HostToGuest::Tick,
            HostToGuest::Quit,
            HostToGuest::Reset,
            HostToGuest::SaveState { slot: 2 },
            HostToGuest::LoadState { slot: 2 },
            HostToGuest::Resume,
        ];
        for message in &host {
            match message {
                HostToGuest::Input(_)
                | HostToGuest::ModalDismissed
                | HostToGuest::Tick
                | HostToGuest::Quit
                | HostToGuest::Reset
                | HostToGuest::SaveState { .. }
                | HostToGuest::LoadState { .. }
                | HostToGuest::Resume => {}
            }
        }
        let kinds: std::collections::HashSet<_> = host.iter().map(std::mem::discriminant).collect();
        assert_eq!(kinds.len(), 8);
        let bytes = encode_stream(&host).unwrap();
        assert_eq!(decode_stream::<HostToGuest>(&bytes), Ok(host));
    }

    #[test]
    fn reset_is_not_quit() {
        let bytes = encode_stream(&[HostToGuest::Reset, HostToGuest::Quit]).unwrap();
        let decoded = decode_stream::<HostToGuest>(&bytes).unwrap();
        assert_eq!(decoded, [HostToGuest::Reset, HostToGuest::Quit]);
        assert_ne!(decoded[0], decoded[1]);
        assert_ne!(
            encode_stream(&[HostToGuest::Reset]).unwrap(),
            encode_stream(&[HostToGuest::Quit]).unwrap()
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

    /// Scenario "Round trip" of the raw key and pointer state.
    #[test]
    fn a_snapshot_with_raw_keys_and_a_pointer_round_trips() {
        let snapshot = InputSnapshot::EMPTY
            .with_key(0x01)
            .with_key(0x41)
            .with_pointer(Some((100, 50)));
        let bytes = encode_stream(&[snapshot]).unwrap();
        assert_eq!(decode_stream::<InputSnapshot>(&bytes), Ok(vec![snapshot]));
        let messages = vec![HostToGuest::Input(snapshot)];
        let bytes = encode_stream(&messages).unwrap();
        assert_eq!(decode_stream::<HostToGuest>(&bytes), Ok(messages));
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
