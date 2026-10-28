//! The scenarios of "One wait per shown frame" in `specs/vulkan-compositor/spec.md`: `render`
//! leaves its compose in flight, and every public method called right after it gives the
//! result the reference gives, under validation.

use vkrgss_compose_ref as reference;
use vkrgss_compose_ref::fixtures::{self, bitmap, sprite};
use vkrgss_proto::{
    BitmapDelta, BitmapId, Color, FramePacket, FrameRate, FreezeRule, GuestToHost, Op, Rect, Size,
};

use super::harness::{TestCompositor, assert_clean, test_device};
use crate::{Filter, Frame, Mix, OverlayImage, ValidationLog, VkCompositor};

const SIZE: Size = Size::new(20, 18);
/// A window twice the size of the frames, which shows each pixel as a block of four.
const WINDOW: Size = Size::new(40, 36);
const ID: BitmapId = BitmapId::new(0, 0);

/// A frame that draws the bitmap `ID` over the screen; with `pixels`, the packet creates the
/// bitmap with them first.
fn drawn(pixels: Option<Vec<u8>>) -> FramePacket {
    let mut quad = sprite(ID, SIZE.width, SIZE.height, 0, 0);
    quad.clip = Rect::from_size(SIZE);
    FramePacket {
        frame_index: 0,
        logical_size: SIZE,
        frame_rate: FrameRate::DEFAULT,
        brightness: 255,
        tables: vec![],
        bitmaps: pixels.map_or_else(Vec::new, |rgba| {
            bitmap(ID, SIZE.width, SIZE.height, rgba).to_vec()
        }),
        ops: vec![Op::Quad(quad)],
    }
}

/// The first frame: it creates the bitmap with a gradient and draws it.
fn first() -> FramePacket {
    drawn(Some(fixtures::gradient(SIZE.width, SIZE.height)))
}

/// A frame of one colour that destroys the bitmap, whose image is then released once the
/// frame's compose has completed.
fn flat_destroying(color: [u8; 4]) -> FramePacket {
    FramePacket {
        bitmaps: vec![BitmapDelta::Destroy { id: ID }],
        ops: vec![Op::Clear {
            clip: Rect::from_size(SIZE),
            color: Color::from_rgba(color),
        }],
        ..drawn(None)
    }
}

/// What the reference renders for `packets`, the last frame of them.
fn reference(packets: &[&FramePacket]) -> Frame {
    let mut compositor = reference::Compositor::new();
    let mut last = None;
    for packet in packets {
        last = Some(compositor.render(packet).unwrap());
    }
    let frame = last.expect("a packet");
    Frame {
        size: frame.size,
        rgba: frame.rgba,
    }
}

/// The frame as a window shows it, `factor` times as large: fully opaque, each pixel a block.
fn shown(frame: &Frame, factor: u32) -> Frame {
    let size = Size::new(frame.size.width * factor, frame.size.height * factor);
    let mut rgba = Vec::with_capacity((size.width * size.height * 4) as usize);
    for y in 0..size.height {
        for x in 0..size.width {
            let at = (((y / factor) * frame.size.width + x / factor) * 4) as usize;
            rgba.extend_from_slice(&frame.rgba[at..at + 3]);
            rgba.push(255);
        }
    }
    Frame { size, rgba }
}

/// A frame that draws the bitmap the first frame made.
fn again() -> FramePacket {
    drawn(None)
}

/// A compositor with validation that has rendered the first frame and then `packet`, whose
/// compose is still in flight.
fn right_after(packet: &FramePacket) -> TestCompositor {
    let mut compositor = TestCompositor::new();
    compositor.render(&first()).unwrap();
    compositor.render(packet).unwrap();
    assert!(compositor.has_compose_in_flight(), "render waited");
    compositor
}

/// A compositor for a headless surface with validation, its window set to [`WINDOW`].
fn surface_compositor(log: &ValidationLog) -> VkCompositor {
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(log))
        .unwrap_or_else(|error| panic!("no compositor for a headless surface: {error}"));
    compositor.resize_surface(WINDOW);
    compositor
}

/// Asserts that nothing is in flight, as after every method but `render`.
fn assert_settled(compositor: &VkCompositor, after: &str) {
    assert!(
        !compositor.has_compose_in_flight(),
        "the compose is still in flight after {after}"
    );
}

/// A present that shows nothing still leaves nothing in flight.
#[test]
fn present_that_shows_nothing_waits_for_the_compose() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log);
    compositor.render(&first()).unwrap();
    compositor.resize_surface(Size::new(0, 0));
    assert_eq!(compositor.present(), Ok(false));
    assert_settled(&compositor, "a present that showed nothing");
    assert_eq!(
        compositor.read_frame().unwrap(),
        Some(reference(&[&first()]))
    );
    assert_clean(&log, "with a window without pixels");
}

// Scenario "Calls right after a render": each public method of the compositor.

#[test]
fn render_right_after_a_render_releases_the_destroyed_bitmap_once_done() {
    let second = flat_destroying([1, 2, 3, 255]);
    let mut compositor = right_after(&again());
    compositor.render(&second).unwrap();
    assert!(!compositor.has_bitmap(ID));
    assert_eq!(
        compositor.read_frame().unwrap(),
        Some(reference(&[&first(), &again(), &second]))
    );
}

#[test]
fn handle_right_after_a_render_applies_bitmaps_and_renders_frames() {
    let mut compositor = right_after(&again());
    let pixels = fixtures::solid(SIZE.width, SIZE.height, Color::new(9, 8, 7, 255));
    let update = BitmapDelta::Update {
        id: ID,
        rect: Rect::from_size(SIZE),
        rgba: pixels.clone(),
    };
    let message = GuestToHost::Bitmaps(vec![update.clone()]);
    assert_eq!(compositor.handle(&message), Ok(false));
    assert_settled(&compositor, "handling bitmaps");
    assert_eq!(compositor.read_bitmap(ID).unwrap(), pixels);

    let frame = drawn(None);
    assert_eq!(
        compositor.handle(&GuestToHost::Frame(frame.clone())),
        Ok(true)
    );
    let mut with_update = drawn(None);
    with_update.bitmaps = vec![update];
    assert_eq!(
        compositor.read_frame().unwrap(),
        Some(reference(&[&first(), &again(), &with_update, &frame]))
    );
}

#[test]
fn forget_resources_right_after_a_render_keeps_the_frame() {
    let mut compositor = right_after(&again());
    compositor.forget_resources().unwrap();
    assert_settled(&compositor, "forgetting the resources");
    assert!(!compositor.has_bitmap(ID));
    assert_eq!(
        compositor.read_frame().unwrap(),
        Some(reference(&[&first()]))
    );
    // The bitmap can be created anew.
    compositor.render(&first()).unwrap();
}

#[test]
fn read_frame_right_after_a_render_reads_the_frame() {
    let second = flat_destroying([40, 50, 60, 255]);
    let mut compositor = right_after(&second);
    assert_eq!(
        compositor.read_frame().unwrap(),
        Some(reference(&[&first(), &second]))
    );
    assert_settled(&compositor, "reading the frame");
}

#[test]
fn take_frame_timing_right_after_a_render_has_the_compose_time() {
    let mut compositor = TestCompositor::new();
    compositor.set_frame_timing(true);
    compositor.render(&first()).unwrap();
    let timing = compositor.take_frame_timing().expect("a timing");
    assert_settled(&compositor, "taking the timing");
    assert!(timing.submit.is_some(), "{timing:?}");
    assert_eq!(
        timing.compose_device.is_some(),
        compositor.has_timestamps(),
        "{timing:?}"
    );
    assert_eq!(
        compositor.read_frame().unwrap(),
        Some(reference(&[&first()]))
    );
}

#[test]
fn present_right_after_a_render_shows_the_frame() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log);
    compositor.render(&first()).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    let second = flat_destroying([7, 70, 170, 255]);
    compositor.render(&second).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    assert_settled(&compositor, "a present");
    assert_eq!(
        compositor.scale_frame(WINDOW).unwrap(),
        shown(&reference(&[&first(), &second]), 2)
    );
    assert_clean(&log, "while presenting right after renders");
}

#[test]
fn scale_frame_right_after_a_render_scales_the_frame() {
    let mut compositor = right_after(&again());
    assert_eq!(
        compositor.scale_frame(WINDOW).unwrap(),
        shown(&reference(&[&first()]), 2)
    );
    assert_settled(&compositor, "scaling the frame");
    // A target without pixels draws nothing and still leaves nothing in flight.
    compositor.render(&again()).unwrap();
    let empty = compositor.scale_frame(Size::new(0, 4)).unwrap();
    assert!(empty.rgba.is_empty());
    assert_settled(&compositor, "scaling into no pixels");
}

/// After a freeze right after `render`, the held picture is that frame.
fn assert_holds(compositor: &mut VkCompositor, frame: &Frame) {
    compositor.set_mix(Mix::HELD);
    compositor
        .render(&flat_destroying([255, 255, 255, 255]))
        .unwrap();
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), shown(frame, 1));
}

#[test]
fn freeze_right_after_a_render_holds_the_frame() {
    let mut compositor = right_after(&again());
    compositor.freeze().unwrap();
    assert_settled(&compositor, "a freeze");
    assert_holds(&mut compositor, &reference(&[&first()]));
}

#[test]
fn freeze_into_buffer_right_after_a_render_holds_the_frame() {
    let mut compositor = right_after(&again());
    compositor.freeze_into_buffer().unwrap();
    assert_settled(&compositor, "a freeze into the buffer");
    assert_holds(&mut compositor, &reference(&[&first()]));
}

#[test]
fn freeze_by_right_after_a_render_holds_the_frame_by_either_rule() {
    for rule in [FreezeRule::Frame, FreezeRule::Buffer] {
        let mut compositor = right_after(&again());
        compositor.freeze_by(rule).unwrap();
        assert_settled(&compositor, "a freeze");
        assert_holds(&mut compositor, &reference(&[&first()]));
    }
}

#[test]
fn set_overlay_right_after_a_render_leaves_the_frame_alone() {
    let mut compositor = right_after(&again());
    let picture = OverlayImage {
        size: Size::new(4, 4),
        pixels: fixtures::gradient(4, 4),
    };
    compositor.set_overlay(Some(&picture)).unwrap();
    assert_settled(&compositor, "setting the overlay");
    compositor.render(&again()).unwrap();
    compositor.set_overlay(None).unwrap();
    assert_settled(&compositor, "taking the overlay away");
    assert_eq!(
        compositor.read_frame().unwrap(),
        Some(reference(&[&first()]))
    );
}

#[test]
fn release_surface_and_attach_right_after_a_render_keep_the_frame() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log);
    compositor.render(&first()).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    compositor.render(&again()).unwrap();
    compositor.release_surface().unwrap();
    assert_settled(&compositor, "releasing the surface");
    // `attach_window` needs a window; the headless surface goes through the same call.
    compositor.render(&again()).unwrap();
    compositor.attach_headless_surface().unwrap();
    assert_settled(&compositor, "attaching a surface");
    assert_eq!(compositor.present(), Ok(true));
    assert_eq!(
        compositor.scale_frame(WINDOW).unwrap(),
        shown(&reference(&[&first()]), 2)
    );
    assert_clean(&log, "while the surface was released and attached");
}

/// A call of a method that only sets or reports state.
type StateCall = dyn Fn(&mut VkCompositor);

/// The methods that only set or report state use no device: called right after a render
/// they leave the compose in flight, and the frame is the reference's.
#[test]
fn state_methods_right_after_a_render_leave_the_frame_alone() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log);
    let calls: [(&str, &StateCall); 15] = [
        ("set_mix", &|c| c.set_mix(Mix::Newest)),
        ("mix", &|c| assert_eq!(c.mix(), Mix::Newest)),
        ("set_filter", &|c| c.set_filter(Filter::Auto)),
        ("filter", &|c| assert_eq!(c.filter(), Filter::Auto)),
        ("set_mask", &|c| c.set_mask(None)),
        ("has_bitmap", &|c| assert!(c.has_bitmap(ID))),
        ("has_surface", &|c| assert!(c.has_surface())),
        ("set_frame_rate", &|c| c.set_frame_rate(40)),
        ("restart_pacing", &|c| c.restart_pacing()),
        ("present_summary", &|c| {
            c.present_summary();
        }),
        ("paces_with_present_timing", &|c| {
            c.paces_with_present_timing();
        }),
        ("take_pacing_report", &|c| {
            assert_eq!(c.take_pacing_report(), None)
        }),
        ("resize_surface", &|c| c.resize_surface(WINDOW)),
        ("device", &|c| assert!(!c.device().name.is_empty())),
        ("set_frame_timing", &|c| c.set_frame_timing(false)),
    ];
    let mut packets = vec![first()];
    compositor.render(&first()).unwrap();
    for (name, call) in calls {
        let packet = again();
        compositor.render(&packet).unwrap();
        packets.push(packet);
        call(&mut compositor);
        assert!(compositor.has_compose_in_flight(), "{name} waited");
    }
    let packets: Vec<&FramePacket> = packets.iter().collect();
    assert_eq!(compositor.read_frame().unwrap(), Some(reference(&packets)));
    assert_clean(&log, "while the state was set right after renders");
}

/// Dropping the compositor waits for the compose in flight, which still uses the image of a
/// bitmap its packet destroyed.
#[test]
fn drop_right_after_a_render_waits_for_it() {
    let compositor = right_after(&flat_destroying([5, 6, 7, 255]));
    // The test compositor asserts on drop that the validation layer reported nothing.
    drop(compositor);
}
