//! The scenarios of "Frame timings on request" in `specs/vulkan-compositor/spec.md`. That
//! measuring changes no frame is checked with the fixture streams, in `fixtures`.

use vkxp_compose_ref::fixtures::{self, bitmap, sprite};
use vkxp_proto::{BitmapId, FramePacket, FrameRate, Op, Rect, Size};

use super::harness::{TestCompositor, assert_clean, test_device};
use crate::{ValidationLog, VkCompositor, VkError};

const SIZE: Size = Size::new(20, 18);
const ID: BitmapId = BitmapId::new(0, 0);

/// A frame that creates a bitmap, uploads its pixels and draws it over the screen.
fn frame_with_an_upload() -> FramePacket {
    let mut quad = sprite(ID, SIZE.width, SIZE.height, 0, 0);
    quad.clip = Rect::from_size(SIZE);
    FramePacket {
        frame_index: 0,
        logical_size: SIZE,
        frame_rate: FrameRate::DEFAULT,
        brightness: 255,
        tables: vec![],
        bitmaps: bitmap(
            ID,
            SIZE.width,
            SIZE.height,
            fixtures::gradient(SIZE.width, SIZE.height),
        )
        .to_vec(),
        ops: vec![Op::Quad(quad)],
    }
}

#[test]
fn frame_with_an_upload_has_every_processor_stage_and_the_compose_time() {
    let mut compositor = TestCompositor::new();
    compositor.set_frame_timing(true);
    compositor.render(&frame_with_an_upload()).unwrap();
    let timing = compositor.take_frame_timing().expect("a timing");
    assert!(timing.encode.is_some(), "{timing:?}");
    assert!(timing.bin.is_some(), "{timing:?}");
    assert!(timing.upload.is_some(), "{timing:?}");
    assert!(timing.submit.is_some(), "{timing:?}");
    assert_eq!(
        timing.compose_device.is_some(),
        compositor.has_timestamps(),
        "{timing:?}"
    );
    // Nothing was presented, and the timing is handed out once.
    assert_eq!(timing.present, None);
    assert_eq!(timing.present_device, None);
    assert_eq!(compositor.take_frame_timing(), None);
}

#[test]
fn nothing_is_measured_without_the_request() {
    let mut compositor = TestCompositor::new();
    compositor.render(&frame_with_an_upload()).unwrap();
    assert_eq!(compositor.take_frame_timing(), None);
}

#[test]
fn turning_the_timing_off_drops_what_was_measured() {
    let mut compositor = TestCompositor::new();
    compositor.set_frame_timing(true);
    compositor.render(&frame_with_an_upload()).unwrap();
    compositor.set_frame_timing(false);
    assert_eq!(compositor.take_frame_timing(), None);
    compositor.render(&frame_with_an_upload()).unwrap_err();
    assert_eq!(compositor.take_frame_timing(), None);
}

#[test]
fn rejected_frame_has_no_timing() {
    let mut compositor = TestCompositor::new();
    compositor.set_frame_timing(true);
    compositor.render(&frame_with_an_upload()).unwrap();
    // The bitmap exists already, so the same packet is invalid now.
    compositor.render(&frame_with_an_upload()).unwrap_err();
    assert_eq!(compositor.take_frame_timing(), None);
}

/// A packet refused for its size, before anything else is looked at, leaves no timing
/// either: not its own and not that of the frame before it.
#[test]
fn frame_of_an_invalid_size_drops_the_timing_of_the_frame_before() {
    let mut compositor = TestCompositor::new();
    compositor.set_frame_timing(true);
    let mut packet = FramePacket {
        bitmaps: Vec::new(),
        ops: Vec::new(),
        logical_size: Size::new(1, 1),
        ..frame_with_an_upload()
    };
    compositor.render(&packet).unwrap();
    packet.logical_size = Size::new(0, 1);
    let error = compositor.render(&packet).unwrap_err();
    assert!(
        matches!(error, VkError::InvalidLogicalSize { .. }),
        "{error}"
    );
    assert_eq!(compositor.take_frame_timing(), None);
}

#[test]
fn present_adds_its_stage_and_the_time_of_its_pass() {
    let log = ValidationLog::new();
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(&log)).unwrap();
    compositor.set_frame_timing(true);
    compositor.resize_surface(Size::new(40, 36));
    compositor.render(&frame_with_an_upload()).unwrap();
    assert!(compositor.present().unwrap());
    let timing = compositor.take_frame_timing().expect("a timing");
    assert!(timing.encode.is_some(), "{timing:?}");
    assert!(timing.present.is_some(), "{timing:?}");
    assert_eq!(
        timing.present_device.is_some(),
        compositor.has_timestamps(),
        "{timing:?}"
    );
    if let (Some(compose), Some(present)) = (timing.compose_device, timing.present_device) {
        // Each is the difference of two timestamps of one submission that was waited for.
        assert!(compose <= timing.submit.unwrap(), "{timing:?}");
        assert!(present <= timing.present.unwrap(), "{timing:?}");
    }
    assert_clean(&log, "with frame timing on a surface");
    drop(compositor);
    assert_clean(&log, "while the compositor was dropped");
}
