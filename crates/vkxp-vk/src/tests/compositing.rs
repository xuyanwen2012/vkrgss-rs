//! First frames on the device, the failed-packet scenarios of
//! `specs/frame-composition/spec.md`, and message handling.
//!
//! Expected pixels are written out by hand where that is practical, so these tests do not rest
//! on the reference alone; each stream is also compared with the reference.

use vkxp_compose_ref::fixtures::{self, bitmap, sprite};
use vkxp_proto::{
    AudioChannel, AudioCommand, BitmapDelta, BitmapId, Color, FramePacket, FrameRate, GuestToHost,
    Op, OpKind, Rect, Size,
};

use super::differential::{Pair, assert_stream_matches};
use super::harness::TestCompositor;
use crate::{BitmapError, Frame, SlotState, VkError};

const SIZE: Size = Size::new(20, 18);
const SCREEN: Rect = Rect::from_size(SIZE);
const BLACK: [u8; 4] = [0, 0, 0, 255];
const ID: BitmapId = BitmapId::new(0, 0);

fn packet(bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> FramePacket {
    FramePacket {
        frame_index: 0,
        logical_size: SIZE,
        frame_rate: FrameRate::DEFAULT,
        bitmaps,
        ops,
    }
}

fn frame(bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> GuestToHost {
    GuestToHost::Frame(packet(bitmaps, ops))
}

fn clear(color: [u8; 4]) -> Op {
    Op::Clear {
        clip: SCREEN,
        color: Color::from_rgba(color),
    }
}

/// Renders one packet on a fresh compositor and returns its frame.
fn render(bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> Frame {
    let mut compositor = TestCompositor::new();
    compositor.render(&packet(bitmaps, ops)).unwrap();
    compositor
        .read_frame()
        .unwrap()
        .expect("a frame was rendered")
}

fn assert_pixels(frame: &Frame, expected: impl Fn(u32, u32) -> [u8; 4]) {
    assert_eq!(frame.size, SIZE);
    assert_eq!(frame.rgba.len(), (SIZE.width * SIZE.height * 4) as usize);
    for y in 0..SIZE.height {
        for x in 0..SIZE.width {
            let offset = ((y * SIZE.width + x) * 4) as usize;
            assert_eq!(
                frame.rgba[offset..offset + 4],
                expected(x, y),
                "pixel ({x}, {y})"
            );
        }
    }
}

// Requirement: Initial frame state

#[test]
fn empty_op_list_is_opaque_black() {
    assert_pixels(&render(vec![], vec![]), |_, _| BLACK);
    assert_stream_matches(&[frame(vec![], vec![])]);
}

#[test]
fn previous_frame_does_not_show_through() {
    let mut compositor = TestCompositor::new();
    compositor
        .render(&packet(vec![], vec![clear([200, 100, 50, 7])]))
        .unwrap();
    compositor.render(&packet(vec![], vec![])).unwrap();
    assert_pixels(&compositor.read_frame().unwrap().unwrap(), |_, _| BLACK);

    assert_stream_matches(&[
        frame(vec![], vec![clear([200, 100, 50, 7])]),
        frame(vec![], vec![]),
    ]);
}

#[test]
fn no_frame_before_the_first_render() {
    let mut compositor = TestCompositor::new();
    assert_eq!(compositor.read_frame(), Ok(None));
}

// First frames

#[test]
fn full_screen_clear_sets_every_pixel_alpha_included() {
    let color = [12, 34, 56, 78];
    assert_pixels(&render(vec![], vec![clear(color)]), |_, _| color);
    assert_stream_matches(&[frame(vec![], vec![clear(color)])]);
}

#[test]
fn identity_quad_copies_texels() {
    let (width, height) = (8, 6);
    let texels = fixtures::gradient(width, height);
    let deltas: Vec<BitmapDelta> = bitmap(ID, width, height, texels.clone()).into();
    let ops = vec![Op::Quad(sprite(ID, width, height, 3, 2))];

    let rendered = render(deltas.clone(), ops.clone());
    assert_pixels(&rendered, |x, y| {
        if (3..3 + width).contains(&x) && (2..2 + height).contains(&y) {
            let offset = (((y - 2) * width + (x - 3)) * 4) as usize;
            // The gradient is opaque, so a normal blend copies it.
            [texels[offset], texels[offset + 1], texels[offset + 2], 255]
        } else {
            BLACK
        }
    });
    assert_stream_matches(&[frame(deltas, ops)]);
}

#[test]
fn frame_follows_a_change_of_logical_size() {
    let sized = |width, height| {
        GuestToHost::Frame(FramePacket {
            frame_index: 0,
            logical_size: Size::new(width, height),
            frame_rate: FrameRate::DEFAULT,
            bitmaps: vec![],
            ops: vec![Op::Clear {
                clip: Rect::new(1, 1, width, height),
                color: Color::new(9, 8, 7, 6),
            }],
        })
    };
    assert_stream_matches(&[sized(20, 18), sized(33, 5), sized(1, 1), sized(20, 18)]);
}

// Requirement: Failed packets produce no frame and keep bitmap state

#[test]
fn op_error_after_valid_deltas_keeps_the_bitmap() {
    let red = fixtures::solid(4, 4, Color::new(255, 0, 0, 255));
    let mut pair = Pair::new();
    let failed = pair
        .step(&frame(
            bitmap(ID, 4, 4, red).into(),
            vec![clear([1, 2, 3, 255]), Op::Plane { clip: SCREEN }],
        ))
        .unwrap();
    assert!(failed.is_err(), "the reserved op must fail the packet");

    // The next packet draws the bitmap the failed packet created.
    let drawn = pair
        .step(&frame(vec![], vec![Op::Quad(sprite(ID, 4, 4, 0, 0))]))
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(drawn.pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(drawn.pixel(4, 4), BLACK);
}

#[test]
fn op_error_produces_no_frame() {
    let mut compositor = TestCompositor::new();
    assert_eq!(
        compositor.render(&packet(vec![], vec![Op::TilemapLayer { clip: SCREEN }])),
        Err(VkError::UnsupportedOp {
            kind: OpKind::TilemapLayer,
            index: 0
        })
    );
    assert_eq!(compositor.read_frame(), Ok(None));

    // After a good frame, a failed packet leaves that frame in place.
    compositor
        .render(&packet(vec![], vec![clear([5, 6, 7, 8])]))
        .unwrap();
    assert!(
        compositor
            .render(&packet(
                vec![],
                vec![clear([9, 9, 9, 9]), Op::Plane { clip: SCREEN }]
            ))
            .is_err()
    );
    assert_pixels(&compositor.read_frame().unwrap().unwrap(), |_, _| {
        [5, 6, 7, 8]
    });
}

#[test]
fn delta_error_part_way_through_keeps_the_first_delta_only() {
    let (first, third) = (BitmapId::new(0, 0), BitmapId::new(2, 0));
    let create = |id| BitmapDelta::Create {
        id,
        width: 2,
        height: 2,
    };
    let mut pair = Pair::new();
    let failed = pair
        .step(&frame(
            vec![
                create(first),
                BitmapDelta::Destroy {
                    id: BitmapId::new(1, 0),
                },
                create(third),
            ],
            vec![clear([1, 2, 3, 4])],
        ))
        .unwrap();
    assert!(matches!(
        failed,
        Err(vkxp_compose_ref::ComposeError::InvalidDelta { index: 1, .. })
    ));

    // The first delta is applied: the bitmap can be updated and drawn.
    let white = fixtures::solid(2, 2, Color::new(255, 255, 255, 255));
    let update = BitmapDelta::Update {
        id: first,
        rect: Rect::new(0, 0, 2, 2),
        rgba: white,
    };
    let drawn = pair
        .step(&frame(
            vec![update],
            vec![Op::Quad(sprite(first, 2, 2, 5, 5))],
        ))
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(drawn.pixel(5, 5), [255, 255, 255, 255]);

    // The third is not: drawing it is an invalid reference, and its slot is still free.
    let stale = pair
        .step(&frame(vec![], vec![Op::Quad(sprite(third, 2, 2, 0, 0))]))
        .unwrap();
    assert!(stale.is_err());
    assert!(
        pair.step(&frame(vec![create(third)], vec![]))
            .unwrap()
            .is_ok()
    );
}

#[test]
fn delta_error_is_reported_before_any_op_error() {
    let mut compositor = TestCompositor::new();
    let id = BitmapId::new(4, 0);
    assert_eq!(
        compositor.render(&packet(
            vec![BitmapDelta::Destroy { id }],
            vec![Op::Plane { clip: SCREEN }]
        )),
        Err(VkError::InvalidDelta {
            index: 0,
            source: BitmapError::InvalidReference {
                id,
                slot: SlotState::Empty
            },
        })
    );
}

#[test]
fn invalid_logical_size_is_rejected_before_the_deltas_are_applied() {
    let mut pair = Pair::new();
    for size in [Size::new(0, 10), Size::new(10, 16385)] {
        let invalid = GuestToHost::Frame(FramePacket {
            frame_index: 0,
            logical_size: size,
            frame_rate: FrameRate::DEFAULT,
            bitmaps: vec![BitmapDelta::Create {
                id: ID,
                width: 1,
                height: 1,
            }],
            ops: vec![],
        });
        assert!(pair.step(&invalid).unwrap().is_err());
    }
    // The bitmap was not created, so its slot is free.
    let create = BitmapDelta::Create {
        id: ID,
        width: 1,
        height: 1,
    };
    assert!(pair.step(&frame(vec![create], vec![])).unwrap().is_ok());
}

// Message handling

/// Every kind of message: only frames render, and nothing else affects later frames.
#[test]
fn non_frame_messages_are_ignored_as_in_the_reference() {
    let others = [
        GuestToHost::Freeze,
        GuestToHost::Transition {
            duration: 20,
            mask: Some(ID),
            vague: 40,
        },
        GuestToHost::Transition {
            duration: 0,
            mask: None,
            vague: 0,
        },
        GuestToHost::Audio(AudioCommand::Play {
            channel: AudioChannel::Bgm,
            data: vec![1, 2, 3],
            volume: 100,
            pitch: 100,
        }),
        GuestToHost::Audio(AudioCommand::Stop {
            channel: AudioChannel::Se,
        }),
        GuestToHost::Audio(AudioCommand::Fade {
            channel: AudioChannel::Me,
            duration_ms: 500,
        }),
        GuestToHost::Modal {
            text: "hello".to_owned(),
        },
        GuestToHost::SetTitle {
            title: "title".to_owned(),
        },
        GuestToHost::Shutdown,
    ];

    // Before any frame: nothing renders and there is still no frame.
    let mut compositor = TestCompositor::new();
    for message in &others {
        assert_eq!(compositor.handle(message), Ok(false), "{message:?}");
    }
    assert_eq!(compositor.read_frame(), Ok(None));
    assert_eq!(compositor.handle(&frame(vec![], vec![])), Ok(true));
    drop(compositor);

    // Interleaved with frames, compared with the reference at every step.
    let gradient = fixtures::gradient(6, 6);
    let mut pair = Pair::new();
    let first = frame(
        bitmap(ID, 6, 6, gradient).into(),
        vec![clear([40, 40, 40, 255]), Op::Quad(sprite(ID, 6, 6, 1, 1))],
    );
    let expected = pair.step(&first).unwrap().unwrap().unwrap();
    for message in &others {
        assert_eq!(pair.step(message).unwrap(), Ok(None), "{message:?}");
        // The same frame again: the message in between changed nothing.
        let again = frame(
            vec![],
            vec![clear([40, 40, 40, 255]), Op::Quad(sprite(ID, 6, 6, 1, 1))],
        );
        assert_eq!(pair.step(&again).unwrap().unwrap().unwrap(), expected);
    }
    // An invalid frame message is an error from `handle`, as from `render`.
    assert!(
        pair.step(&frame(vec![], vec![Op::Plane { clip: SCREEN }]))
            .unwrap()
            .is_err()
    );
}
