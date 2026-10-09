//! Property tests for the stream encoding: every message survives a round trip, and equal
//! messages always encode to equal bytes.

use proptest::collection::vec;
use proptest::option;
use proptest::prelude::*;
use vkxp_proto::codec::{Message, decode_stream, encode_stream};
use vkxp_proto::{
    AudioChannel, AudioCommand, BitmapDelta, BitmapId, BlendMode, Color, Fixed, FramePacket,
    GuestToHost, HostToGuest, InputSnapshot, Op, Quad, Rect, RectEffect, Size, Tone, Transform,
};

fn fixed() -> impl Strategy<Value = Fixed> {
    any::<i32>().prop_map(Fixed::from_bits)
}

fn rect() -> impl Strategy<Value = Rect> {
    any::<(i32, i32, u32, u32)>().prop_map(|(x, y, width, height)| Rect::new(x, y, width, height))
}

fn color() -> impl Strategy<Value = Color> {
    any::<[u8; 4]>().prop_map(Color::from_rgba)
}

fn tone() -> impl Strategy<Value = Tone> {
    any::<(i16, i16, i16, u8)>()
        .prop_map(|(red, green, blue, gray)| Tone::new(red, green, blue, gray))
}

fn bitmap_id() -> impl Strategy<Value = BitmapId> {
    any::<(u32, u32)>().prop_map(|(index, generation)| BitmapId::new(index, generation))
}

fn transform() -> impl Strategy<Value = Transform> {
    (fixed(), fixed(), fixed(), fixed(), fixed(), fixed()).prop_map(|(xx, xy, yx, yy, tx, ty)| {
        Transform {
            xx,
            xy,
            yx,
            yy,
            tx,
            ty,
        }
    })
}

fn blend_mode() -> impl Strategy<Value = BlendMode> {
    prop_oneof![
        Just(BlendMode::Normal),
        Just(BlendMode::Add),
        Just(BlendMode::Subtract),
    ]
}

fn quad() -> impl Strategy<Value = Quad> {
    (
        rect(),
        bitmap_id(),
        rect(),
        transform(),
        any::<u8>(),
        blend_mode(),
    )
        .prop_map(|(clip, bitmap, src, transform, opacity, blend)| Quad {
            clip,
            bitmap,
            src,
            transform,
            opacity,
            blend,
        })
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (rect(), color()).prop_map(|(clip, color)| Op::Clear { clip, color }),
        quad().prop_map(Op::Quad),
        (rect(), tone(), color()).prop_map(|(clip, tone, color)| Op::RectEffect(RectEffect {
            clip,
            tone,
            color
        })),
        rect().prop_map(|clip| Op::Plane { clip }),
        rect().prop_map(|clip| Op::TilemapLayer { clip }),
    ]
}

fn bitmap_delta() -> impl Strategy<Value = BitmapDelta> {
    prop_oneof![
        (bitmap_id(), any::<u32>(), any::<u32>())
            .prop_map(|(id, width, height)| BitmapDelta::Create { id, width, height }),
        (bitmap_id(), rect(), vec(any::<u8>(), 0..64))
            .prop_map(|(id, rect, rgba)| BitmapDelta::Update { id, rect, rgba }),
        bitmap_id().prop_map(|id| BitmapDelta::Destroy { id }),
    ]
}

fn frame_packet() -> impl Strategy<Value = FramePacket> {
    (
        any::<u64>(),
        any::<(u32, u32)>(),
        vec(bitmap_delta(), 0..6),
        vec(op(), 0..8),
    )
        .prop_map(|(frame_index, (width, height), bitmaps, ops)| FramePacket {
            frame_index,
            logical_size: Size::new(width, height),
            bitmaps,
            ops,
        })
}

fn audio_channel() -> impl Strategy<Value = AudioChannel> {
    prop_oneof![
        Just(AudioChannel::Bgm),
        Just(AudioChannel::Bgs),
        Just(AudioChannel::Me),
        Just(AudioChannel::Se),
    ]
}

fn audio_command() -> impl Strategy<Value = AudioCommand> {
    prop_oneof![
        (
            audio_channel(),
            vec(any::<u8>(), 0..64),
            any::<u8>(),
            any::<u16>()
        )
            .prop_map(|(channel, data, volume, pitch)| AudioCommand::Play {
                channel,
                data,
                volume,
                pitch,
            }),
        audio_channel().prop_map(|channel| AudioCommand::Stop { channel }),
        (audio_channel(), any::<u32>()).prop_map(|(channel, duration_ms)| AudioCommand::Fade {
            channel,
            duration_ms,
        }),
    ]
}

fn guest_to_host() -> impl Strategy<Value = GuestToHost> {
    prop_oneof![
        frame_packet().prop_map(GuestToHost::Frame),
        Just(GuestToHost::Freeze),
        (any::<u32>(), option::of(bitmap_id()), any::<u16>()).prop_map(
            |(duration, mask, vague)| GuestToHost::Transition {
                duration,
                mask,
                vague,
            }
        ),
        audio_command().prop_map(GuestToHost::Audio),
        any::<String>().prop_map(|text| GuestToHost::Modal { text }),
        any::<String>().prop_map(|title| GuestToHost::SetTitle { title }),
        Just(GuestToHost::Shutdown),
    ]
}

fn input_snapshot() -> impl Strategy<Value = InputSnapshot> {
    any::<u32>().prop_map(|buttons| InputSnapshot { buttons })
}

fn host_to_guest() -> impl Strategy<Value = HostToGuest> {
    prop_oneof![
        input_snapshot().prop_map(HostToGuest::Input),
        Just(HostToGuest::ModalDismissed),
        Just(HostToGuest::Tick),
        Just(HostToGuest::Quit),
    ]
}

/// Round trip, plus determinism checked three ways: encoding the same value twice, encoding
/// a clone, and encoding what was decoded.
fn check<T>(messages: &[T]) -> Result<(), TestCaseError>
where
    T: Message + Clone + PartialEq + std::fmt::Debug,
{
    let bytes = encode_stream(messages).unwrap();
    let decoded: Vec<T> = decode_stream(&bytes).unwrap();
    prop_assert_eq!(&decoded, messages);

    prop_assert_eq!(&encode_stream(messages).unwrap(), &bytes);
    prop_assert_eq!(&encode_stream(&messages.to_vec()).unwrap(), &bytes);
    prop_assert_eq!(&encode_stream(&decoded).unwrap(), &bytes);
    Ok(())
}

proptest! {
    #[test]
    fn guest_to_host_streams_round_trip_deterministically(
        messages in vec(guest_to_host(), 0..6)
    ) {
        check(&messages)?;
    }

    #[test]
    fn host_to_guest_streams_round_trip_deterministically(
        messages in vec(host_to_guest(), 0..32)
    ) {
        check(&messages)?;
    }

    #[test]
    fn input_recordings_round_trip_deterministically(
        snapshots in vec(input_snapshot(), 0..256)
    ) {
        check(&snapshots)?;
    }

    #[test]
    fn single_messages_round_trip(message in guest_to_host()) {
        check(std::slice::from_ref(&message))?;
    }

    /// Concatenation is the only framing: a stream's body is its messages' bodies in order.
    #[test]
    fn stream_body_is_the_concatenation_of_its_messages(
        messages in vec(guest_to_host(), 0..5)
    ) {
        let whole = encode_stream(&messages).unwrap();
        let mut expected = encode_stream::<GuestToHost, _>([]).unwrap();
        let header_len = expected.len();
        for message in &messages {
            let single = encode_stream([message]).unwrap();
            expected.extend_from_slice(&single[header_len..]);
        }
        prop_assert_eq!(whole, expected);
    }

    /// Arbitrary bytes never panic the decoder and never yield messages after an error.
    #[test]
    fn arbitrary_bytes_do_not_panic(bytes in vec(any::<u8>(), 0..256)) {
        let _ = decode_stream::<GuestToHost>(&bytes);
        let mut with_header = encode_stream::<GuestToHost, _>([]).unwrap();
        with_header.extend_from_slice(&bytes);
        let _ = decode_stream::<GuestToHost>(&with_header);
    }
}
