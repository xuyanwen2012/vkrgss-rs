//! The demo guest against a host played by the test.

mod common;

use common::{SMALL_DEMO, drive_demo, spawn_demo};
use vkxp_host::InputScript;
use vkxp_host::demo::{self, DemoOptions, SPRITE_SIZE, TITLE, TRANSITION_FRAMES};
use vkxp_proto::link::Disconnected;
use vkxp_proto::{
    BitmapDelta, Button, FramePacket, FrameRate, GuestToHost, HostToGuest, InputSnapshot, Op,
    OpKind, Size, Transform,
};

fn packets(messages: &[GuestToHost]) -> Vec<&FramePacket> {
    messages
        .iter()
        .filter_map(|message| match message {
            GuestToHost::Frame(packet) => Some(packet),
            _ => None,
        })
        .collect()
}

/// The transform of the sprite, which is the last quad of a packet.
fn sprite(packet: &FramePacket) -> Transform {
    packet
        .ops
        .iter()
        .rev()
        .find_map(|op| match op {
            Op::Quad(quad) => Some(quad.transform),
            _ => None,
        })
        .expect("every frame draws the sprite")
}

fn centre(size: Size) -> (i16, i16) {
    (
        ((size.width - SPRITE_SIZE) / 2) as i16,
        ((size.height - SPRITE_SIZE) / 2) as i16,
    )
}

#[test]
fn demo_sends_one_frame_per_tick_for_fifty_frames_and_shuts_down_on_quit() {
    let options = DemoOptions::default();
    let (host, demo) = spawn_demo(options);
    assert_eq!(
        host.recv(),
        Ok(GuestToHost::SetTitle {
            title: TITLE.to_owned()
        })
    );

    let mut ticks = 0;
    for index in 0..50 {
        // Exactly one message follows each tick, and it is the next frame.
        let message = host.recv().unwrap();
        let GuestToHost::Frame(packet) = message else {
            panic!("expected frame {index}, got {message:?}");
        };
        assert_eq!(packet.frame_index, index);
        assert_eq!(packet.logical_size, Size::new(640, 480));
        assert_eq!(packet.frame_rate, FrameRate::DEFAULT);
        assert_eq!(packet.bitmaps.is_empty(), index != 0, "frame {index}");
        assert_eq!(packet.ops.len(), 3, "frame {index}");
        if index < 49 {
            host.send(HostToGuest::Tick).unwrap();
            ticks += 1;
        }
    }
    assert_eq!(ticks, 49);

    // The fiftieth frame has no tick yet, so the demo must not have sent a fifty-first: the
    // answer to the quit request is the very next message.
    host.send(HostToGuest::Quit).unwrap();
    assert_eq!(host.recv(), Ok(GuestToHost::Shutdown));
    assert_eq!(host.recv(), Err(Disconnected));
    assert_eq!(demo.join().unwrap(), Ok(()));
}

#[test]
fn first_frame_creates_the_bitmaps_every_later_frame_uses() {
    let messages = drive_demo(SMALL_DEMO, &InputScript::new(), 3);
    let packets = packets(&messages);
    let created: Vec<_> = packets[0]
        .bitmaps
        .iter()
        .filter_map(|delta| match delta {
            BitmapDelta::Create { id, .. } => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(created.len(), 4);
    for packet in &packets {
        for op in &packet.ops {
            let Op::Quad(quad) = op else {
                panic!("an idle frame only draws quads, found {:?}", op.kind());
            };
            assert!(created.contains(&quad.bitmap));
        }
    }
}

#[test]
fn directions_move_the_sprite_and_shift_doubles_the_speed() {
    let script = InputScript::parse("0:RIGHT, 2:RIGHT+SHIFT, 3:UP, 4:LEFT+DOWN, 5:").unwrap();
    let messages = drive_demo(SMALL_DEMO, &script, 8);
    let (x, y) = centre(SMALL_DEMO.size);
    let expected = [
        (x, y),
        // Input sent before the tick of frame n shows in frame n + 1.
        (x + 4, y),
        (x + 8, y),
        (x + 16, y),
        (x + 16, y - 4),
        (x + 12, y),
        (x + 12, y),
        (x + 12, y),
    ];
    let positions: Vec<Transform> = packets(&messages).into_iter().map(sprite).collect();
    let expected: Vec<Transform> = expected
        .into_iter()
        .map(|(x, y)| Transform::place_at(x, y))
        .collect();
    assert_eq!(positions, expected);
}

#[test]
fn sprite_stops_at_the_edges_of_the_screen() {
    let script = InputScript::parse("0:LEFT+UP+SHIFT").unwrap();
    let messages = drive_demo(SMALL_DEMO, &script, 20);
    let last = sprite(packets(&messages).last().unwrap());
    assert_eq!(last, Transform::place_at(0, 0));
}

#[test]
fn c_held_adds_a_rectangle_effect() {
    let script = InputScript::parse("1:C, 3:").unwrap();
    let messages = drive_demo(SMALL_DEMO, &script, 6);
    let effects: Vec<bool> = packets(&messages)
        .iter()
        .map(|packet| packet.ops.iter().any(|op| op.kind() == OpKind::RectEffect))
        .collect();
    assert_eq!(effects, [false, false, true, true, false, false]);
}

#[test]
fn b_pressed_freezes_swaps_the_background_and_starts_a_transition_once() {
    let script = InputScript::parse("1:B, 4:").unwrap();
    let messages = drive_demo(SMALL_DEMO, &script, 6);
    let kinds: Vec<&str> = messages
        .iter()
        .map(|message| match message {
            GuestToHost::SetTitle { .. } => "title",
            GuestToHost::Frame(_) => "frame",
            GuestToHost::Freeze => "freeze",
            GuestToHost::Transition { .. } => "transition",
            _ => "other",
        })
        .collect();
    // Held for three frames, acted on once: between frame 1 and frame 2.
    assert_eq!(
        kinds,
        [
            "title",
            "frame",
            "frame",
            "freeze",
            "transition",
            "frame",
            "frame",
            "frame",
            "frame"
        ]
    );
    assert!(messages.contains(&GuestToHost::Transition {
        duration: TRANSITION_FRAMES,
        mask: None,
        vague: 0
    }));

    let background = |packet: &FramePacket| match &packet.ops[0] {
        Op::Quad(quad) => quad.bitmap,
        op => panic!("the background is a quad, found {:?}", op.kind()),
    };
    let packets = packets(&messages);
    assert_eq!(background(packets[0]), background(packets[1]));
    assert_ne!(background(packets[1]), background(packets[2]));
    assert_eq!(background(packets[2]), background(packets[5]));
}

#[test]
fn reset_request_puts_the_sprite_back_and_resets_the_pacing() {
    let (host, demo) = spawn_demo(SMALL_DEMO);
    let frame = |host: &vkxp_host::HostLink| loop {
        match host.recv().unwrap() {
            GuestToHost::Frame(packet) => return packet,
            GuestToHost::SetTitle { .. } => {}
            other => panic!("unexpected {other:?}"),
        }
    };
    let (x, y) = centre(SMALL_DEMO.size);

    frame(&host);
    let right = InputSnapshot::EMPTY.with(Button::Right);
    host.send(HostToGuest::Input(right)).unwrap();
    host.send(HostToGuest::Tick).unwrap();
    assert_eq!(sprite(&frame(&host)), Transform::place_at(x + 4, y));

    host.send(HostToGuest::Reset).unwrap();
    host.send(HostToGuest::Tick).unwrap();
    assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));
    let after = frame(&host);
    assert_eq!(sprite(&after), Transform::place_at(x, y));
    assert_eq!(after.frame_index, 2, "frames keep counting");

    // It keeps running, still holding RIGHT.
    host.send(HostToGuest::Tick).unwrap();
    assert_eq!(sprite(&frame(&host)), Transform::place_at(x + 4, y));

    host.send(HostToGuest::Quit).unwrap();
    assert_eq!(host.recv(), Ok(GuestToHost::Shutdown));
    assert_eq!(demo.join().unwrap(), Ok(()));
}

#[test]
fn demo_ends_with_a_disconnection_when_the_host_goes_away() {
    let (host, demo) = spawn_demo(SMALL_DEMO);
    host.recv().unwrap();
    host.recv().unwrap();
    drop(host);
    assert_eq!(demo.join().unwrap(), Err(Disconnected));
}

#[test]
fn same_input_gives_the_same_messages() {
    let script = InputScript::parse("1:RIGHT, 4:C+DOWN, 6:B, 7:, 9:LEFT").unwrap();
    let first = drive_demo(SMALL_DEMO, &script, 12);
    let second = drive_demo(SMALL_DEMO, &script, 12);
    assert_eq!(first, second);
}

#[test]
fn every_packet_is_accepted_by_the_reference_compositor() {
    let script = InputScript::parse("1:RIGHT+SHIFT, 3:C, 5:B, 6:UP").unwrap();
    let messages = drive_demo(SMALL_DEMO, &script, 10);
    let mut compositor = vkxp_compose_ref::Compositor::new();
    let mut frames = 0;
    for message in &messages {
        if let Some(frame) = compositor.handle(message).unwrap() {
            assert_eq!(frame.size, SMALL_DEMO.size);
            frames += 1;
        }
    }
    assert_eq!(frames, 10);
}

#[test]
#[should_panic(expected = "at least 64 by 64")]
fn demo_refuses_a_screen_too_small_for_its_sprite() {
    let (guest, _host) = vkxp_host::headless_link();
    let options = DemoOptions {
        size: Size::new(32, 32),
        ..DemoOptions::default()
    };
    let _ = demo::run(&guest, options);
}
