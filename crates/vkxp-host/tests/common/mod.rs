//! Helpers shared by the integration tests.
#![allow(dead_code)]

use std::thread;

use vkxp_host::demo::{self, DemoOptions};
use vkxp_host::{HostLink, InputScript, headless_link};
use vkxp_proto::link::Disconnected;
use vkxp_proto::{
    Color, FramePacket, FrameRate, GuestToHost, HostToGuest, InputSnapshot, Op, Rect, Size,
};
use vkxp_vk::DeviceChoice;

/// The device the tests composite on: llvmpipe, unless `VKXP_TEST_DEVICE` names another one by
/// index or by part of its name, as for the tests of `vkxp-vk`.
pub fn test_device() -> DeviceChoice {
    match std::env::var("VKXP_TEST_DEVICE") {
        Ok(value) if !value.is_empty() => vkxp_host::device_from_text(&value),
        _ => DeviceChoice::Name(vkxp_host::DEFAULT_DEVICE.to_owned()),
    }
}

/// A demo small enough to composite quickly on the CPU, also with the reference.
pub const SMALL_DEMO: DemoOptions = DemoOptions {
    size: Size::new(160, 120),
    frame_rate: FrameRate::DEFAULT,
};

/// Starts the demo on its own thread and returns the host's end of its link.
pub fn spawn_demo(
    options: DemoOptions,
) -> (HostLink, thread::JoinHandle<Result<(), Disconnected>>) {
    let (guest, host) = headless_link();
    let demo = thread::spawn(move || demo::run(&guest, options));
    (host, demo)
}

/// Plays host for the demo by hand: feeds it `script` the way a headless run does, the entry
/// for frame `n` before the tick of frame `n`, for `frames` frames, then asks it to quit.
/// Returns every message the demo sent before the quit request.
pub fn drive_demo(options: DemoOptions, script: &InputScript, frames: u64) -> Vec<GuestToHost> {
    let (host, demo) = spawn_demo(options);
    let mut messages = Vec::new();
    let mut count = 0;
    let mut held = InputSnapshot::EMPTY;
    while count < frames {
        let message = host.recv().expect("the demo is running");
        let is_frame = matches!(message, GuestToHost::Frame(_));
        messages.push(message);
        if is_frame {
            if let Some(buttons) = script.at(count).filter(|buttons| *buttons != held) {
                held = buttons;
                host.send(HostToGuest::Input(buttons)).unwrap();
            }
            count += 1;
            if count < frames {
                host.send(HostToGuest::Tick).unwrap();
            }
        }
    }
    host.send(HostToGuest::Quit).unwrap();
    assert_eq!(host.recv(), Ok(GuestToHost::Shutdown));
    assert_eq!(host.recv(), Err(Disconnected));
    assert_eq!(demo.join().unwrap(), Ok(()));
    messages
}

/// A tiny frame of one colour, for guests written in the tests.
pub fn small_frame(frame_index: u64) -> GuestToHost {
    let size = Size::new(16, 12);
    GuestToHost::Frame(FramePacket {
        frame_index,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        bitmaps: Vec::new(),
        ops: vec![Op::Clear {
            clip: Rect::from_size(size),
            color: Color::new(frame_index as u8, 128, 64, 255),
        }],
    })
}
