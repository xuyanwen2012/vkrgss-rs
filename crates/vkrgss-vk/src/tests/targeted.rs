//! Targeted cases: the corners that random generation is unlikely to hit squarely.

use std::process::Command;

use vkrgss_compose_ref as reference;
use vkrgss_compose_ref::fixtures::{self, bitmap, sprite};
use vkrgss_proto::{
    BitmapDelta, BitmapId, BlendMode, Color, Fixed, FramePacket, FrameRate, GuestToHost, Op, Quad,
    Rect, RectEffect, Sampling, Size, Tone, Transform,
};

use super::differential::{Pair, assert_stream_matches, frame_difference};
use super::harness::{TestCompositor, require_isolation, test_device};
use crate::{
    BitmapError, DeviceChoice, DeviceError, Limit, SlotState, VkCompositor, VkError,
    available_devices,
};

fn packet(size: Size, bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> FramePacket {
    FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        brightness: 255,
        tables: vec![],
        bitmaps,
        ops,
    }
}

fn frame(size: Size, bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> GuestToHost {
    GuestToHost::Frame(packet(size, bitmaps, ops))
}

fn clear(clip: Rect, color: [u8; 4]) -> Op {
    Op::Clear {
        clip,
        color: Color::from_rgba(color),
    }
}

fn effect(clip: Rect, tone: Tone, color: [u8; 4]) -> Op {
    Op::RectEffect(RectEffect {
        clip,
        tone,
        color: Color::from_rgba(color),
    })
}

// Requirement: Tile binning is not observable

/// Six thousand ops whose result depends on their order, all covering one pixel: translucent
/// quads in every blend mode, tones, colours and partial clears, with clips of many sizes
/// around the pixel so that neighbouring tiles see different sub-lists.
#[test]
fn several_thousand_ops_on_one_pixel() {
    const OPS: u32 = 6000;
    let size = Size::new(40, 30);
    let (px, py) = (17, 9);
    let id = BitmapId::new(0, 0);
    let texels = fixtures::disc(8, Color::new(200, 120, 40, 255));
    let ops: Vec<Op> = (0..OPS)
        .map(|i| {
            // Always contains (px, py); reaches a different distance each time.
            let reach = (i % 23) as i32;
            let clip = Rect::new(px - reach, py - reach % 9, 2 * reach as u32 + 1, 12);
            match i % 7 {
                0 => clear(
                    Rect::new(px, py, 1 + i % 3, 1),
                    [(i % 251) as u8, (i % 13) as u8, 90, 255 - (i % 200) as u8],
                ),
                1..=3 => Op::Quad(Quad {
                    clip,
                    bitmap: id,
                    src: Rect::new(0, 0, 8, 8),
                    sampling: Sampling::Affine(Transform::place_at(
                        (px - 4) as i16,
                        (py - 4) as i16,
                    )),
                    tone: Tone::NEUTRAL,
                    color: Color::new(0, 0, 0, 0),
                    opacity: (40 + i % 200) as u8,
                    blend: [BlendMode::Normal, BlendMode::Add, BlendMode::Subtract]
                        [(i % 3) as usize],
                }),
                4 => effect(clip, Tone::new(3, -2, 1, (i % 40) as u8), [0, 0, 0, 0]),
                5 => effect(clip, Tone::NEUTRAL, [(i % 256) as u8, 20, 200, 9]),
                _ => effect(clip, Tone::new(-1, 2, -3, 1), [10, (i % 256) as u8, 30, 5]),
            }
        })
        .collect();
    assert_eq!(ops.len() as u32, OPS);
    assert!(ops.iter().all(|op| op.clip().contains(px, py)));

    let stream = [frame(size, bitmap(id, 8, 8, texels).into(), ops.clone())];
    assert_stream_matches(&stream);

    // The order matters: the same ops reversed give another pixel, so a backend that skipped
    // or reordered some could not have matched by accident.
    let forwards = reference::render_stream(&stream).unwrap().unwrap();
    let mut reversed = ops;
    reversed.reverse();
    let GuestToHost::Frame(first) = &stream[0] else {
        unreachable!()
    };
    let backwards = reference::render_stream(&[frame(size, first.bitmaps.clone(), reversed)])
        .unwrap()
        .unwrap();
    assert_ne!(
        forwards.pixel(px as u32, py as u32),
        backwards.pixel(px as u32, py as u32)
    );
}

#[test]
fn ops_entirely_off_screen_change_nothing() {
    let size = Size::new(50, 40);
    let id = BitmapId::new(0, 0);
    let deltas: Vec<BitmapDelta> = bitmap(id, 8, 8, fixtures::gradient(8, 8)).into();
    let beside = [
        Rect::new(50, 0, 10, 40),
        Rect::new(0, 40, 50, 10),
        Rect::new(-10, 0, 10, 40),
        Rect::new(0, -10, 50, 10),
        Rect::new(1000, 1000, 5, 5),
        Rect::new(i32::MIN, i32::MIN, 100, 100),
        Rect::new(i32::MAX, i32::MAX, u32::MAX, u32::MAX),
        Rect::new(10, 10, 0, 0),
    ];
    let mut ops = vec![clear(Rect::from_size(size), [10, 20, 30, 255])];
    for clip in beside {
        ops.push(clear(clip, [255, 0, 0, 255]));
        ops.push(effect(
            clip,
            Tone::new(100, 100, 100, 255),
            [0, 255, 0, 255],
        ));
        ops.push(Op::Quad(Quad {
            clip,
            ..sprite(id, 8, 8, 0, 0)
        }));
    }
    // A quad whose clip is the screen but whose transform puts it far away.
    ops.push(Op::Quad(Quad {
        clip: Rect::from_size(size),
        ..sprite(id, 8, 8, 500, -500)
    }));

    let mut pair = Pair::new();
    let rendered = pair
        .step(&frame(size, deltas, ops))
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        rendered
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| *pixel == [10, 20, 30, 255])
    );
    // Only off-screen ops, without a clear: the background.
    let only_off_screen = vec![clear(Rect::new(50, 40, 5, 5), [255, 0, 0, 255])];
    let background = pair
        .step(&frame(size, vec![], only_off_screen))
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(background.pixel(49, 39), reference::BACKGROUND);
}

/// A logical size that is not a multiple of the tile size in either direction: one partial
/// column and one partial row of tiles, drawn into right up to the last pixel.
#[test]
fn logical_size_641_by_479() {
    let size = Size::new(641, 479);
    let screen = Rect::from_size(size);
    let (board, shape) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let mut deltas: Vec<BitmapDelta> = bitmap(
        board,
        64,
        48,
        fixtures::checkerboard(
            64,
            48,
            4,
            Color::new(32, 32, 48, 255),
            Color::new(224, 224, 208, 255),
        ),
    )
    .into();
    deltas.extend(bitmap(
        shape,
        40,
        40,
        fixtures::disc(40, Color::new(255, 160, 64, 255)),
    ));
    // The checkerboard magnified to just cover 641 by 479.
    let magnified = Quad {
        clip: screen,
        sampling: Sampling::Affine(Transform {
            xx: Fixed::from_ratio(64, 641).unwrap(),
            yy: Fixed::from_ratio(48, 479).unwrap(),
            ..Transform::IDENTITY
        }),
        ..sprite(board, 64, 48, 0, 0)
    };
    let ops = vec![
        Op::Quad(magnified),
        // Over the right and bottom edges, into the partial tiles.
        Op::Quad(Quad {
            clip: screen,
            opacity: 192,
            blend: BlendMode::Add,
            ..sprite(shape, 40, 40, 620, 460)
        }),
        effect(
            Rect::new(600, 0, 41, 479),
            Tone::new(40, -30, 0, 128),
            [0, 0, 255, 40],
        ),
        effect(Rect::new(0, 470, 641, 9), Tone::NEUTRAL, [255, 255, 0, 96]),
        clear(Rect::new(640, 478, 1, 1), [1, 2, 3, 4]),
        clear(Rect::new(640, 0, 1, 1), [5, 6, 7, 8]),
        clear(Rect::new(0, 478, 1, 1), [9, 10, 11, 12]),
    ];
    let mut pair = Pair::new();
    let rendered = pair
        .step(&frame(size, deltas, ops))
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(rendered.size, size);
    assert_eq!(rendered.pixel(640, 478), [1, 2, 3, 4]);
    assert_eq!(rendered.pixel(640, 0), [5, 6, 7, 8]);
    assert_eq!(rendered.pixel(0, 478), [9, 10, 11, 12]);
}

/// Clips that end on, start on, straddle and sit inside tile boundaries, at a size that is a
/// multiple of the tile size and at sizes one pixel either side of it.
#[test]
fn ops_that_span_and_touch_tile_boundaries() {
    for size in [Size::new(64, 48), Size::new(65, 49), Size::new(63, 47)] {
        let mut ops = vec![clear(Rect::from_size(size), [20, 20, 20, 255])];
        let mut shade = 0u8;
        for (x, width) in [
            (0, 16),
            (16, 16),
            (15, 2),
            (31, 1),
            (32, 1),
            (8, 40),
            (48, 17),
        ] {
            for (y, height) in [
                (0, 16),
                (16, 16),
                (15, 2),
                (31, 1),
                (32, 1),
                (4, 40),
                (47, 2),
            ] {
                shade = shade.wrapping_add(37);
                ops.push(effect(
                    Rect::new(x, y, width, height),
                    Tone::new(i16::from(shade % 9) - 4, 2, -1, shade % 5),
                    [shade, 255 - shade, 7, 23],
                ));
            }
        }
        assert_stream_matches(&[frame(size, vec![], ops)]);
    }
}

// Requirement: Quad sampling position

/// Transforms built from the largest and smallest coefficients, placed so that some pixels do
/// land inside the source rectangle.
#[test]
fn extreme_coefficients_select_the_reference_texels() {
    let size = Size::new(70, 50);
    let id = BitmapId::new(0, 0);
    let deltas: Vec<BitmapDelta> = bitmap(id, 64, 64, fixtures::gradient(64, 64)).into();
    let (max, min) = (i32::MAX, i32::MIN);
    // `u = c * (px + 1/2) + t`: with the largest coefficient one pixel step is 32768 texels,
    // and `t = -c / 2` brings the first pixel back to texel 0.
    let cases = [
        [max, 0, 0, max, -(max / 2), -(max / 2)],
        [min, 0, 0, min, 16384 << 16 >> 1, 16384 << 16 >> 1],
        [max, max, min, min, min, max],
        [min, max, max, min, 0, 0],
        [0, max, min, 0, min, max],
        [1, 0, 0, 1, 0, 0],
        [-1, 0, 0, -1, 1 << 16, 1 << 16],
        [0, 0, 0, 0, 5 << 16, 63 << 16],
        [0, 0, 0, 0, max, max],
        [0, 0, 0, 0, min, min],
        [65535, 0, 0, 65537, 32767, -32768],
    ];
    let mut ops = vec![clear(Rect::from_size(size), [0, 0, 0, 0])];
    for (index, bits) in cases.iter().enumerate() {
        let coefficient = |i: usize| Fixed::from_bits(bits[i]);
        ops.push(Op::Quad(Quad {
            clip: Rect::from_size(size),
            bitmap: id,
            src: Rect::new(0, 0, 64, 64),
            sampling: Sampling::Affine(Transform {
                xx: coefficient(0),
                xy: coefficient(1),
                yx: coefficient(2),
                yy: coefficient(3),
                tx: coefficient(4),
                ty: coefficient(5),
            }),
            tone: Tone::NEUTRAL,
            color: Color::new(0, 0, 0, 0),
            opacity: 200,
            blend: [BlendMode::Normal, BlendMode::Add, BlendMode::Subtract][index % 3],
        }));
    }
    let mut pair = Pair::new();
    let rendered = pair
        .step(&frame(size, deltas, ops))
        .unwrap()
        .unwrap()
        .unwrap();
    // Something was drawn: the test would mean little if every quad missed the screen.
    assert_ne!(rendered.pixel(0, 0), [0, 0, 0, 0]);
}

// guest-host-protocol, Requirement: Every op kind is implemented

#[test]
fn packet_with_one_op_of_each_kind_is_accepted_and_matches_the_reference() {
    let message = fixtures::one_op_of_each_kind();
    let mut pair = Pair::new();
    assert!(pair.step(&message).unwrap().unwrap().is_some());
}

// Requirement: Errors match the reference

/// A destroyed bitmap whose slot was reused: the old id is an error, and the new occupant is
/// not drawn in its place.
#[test]
fn stale_reference_to_a_reused_slot_does_not_draw_the_new_occupant() {
    let size = Size::new(16, 16);
    let screen = Rect::from_size(size);
    let (old, new) = (BitmapId::new(3, 0), BitmapId::new(3, 1));
    let red = fixtures::solid(4, 4, Color::new(255, 0, 0, 255));
    let blue = fixtures::solid(4, 4, Color::new(0, 0, 255, 255));

    let mut pair = Pair::new();
    let first = frame(
        size,
        bitmap(old, 4, 4, red).into(),
        vec![
            clear(screen, [50, 50, 50, 255]),
            Op::Quad(sprite(old, 4, 4, 0, 0)),
        ],
    );
    assert!(pair.step(&first).unwrap().is_ok());

    let mut deltas = vec![BitmapDelta::Destroy { id: old }];
    deltas.extend(bitmap(new, 4, 4, blue));
    let stale = frame(size, deltas, vec![Op::Quad(sprite(old, 4, 4, 0, 0))]);
    // `step` also checks that the frame of the first packet is still in place.
    assert_eq!(
        pair.step(&stale).unwrap(),
        Err(reference::ComposeError::InvalidOpBitmap {
            index: 0,
            source: reference::BitmapError::InvalidReference {
                id: old,
                slot: reference::SlotState::Occupied { generation: 1 },
            },
        })
    );
    // The new occupant is there under its own id.
    let drawn = pair
        .step(&frame(
            size,
            vec![],
            vec![Op::Quad(sprite(new, 4, 4, 0, 0))],
        ))
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(drawn.pixel(0, 0), [0, 0, 255, 255]);
}

// Requirement: Device limits are reported

/// Feeds the same packets to a reference compositor and to the backend, where the backend is
/// expected to refuse `over_limit` with `expected` and the reference to accept it, and then
/// checks that both render `later` identically.
fn assert_limit_then_recovery(
    compositor: &mut TestCompositor,
    before: &FramePacket,
    over_limit: &FramePacket,
    expected: VkError,
    later: &FramePacket,
) {
    let mut reference = reference::Compositor::new();
    let first = reference.render(before).unwrap();
    compositor.render(before).unwrap();

    assert!(
        reference.render(over_limit).is_ok(),
        "the packet must be valid for the reference"
    );
    assert_eq!(compositor.render(over_limit), Err(expected));
    // No partial or different frame: the previous one is still there.
    let kept = compositor.read_frame().unwrap().unwrap();
    assert_eq!(frame_difference(&first, &kept), None);

    let wanted = reference.render(later).unwrap();
    compositor.render(later).unwrap();
    let rendered = compositor.read_frame().unwrap().unwrap();
    assert_eq!(frame_difference(&wanted, &rendered), None);
}

#[test]
fn bitmap_larger_than_the_device_supports_is_a_limit_error_and_recoverable() {
    let size = Size::new(24, 24);
    let screen = Rect::from_size(size);
    let mut compositor = TestCompositor::new();
    let maximum = compositor.max_image_dimension();
    assert!(
        maximum < (1 << 28),
        "a bitmap one pixel wider must still be valid for the reference"
    );
    let (small, wide, tall) = (
        BitmapId::new(0, 0),
        BitmapId::new(1, 0),
        BitmapId::new(2, 0),
    );
    let before = packet(
        size,
        bitmap(small, 6, 6, fixtures::gradient(6, 6)).into(),
        vec![clear(screen, [9, 9, 9, 255])],
    );
    let create = |id, width, height| BitmapDelta::Create { id, width, height };
    for (id, width, height) in [(wide, maximum + 1, 1), (tall, 1, maximum + 1)] {
        // The oversized bitmap is created after a delta that stays applied and is never drawn.
        let marker = BitmapDelta::Update {
            id: small,
            rect: Rect::new(0, 0, 1, 1),
            rgba: vec![255, 255, 255, 255],
        };
        let over_limit = packet(
            size,
            vec![marker, create(id, width, height)],
            vec![clear(screen, [200, 0, 0, 255])],
        );
        let later = packet(
            size,
            vec![],
            vec![
                clear(screen, [0, 40, 80, 255]),
                Op::Quad(sprite(small, 6, 6, 3, 3)),
            ],
        );
        assert_limit_then_recovery(
            &mut compositor,
            &before,
            &over_limit,
            VkError::Limit {
                limit: Limit::BitmapDimension,
                requested: u64::from(maximum) + 1,
                maximum: u64::from(maximum),
            },
            &later,
        );
        // The refused bitmap does not exist, and its slot can be used.
        assert!(matches!(
            compositor.render(&packet(size, vec![BitmapDelta::Destroy { id }], vec![])),
            Err(VkError::InvalidDelta {
                index: 0,
                source: BitmapError::InvalidReference {
                    slot: SlotState::Empty,
                    ..
                },
            })
        ));
        compositor
            .render(&packet(size, vec![create(id, 2, 2)], vec![]))
            .unwrap();
        compositor
            .render(&packet(size, vec![BitmapDelta::Destroy { id }], vec![]))
            .unwrap();
        compositor
            .render(&packet(
                size,
                vec![BitmapDelta::Destroy { id: small }],
                vec![],
            ))
            .unwrap();
        compositor.render(&before).unwrap();
        // Back to the state `before` leaves, for the next round.
        compositor
            .render(&packet(
                size,
                vec![BitmapDelta::Destroy { id: small }],
                vec![],
            ))
            .unwrap();
    }
    let error = VkError::Limit {
        limit: Limit::BitmapDimension,
        requested: 20000,
        maximum: 16384,
    };
    assert_eq!(
        error.to_string(),
        "limit exceeded: bitmap width or height is at most 16384, requested 20000"
    );
}

#[test]
fn bitmap_slot_beyond_the_table_is_a_limit_error_and_recoverable() {
    let size = Size::new(24, 24);
    let screen = Rect::from_size(size);
    let mut compositor = TestCompositor::new();
    let (small, beyond) = (BitmapId::new(65535, 0), BitmapId::new(65536, 0));
    let before = packet(
        size,
        bitmap(small, 6, 6, fixtures::gradient(6, 6)).into(),
        vec![clear(screen, [9, 9, 9, 255])],
    );
    let over_limit = packet(
        size,
        vec![BitmapDelta::Create {
            id: beyond,
            width: 2,
            height: 2,
        }],
        vec![clear(screen, [200, 0, 0, 255])],
    );
    // The last slot of the table works, bitmap array entry 65535 included.
    let later = packet(
        size,
        vec![],
        vec![
            clear(screen, [0, 40, 80, 255]),
            Op::Quad(sprite(small, 6, 6, 10, 10)),
        ],
    );
    assert_limit_then_recovery(
        &mut compositor,
        &before,
        &over_limit,
        VkError::Limit {
            limit: Limit::BitmapSlots,
            requested: 65536,
            maximum: 65535,
        },
        &later,
    );
}

// Requirement: Device selection, on the real devices of this machine

#[test]
fn named_device_that_does_not_exist_fails_and_lists_the_devices() {
    require_isolation();
    let devices = available_devices().unwrap();
    assert!(!devices.is_empty());
    for choice in [
        DeviceChoice::Name("no such device, surely".to_owned()),
        DeviceChoice::Index(devices.len()),
    ] {
        let error = match VkCompositor::new(choice.clone()) {
            Err(VkError::Device(error)) => error,
            other => panic!("expected a device error, got {:?}", other.map(|_| ())),
        };
        assert_eq!(
            error,
            DeviceError::NotFound {
                choice: choice.to_string(),
                devices: devices.clone(),
            }
        );
        let text = error.to_string();
        for device in &devices {
            assert!(text.contains(&device.name), "{text}");
        }
    }
}

#[test]
fn explicit_choice_selects_exactly_that_device() {
    require_isolation();
    let devices = available_devices().unwrap();
    let wanted = test_device();
    let compositor = VkCompositor::new(wanted.clone()).unwrap();
    let chosen = compositor.device();
    assert_eq!(devices[chosen.index], *chosen);
    if let DeviceChoice::Name(name) = wanted {
        assert!(chosen.name.to_lowercase().contains(&name.to_lowercase()));
    }
}

// Requirement: Device requirements

/// Runs one rendering test again in a child process that has no window system to connect to.
#[test]
fn renders_without_a_window_system_connection() {
    const TEST: &str = "tests::compositing::identity_quad_copies_texels";
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--test-threads", "1"])
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env_remove("XAUTHORITY")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "the child failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Requirement: Clean under validation, on the failure paths too

/// A packet with uploads and ops needs five buffers for its one submission: the bitmap staging
/// buffer and the four storage buffers of the frame. Whichever of them cannot be created, or
/// later cannot be replaced by a larger one, nothing may leak: every compositor here is
/// dropped under the harness, which fails the test when the validation layer reports a leaked
/// object. The backend also has to stay usable, with the frame it had and the bitmaps in a
/// drawable state. Also scenario "A buffer cannot be had" of "Per-frame buffers are reused".
#[test]
fn failed_buffer_allocation_releases_the_buffers_before_it() {
    let size = Size::new(40, 30);
    let screen = Rect::from_size(size);
    let id = BitmapId::new(0, 0);
    let deltas: Vec<BitmapDelta> = bitmap(id, 8, 8, fixtures::gradient(8, 8)).into();
    let upload = deltas[1].clone();
    let ops = vec![
        clear(screen, [10, 20, 30, 255]),
        Op::Quad(sprite(id, 8, 8, 5, 5)),
    ];
    let mut reference = reference::Compositor::new();
    let wanted = reference
        .render(&packet(size, deltas.clone(), ops.clone()))
        .unwrap();
    let large = Size::new(100, 70);
    let large_ops: Vec<Op> = (0..40)
        .map(|n| clear(Rect::new(n, n, 60, 30), [n as u8 * 6, 20, 30, 255]))
        .chain([Op::Quad(sprite(id, 8, 8, 50, 40))])
        .collect();
    let wanted_large = reference
        .render(&packet(large, vec![], large_ops.clone()))
        .unwrap();
    let injected = |what| {
        Err(VkError::Memory {
            what,
            message: "injected failure".to_owned(),
        })
    };

    let buffers = [
        "bitmap staging",
        "op records",
        "tile headers",
        "tile op indices",
        "tilemap records",
    ];
    for (successes, what) in buffers.into_iter().enumerate() {
        let mut compositor = TestCompositor::new();
        compositor.fail_frame_buffer_after(successes as u32);
        assert_eq!(
            compositor.render(&packet(size, deltas.clone(), ops.clone())),
            injected(what),
        );
        assert_eq!(compositor.read_frame(), Ok(None), "no frame");

        // The bitmap exists and can be drawn. When the staging buffer was the one that failed
        // its pixels were not uploaded, so they are sent again; otherwise they are there.
        let again = if successes == 0 {
            vec![upload.clone()]
        } else {
            vec![]
        };
        compositor
            .render(&packet(size, again, ops.clone()))
            .unwrap();
        for _ in 0..2 {
            let rendered = compositor.read_frame().unwrap().unwrap();
            assert_eq!(frame_difference(&wanted, &rendered), None, "after {what}");
            compositor
                .render(&packet(size, vec![], ops.clone()))
                .unwrap();
        }

        // A failure with a frame on the device, where the buffers exist and a frame needs
        // larger ones: twice the upload, more ops and more tiles. That frame stays, and the
        // frame that failed renders once the buffer can be had. It has no tilemap, so the
        // buffer of the tilemap records does not grow and is left out here.
        if what == "tilemap records" {
            continue;
        }
        compositor.fail_frame_buffer_after(successes as u32);
        let larger = packet(
            large,
            vec![upload.clone(), upload.clone()],
            large_ops.clone(),
        );
        assert_eq!(compositor.render(&larger), injected(what));
        let kept = compositor.read_frame().unwrap().unwrap();
        assert_eq!(frame_difference(&wanted, &kept), None, "after {what}");
        compositor.render(&larger).unwrap();
        let rendered = compositor.read_frame().unwrap().unwrap();
        assert_eq!(
            frame_difference(&wanted_large, &rendered),
            None,
            "after {what}"
        );
    }
}

/// The same for a packet without uploads, where the op records are the first of only four
/// buffers, and with no earlier frame to fall back on.
#[test]
fn failed_frame_buffer_allocation_without_uploads_leaks_nothing() {
    let size = Size::new(33, 17);
    let ops = vec![clear(Rect::from_size(size), [1, 2, 3, 4])];
    for successes in 0..4 {
        let mut compositor = TestCompositor::new();
        compositor.fail_frame_buffer_after(successes);
        assert!(matches!(
            compositor.render(&packet(size, vec![], ops.clone())),
            Err(VkError::Memory { .. })
        ));
        assert_eq!(compositor.read_frame(), Ok(None));
    }
}

// Requirement: Per-frame buffers are reused

/// A frame with an upload, a quad and `clears` clears, and the bitmap the quad draws.
fn buffer_frame(size: Size, id: BitmapId, clears: i32) -> FramePacket {
    let upload = bitmap(id, 8, 8, fixtures::gradient(8, 8))[1].clone();
    let ops = (0..clears)
        .map(|n| {
            clear(
                Rect::new(n, n / 2, 20, 10),
                [(n % 50) as u8 * 5, 90, 30, 255],
            )
        })
        .chain([Op::Quad(sprite(id, 8, 8, 5, 5))])
        .collect();
    packet(size, vec![upload], ops)
}

/// Scenario "Steady frames".
#[test]
fn steady_frames_create_no_buffer_after_the_first() {
    let size = Size::new(40, 30);
    let id = BitmapId::new(0, 0);
    let create = bitmap(id, 8, 8, fixtures::gradient(8, 8))[0].clone();
    let steady = buffer_frame(size, id, 3);
    let mut reference = reference::Compositor::new();
    let mut compositor = TestCompositor::new();
    reference
        .render(&packet(size, vec![create.clone()], vec![]))
        .unwrap();
    compositor
        .render(&packet(size, vec![create], vec![]))
        .unwrap();

    let wanted = reference.render(&steady).unwrap();
    compositor.render(&steady).unwrap();
    let created = compositor.frame_buffers_created();
    for round in 0..20 {
        let wanted = reference.render(&steady).unwrap();
        compositor.render(&steady).unwrap();
        let rendered = compositor.read_frame().unwrap().unwrap();
        assert_eq!(frame_difference(&wanted, &rendered), None, "round {round}");
    }
    assert_eq!(compositor.frame_buffers_created(), created);
    let rendered = compositor.read_frame().unwrap().unwrap();
    assert_eq!(frame_difference(&wanted, &rendered), None);
}

/// Scenario "A larger frame": the buffers grow for it, and the small frame after it reads
/// only its own part of what they hold. The harness checks the validation layer.
#[test]
fn larger_frame_and_the_small_one_after_it_match_the_reference() {
    let id = BitmapId::new(0, 0);
    let create = bitmap(id, 8, 8, fixtures::gradient(8, 8))[0].clone();
    let small = buffer_frame(Size::new(40, 30), id, 2);
    let large = buffer_frame(Size::new(200, 150), id, 300);
    let mut reference = reference::Compositor::new();
    let mut compositor = TestCompositor::new();
    let setup = packet(Size::new(40, 30), vec![create], vec![]);
    reference.render(&setup).unwrap();
    compositor.render(&setup).unwrap();

    let mut created = Vec::new();
    for (what, packet) in [
        ("small", &small),
        ("large", &large),
        ("small again", &small),
    ] {
        let wanted = reference.render(packet).unwrap();
        compositor.render(packet).unwrap();
        let rendered = compositor.read_frame().unwrap().unwrap();
        assert_eq!(frame_difference(&wanted, &rendered), None, "{what}");
        created.push(compositor.frame_buffers_created());
    }
    assert!(created[1] > created[0], "the large frame needs more room");
    assert_eq!(created[2], created[1], "nothing shrinks");
}

// Requirement: Stretch sampling of quads

/// A bitmap one row high in which every texel differs from its neighbours.
fn row_bitmap(id: BitmapId, width: u32) -> Vec<BitmapDelta> {
    let rgba = (0..width)
        .flat_map(|x| [(x % 256) as u8, (x / 256) as u8, (x % 7) as u8 * 30, 255])
        .collect();
    bitmap(id, width, 1, rgba).into()
}

fn stretch(clip: Rect, bitmap: BitmapId, src: Rect, dest: Rect, mirror: bool) -> Op {
    Op::Quad(Quad {
        clip,
        bitmap,
        src,
        sampling: Sampling::Stretch { dest, mirror },
        tone: Tone::NEUTRAL,
        color: Color::new(0, 0, 0, 0),
        opacity: 255,
        blend: BlendMode::Normal,
    })
}

/// Every destination width from 1 to 200 for a source of 16 texels, which contains every
/// measured zoom of `sprite-zoom-samples.txt`, plain and mirrored, one row each.
#[test]
fn stretch_quads_of_every_small_width_match_the_reference() {
    let size = Size::new(210, 400);
    let screen = Rect::from_size(size);
    let id = BitmapId::new(0, 0);
    let mut ops = vec![clear(screen, [10, 20, 30, 255])];
    for width in 1..=200u32 {
        for mirror in [false, true] {
            let row = (width - 1) * 2 + u32::from(mirror);
            let dest = Rect::new(5, row as i32, width, 1);
            ops.push(stretch(screen, id, Rect::new(0, 0, 16, 1), dest, mirror));
        }
    }
    assert_stream_matches(&[frame(size, row_bitmap(id, 16), ops)]);
}

/// Destination rectangles that begin or end far outside the screen, up to the ends of the
/// coordinate range, with clips of their own.
#[test]
fn stretch_quads_far_outside_the_screen_match_the_reference() {
    let size = Size::new(70, 50);
    let screen = Rect::from_size(size);
    let id = BitmapId::new(0, 0);
    let src = Rect::new(3, 0, 40, 1);
    let dests = [
        Rect::new(-100, 0, 300, 50),
        Rect::new(-100_000, -100_000, 200_050, 200_020),
        Rect::new(i32::MIN, i32::MIN, u32::MAX, u32::MAX),
        Rect::new(i32::MIN, 10, 1 << 31, 5),
        Rect::new(i32::MIN, 10, (1 << 31) + 20, 5),
        Rect::new(-1, -1, u32::MAX, u32::MAX),
        Rect::new(69, 49, u32::MAX, u32::MAX),
        Rect::new(30, i32::MIN + 7, 9, u32::MAX),
        // Beside the screen, or without pixels.
        Rect::new(70, 0, 500, 50),
        Rect::new(-500, 0, 500, 50),
        Rect::new(i32::MAX, i32::MAX, u32::MAX, u32::MAX),
        Rect::new(5, 5, 0, 9),
    ];
    let mut messages = vec![frame(size, row_bitmap(id, 64), vec![])];
    for dest in dests {
        for mirror in [false, true] {
            for clip in [
                screen,
                Rect::new(13, 7, 31, 22),
                Rect::new(-40, -40, 60, 60),
            ] {
                let ops = vec![
                    clear(screen, [10, 20, 30, 255]),
                    stretch(clip, id, src, dest, mirror),
                ];
                messages.push(frame(size, vec![], ops));
            }
        }
    }
    assert_stream_matches(&messages);
}

/// Sources and destinations wider than 4096, where a step or a product no longer fits 32 bits.
#[test]
fn stretch_quads_wider_than_4096_match_the_reference() {
    let size = Size::new(300, 20);
    let screen = Rect::from_size(size);
    let (small, wide) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let wide_width = 5000u32;
    let mut deltas = row_bitmap(small, 16);
    deltas.extend(row_bitmap(wide, wide_width));
    let mut ops = vec![clear(screen, [10, 20, 30, 255])];
    let mut row = 0;
    let mut push = |bitmap, src: Rect, x: i32, width: u32, mirror| {
        let dest = Rect::new(x, row, width, 1);
        ops.push(stretch(screen, bitmap, src, dest, mirror));
        row += 1;
    };
    let all = Rect::new(0, 0, wide_width, 1);
    // A wide source squeezed and unscaled, a narrow one stretched wide, and both at once.
    push(wide, all, 0, 300, false);
    push(wide, all, 0, 3, false);
    push(wide, all, 40, 1, true);
    push(wide, all, -4800, wide_width, false);
    push(wide, all, -4800, wide_width, true);
    push(wide, Rect::new(4097, 0, 900, 1), -3, 4097, false);
    push(small, Rect::new(0, 0, 16, 1), -2000, 4097, false);
    push(small, Rect::new(0, 0, 16, 1), -99_000, 100_000, true);
    push(small, Rect::new(0, 0, 16, 1), -(1 << 30), u32::MAX, false);
    push(wide, all, -6000, 12_345, false);
    push(wide, all, -70_000, 100_003, true);
    push(wide, all, -(1 << 30), u32::MAX, false);
    push(wide, all, i32::MIN, u32::MAX, true);
    assert_stream_matches(&[frame(size, deltas, ops)]);
}

/// Stretch quads keep their place in the op order and take tone, colour, opacity and blend
/// mode like any quad.
#[test]
fn stretch_quads_blend_like_other_quads() {
    let size = Size::new(64, 48);
    let screen = Rect::from_size(size);
    let (disc, grid) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let mut deltas: Vec<BitmapDelta> =
        bitmap(disc, 8, 8, fixtures::disc(8, Color::new(200, 120, 40, 255))).into();
    deltas.extend(bitmap(grid, 9, 7, fixtures::gradient(9, 7)));
    let mut ops = vec![clear(screen, [90, 60, 30, 255])];
    for (i, blend) in [BlendMode::Normal, BlendMode::Add, BlendMode::Subtract]
        .into_iter()
        .enumerate()
    {
        let x = i as i32 * 20;
        ops.push(Op::Quad(Quad {
            clip: screen,
            bitmap: grid,
            src: Rect::new(1, 1, 7, 5),
            sampling: Sampling::Stretch {
                dest: Rect::new(x - 3, 2, 23, 31),
                mirror: i == 1,
            },
            tone: Tone::new(20, -30, 40, 100),
            color: Color::new(10, 200, 30, 90),
            opacity: 180,
            blend,
        }));
        ops.push(Op::Quad(sprite(disc, 8, 8, x as i16 + 4, 10)));
        ops.push(Op::Quad(Quad {
            clip: Rect::new(x, 20, 15, 25),
            bitmap: disc,
            src: Rect::new(0, 0, 8, 8),
            sampling: Sampling::Stretch {
                dest: Rect::new(x - 2, 14, 21, 40),
                mirror: false,
            },
            tone: Tone::NEUTRAL,
            color: Color::new(0, 0, 0, 0),
            opacity: 120,
            blend,
        }));
    }
    assert_stream_matches(&[frame(size, deltas, ops)]);
}

/// Window-mode quads whose visible part is odd or even, cut by clips and by the frame, placed
/// on whole pixels or not, over opaque and translucent pixels: the one pixel per row that
/// takes the normal formula is the same on both backends.
#[test]
fn window_quads_agree_on_the_last_pixel_of_odd_rows() {
    let size = Size::new(70, 40);
    let screen = Rect::from_size(size);
    let (even, odd) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let texels = |width: u32| -> Vec<u8> {
        (0..width * 6)
            .flat_map(|i| {
                [
                    (i * 37) as u8,
                    (i * 11) as u8,
                    255 - i as u8,
                    (i * 29) as u8,
                ]
            })
            .collect()
    };
    let mut deltas: Vec<BitmapDelta> = bitmap(even, 12, 6, texels(12)).into();
    deltas.extend(bitmap(odd, 11, 6, texels(11)));
    let mut ops = vec![
        clear(screen, [161, 90, 30, 255]),
        clear(Rect::new(0, 20, 70, 20), [200, 100, 50, 77]),
    ];
    let mut push = |bitmap, width, clip, sampling| {
        ops.push(Op::Quad(Quad {
            clip,
            bitmap,
            src: Rect::new(0, 0, width, 6),
            sampling,
            tone: Tone::NEUTRAL,
            color: Color::new(0, 0, 0, 0),
            opacity: 131,
            blend: BlendMode::Window,
        }));
    };
    for (row, y) in [1i16, 21].into_iter().enumerate() {
        let band = Rect::new(0, i32::from(y) - 1, 70, 16);
        for (bitmap, width) in [(even, 12), (odd, 11)] {
            let y = y + if width == 12 { 0 } else { 8 };
            push(bitmap, width, band, Sampling::place_at(-5, y));
            push(bitmap, width, band, Sampling::place_at(8, y));
            push(
                bitmap,
                width,
                Rect::new(23, 0, 7, 40),
                Sampling::place_at(20, y),
            );
            push(
                bitmap,
                width,
                Rect::new(30, 0, 8, 40),
                Sampling::place_at(33, y),
            );
            push(bitmap, width, band, Sampling::place_at(62, y));
            // Half a pixel off, and scaled: no pixel is an exception.
            let moved = Transform {
                tx: Fixed::from_bits(-(46 << 16) + (1 << 15)),
                ..Transform::place_at(46, y)
            };
            push(bitmap, width, band, Sampling::Affine(moved));
        }
        let dest = Rect::new(47, i32::from(y) + 8, 11, 6);
        let stretch = Sampling::Stretch {
            dest,
            mirror: row == 1,
        };
        push(odd, 11, band, stretch);
    }
    assert_stream_matches(&[frame(size, deltas, ops)]);
}
