//! The overlay drawn over the frame in the window: presented on a headless surface under
//! validation, and never part of a frame that is read back.

use vkrgss_compose_ref::fixtures::{self, bitmap, sprite};
use vkrgss_proto::{BitmapId, FramePacket, FrameRate, Op, Rect, Size};

use super::harness::{assert_clean, test_device};
use crate::{OverlayImage, ValidationLog, VkCompositor};

const SIZE: Size = Size::new(20, 18);
const WINDOW: Size = Size::new(64, 48);

/// A frame whose every pixel differs from its neighbours.
fn gradient(size: Size) -> FramePacket {
    let id = BitmapId::new(0, 0);
    let mut quad = sprite(id, size.width, size.height, 0, 0);
    quad.clip = Rect::from_size(size);
    FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        brightness: 255,
        tables: vec![],
        bitmaps: bitmap(
            id,
            size.width,
            size.height,
            fixtures::gradient(size.width, size.height),
        )
        .to_vec(),
        ops: vec![Op::Quad(quad)],
    }
}

/// A picture of `size` whose left half is half-transparent white and whose right half is
/// clear, premultiplied.
fn picture(size: Size) -> OverlayImage {
    let mut pixels = Vec::new();
    for _ in 0..size.height {
        for x in 0..size.width {
            let texel: [u8; 4] = if x < size.width / 2 {
                [128, 128, 128, 128]
            } else {
                [0; 4]
            };
            pixels.extend_from_slice(&texel);
        }
    }
    OverlayImage { size, pixels }
}

fn surface_compositor(log: &ValidationLog, srgb: bool) -> VkCompositor {
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(log))
        .unwrap_or_else(|error| {
            panic!(
                "presentation cannot be tested here: no compositor for a headless surface on \
                 the test device: {error}"
            )
        });
    if srgb {
        compositor.force_srgb_surface_format();
    }
    compositor.resize_surface(WINDOW);
    compositor
}

fn presents_the_overlay_without_changing_the_frames(srgb: bool) {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log, srgb);
    // Before the first frame there is nothing beneath it.
    compositor.set_overlay(Some(&picture(WINDOW))).unwrap();
    assert_eq!(compositor.present(), Ok(true));

    compositor.render(&gradient(SIZE)).unwrap();
    compositor.set_overlay(None).unwrap();
    let frame = compositor.read_frame().unwrap();
    let scaled = compositor.scale_frame(WINDOW).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    compositor.set_overlay(Some(&picture(WINDOW))).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    assert_eq!(compositor.read_frame().unwrap(), frame);
    assert_eq!(compositor.scale_frame(WINDOW).unwrap(), scaled);

    // A picture of another size is scaled to the window, and one of the same size replaces
    // the one there is.
    compositor
        .set_overlay(Some(&picture(Size::new(32, 24))))
        .unwrap();
    assert_eq!(compositor.present(), Ok(true));
    compositor
        .set_overlay(Some(&picture(Size::new(32, 24))))
        .unwrap();
    compositor.resize_surface(Size::new(100, 75));
    assert_eq!(compositor.present(), Ok(true));
    compositor.set_overlay(None).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    assert_eq!(compositor.read_frame().unwrap(), frame);
    compositor.set_overlay(Some(&picture(WINDOW))).unwrap();
    assert_clean(&log, "while presenting an overlay");

    drop(compositor);
    assert_clean(&log, "while the compositor with an overlay was dropped");
}

#[test]
fn the_overlay_is_presented_and_frames_read_back_do_not_hold_it() {
    presents_the_overlay_without_changing_the_frames(false);
}

#[test]
fn the_overlay_is_presented_on_a_forced_srgb_swapchain() {
    presents_the_overlay_without_changing_the_frames(true);
}

#[test]
#[should_panic(expected = "the pixels of an overlay picture")]
fn a_picture_whose_pixels_do_not_fit_its_size_is_refused() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log, false);
    let mut wrong = picture(WINDOW);
    wrong.pixels.pop();
    let _ = compositor.set_overlay(Some(&wrong));
}
