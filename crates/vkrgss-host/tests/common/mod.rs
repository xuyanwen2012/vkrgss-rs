//! Helpers shared by the integration tests.
#![allow(dead_code)]

use std::thread;

use vkrgss_host::demo::{self, DemoOptions};
use vkrgss_host::{HostLink, InputScript, headless_link};
use vkrgss_proto::link::Disconnected;
use vkrgss_proto::{
    Color, FramePacket, FrameRate, GuestToHost, HostToGuest, InputSnapshot, Op, Rect, Size,
};
use vkrgss_vk::DeviceChoice;

/// Environment variables that restrict which drivers the Vulkan loader loads. Naming a
/// device does not do that: the loader loads every installed driver to enumerate the devices,
/// including the driver of a GPU that must be left alone.
const ISOLATION_VARIABLES: [&str; 3] = [
    "VK_LOADER_DRIVERS_SELECT",
    "VK_DRIVER_FILES",
    "VK_ICD_FILENAMES",
];

/// Why the tests may not touch Vulkan in an environment where `variable` reads the loader's
/// variables, or `None` when one of them restricts the loader.
pub(crate) fn isolation_error(variable: impl Fn(&str) -> Option<String>) -> Option<String> {
    let restricted = ISOLATION_VARIABLES
        .iter()
        .any(|name| variable(name).is_some_and(|value| !value.trim().is_empty()));
    (!restricted).then(|| {
        "the Vulkan loader is not restricted to chosen drivers, so these tests would load the \
         driver of every GPU in the machine just to enumerate the devices. Set \
         VK_LOADER_DRIVERS_SELECT before running them: '*lvp*' for the CPU device (lavapipe), \
         which is what the tests use by default, or the pattern of the driver that \
         VKRGSS_TEST_DEVICE names, for example '*radeon*' with VKRGSS_TEST_DEVICE=RADV."
            .to_owned()
    })
}

/// Fails the test before anything is asked of the Vulkan loader unless the loader has been
/// restricted. Every test that reaches Vulkan goes through this, most of them by way of
/// [`test_device`].
pub(crate) fn require_isolation() {
    if let Some(error) = isolation_error(|name| std::env::var(name).ok()) {
        panic!("{error}");
    }
}

/// The device the tests composite on: that of a headless run (llvmpipe on Linux), unless
/// `VKRGSS_TEST_DEVICE` names another one by index or by part of its name, as for the tests of
/// `vkrgss-vk`. Like those, the tests refuse to start while the loader is not restricted to
/// chosen drivers.
pub(crate) fn test_device() -> DeviceChoice {
    require_isolation();
    match std::env::var("VKRGSS_TEST_DEVICE") {
        Ok(value) if !value.is_empty() => vkrgss_host::device_from_text(&value),
        _ => vkrgss_host::default_device(),
    }
}

/// A demo small enough to composite quickly on the CPU, also with the reference.
pub(crate) const SMALL_DEMO: DemoOptions = DemoOptions {
    size: Size::new(160, 120),
    frame_rate: FrameRate::DEFAULT,
};

/// Starts the demo on its own thread and returns the host's end of its link.
pub(crate) fn spawn_demo(
    options: DemoOptions,
) -> (HostLink, thread::JoinHandle<Result<(), Disconnected>>) {
    let (guest, host) = headless_link();
    let demo = thread::spawn(move || demo::run(&guest, options));
    (host, demo)
}

/// Plays host for the demo by hand: feeds it `script` the way a headless run does, the entry
/// for frame `n` before the tick of frame `n`, for `frames` frames, then asks it to quit.
/// Returns every message the demo sent before the quit request.
pub(crate) fn drive_demo(
    options: DemoOptions,
    script: &InputScript,
    frames: u64,
) -> Vec<GuestToHost> {
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
pub(crate) fn small_frame(frame_index: u64) -> GuestToHost {
    let size = Size::new(16, 12);
    GuestToHost::Frame(FramePacket {
        frame_index,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        brightness: 255,
        tables: vec![],
        bitmaps: Vec::new(),
        ops: vec![Op::Clear {
            clip: Rect::from_size(size),
            color: Color::new(frame_index as u8, 128, 64, 255),
        }],
    })
}
