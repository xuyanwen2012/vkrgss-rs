//! The scenarios of `specs/vulkan-presentation/spec.md`: scaling and mixing through the
//! off-screen target, and the swapchain on a headless surface. No test opens a window.

use vkxp_compose_ref::fixtures::{self, bitmap, sprite};
use vkxp_proto::{BitmapId, Color, FramePacket, FrameRate, Op, Rect, Size};

use super::harness::{TestCompositor, assert_clean, require_isolation, test_device};
use crate::{DeviceChoice, DeviceError, DeviceKind, Frame, ValidationLog, VkCompositor, VkError};

const SIZE: Size = Size::new(20, 18);
const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const ID: BitmapId = BitmapId::new(0, 0);

fn packet(size: Size, ops: Vec<Op>) -> FramePacket {
    FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        bitmaps: Vec::new(),
        ops,
    }
}

/// A frame of one colour.
fn flat(size: Size, color: [u8; 4]) -> FramePacket {
    packet(
        size,
        vec![Op::Clear {
            clip: Rect::from_size(size),
            color: Color::from_rgba(color),
        }],
    )
}

/// A frame whose every pixel differs from its neighbours: a gradient bitmap over the screen.
fn gradient(size: Size) -> FramePacket {
    let mut packet = packet(
        size,
        vec![Op::Quad(sprite(ID, size.width, size.height, 0, 0))],
    );
    packet.bitmaps = bitmap(
        ID,
        size.width,
        size.height,
        fixtures::gradient(size.width, size.height),
    )
    .to_vec();
    // The quad is clipped to the fixtures' own screen; open it to this frame.
    if let Op::Quad(quad) = &mut packet.ops[0] {
        quad.clip = Rect::from_size(size);
    }
    packet
}

/// A frame with `left` on its left half and `right` on its right half.
fn halves(size: Size, left: [u8; 4], right: [u8; 4]) -> FramePacket {
    let half = size.width / 2;
    packet(
        size,
        vec![
            Op::Clear {
                clip: Rect::new(0, 0, half, size.height),
                color: Color::from_rgba(left),
            },
            Op::Clear {
                clip: Rect::new(half as i32, 0, size.width - half, size.height),
                color: Color::from_rgba(right),
            },
        ],
    )
}

/// A frame whose columns alternate between black and white, starting with black.
fn columns(size: Size) -> FramePacket {
    let ops = (0..size.width)
        .map(|x| Op::Clear {
            clip: Rect::new(x as i32, 0, 1, size.height),
            color: Color::from_rgba(if x % 2 == 0 { BLACK } else { WHITE }),
        })
        .collect();
    packet(size, ops)
}

fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    assert!(x < frame.size.width && y < frame.size.height);
    let offset = ((y * frame.size.width + x) * 4) as usize;
    frame.rgba[offset..offset + 4].try_into().unwrap()
}

/// The frame as a window shows it: the same colours, fully opaque.
fn opaque(mut frame: Frame) -> Frame {
    for pixel in frame.rgba.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    frame
}

/// `frame` with every pixel repeated as a `factor` by `factor` block, worked out on the CPU.
fn blocks(frame: &Frame, factor: u32) -> Frame {
    let size = Size::new(frame.size.width * factor, frame.size.height * factor);
    let mut rgba = Vec::with_capacity((size.width * size.height * 4) as usize);
    for y in 0..size.height {
        for x in 0..size.width {
            rgba.extend_from_slice(&pixel(frame, x / factor, y / factor));
        }
    }
    Frame { size, rgba }
}

fn assert_all(frame: &Frame, area: Rect, expected: [u8; 4], what: &str) {
    for y in area.y as u32..area.y as u32 + area.height {
        for x in area.x as u32..area.x as u32 + area.width {
            assert_eq!(pixel(frame, x, y), expected, "{what} at ({x}, {y})");
        }
    }
}

/// Renders one packet and returns the compositor with the frame it holds.
fn rendered(packet: &FramePacket) -> (TestCompositor, Frame) {
    let mut compositor = TestCompositor::new();
    compositor.render(packet).unwrap();
    let frame = compositor.read_frame().unwrap().expect("a frame");
    (compositor, opaque(frame))
}

// Presentation is optional.

#[test]
fn compositor_without_a_surface_renders_as_before_and_cannot_present() {
    let (mut compositor, frame) = rendered(&gradient(SIZE));
    assert!(!compositor.has_surface());
    assert_eq!(compositor.present(), Err(VkError::NoSurface));
    compositor.resize_surface(Size::new(64, 48));
    assert_eq!(compositor.present(), Err(VkError::NoSurface));
    // Asking changed nothing about the frame.
    assert_eq!(opaque(compositor.read_frame().unwrap().unwrap()), frame);
}

#[test]
fn compositor_for_a_headless_surface_is_created_on_the_test_device() {
    let log = ValidationLog::new();
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(&log))
        .unwrap_or_else(|error| {
            panic!("cannot create a compositor for a headless surface: {error}")
        });
    assert!(compositor.has_surface());
    assert!(compositor.device().is_suitable());
    if std::env::var_os("VKXP_TEST_DEVICE").is_none() {
        assert_eq!(compositor.device().kind, DeviceKind::Cpu);
    }

    // It composites exactly like the headless one.
    let packet = gradient(SIZE);
    compositor.render(&packet).unwrap();
    let with_surface = compositor.read_frame().unwrap().unwrap();
    let (_headless, expected) = rendered(&packet);
    assert_eq!(opaque(with_surface), expected);

    assert_clean(&log, "with a headless surface");
    drop(compositor);
    assert_clean(&log, "while the compositor with a surface was dropped");
}

#[test]
fn explicit_device_that_does_not_exist_is_not_replaced_for_a_surface() {
    require_isolation();
    let choice = DeviceChoice::Name("no such device".to_owned());
    let error = VkCompositor::with_headless_surface(choice, None)
        .err()
        .expect("no device has that name");
    assert!(
        matches!(error, VkError::Device(DeviceError::NotFound { .. })),
        "{error}"
    );
}

// Scaling into the window.

#[test]
fn frame_that_fits_exactly_fills_the_window() {
    let size = Size::new(640, 480);
    let (mut compositor, frame) = rendered(&gradient(size));
    let scaled = compositor.scale_frame(Size::new(1280, 960)).unwrap();
    assert_eq!(scaled, blocks(&frame, 2));
}

#[test]
fn wide_window_shows_the_frame_centred_between_black_bars() {
    let color = [200, 100, 50, 255];
    let (mut compositor, _) = rendered(&flat(Size::new(640, 480), color));
    let scaled = compositor.scale_frame(Size::new(1920, 1080)).unwrap();
    assert_eq!(scaled.size, Size::new(1920, 1080));
    assert_all(&scaled, Rect::new(0, 0, 240, 1080), BLACK, "left bar");
    assert_all(&scaled, Rect::new(240, 0, 1440, 1080), color, "image");
    assert_all(&scaled, Rect::new(1680, 0, 240, 1080), BLACK, "right bar");
}

#[test]
fn tall_window_shows_the_frame_centred_between_black_bars() {
    let (mut compositor, frame) = rendered(&gradient(SIZE));
    let scaled = compositor.scale_frame(Size::new(40, 50)).unwrap();
    assert_all(&scaled, Rect::new(0, 0, 40, 7), BLACK, "top bar");
    assert_all(&scaled, Rect::new(0, 43, 40, 7), BLACK, "bottom bar");
    let expected = blocks(&frame, 2);
    for y in 0..36 {
        for x in 0..40 {
            assert_eq!(
                pixel(&scaled, x, y + 7),
                pixel(&expected, x, y),
                "({x}, {y})"
            );
        }
    }
}

#[test]
fn target_without_pixels_gives_an_empty_image_and_no_error() {
    let (mut compositor, _) = rendered(&gradient(SIZE));
    for size in [Size::new(0, 48), Size::new(64, 0), Size::new(0, 0)] {
        let scaled = compositor.scale_frame(size).unwrap();
        assert_eq!(scaled, Frame { size, rgba: vec![] });
    }
    // Too narrow for even one row of the frame: all black.
    let sliver = compositor.scale_frame(Size::new(1, 40)).unwrap();
    assert_all(&sliver, Rect::new(0, 0, 1, 40), BLACK, "sliver");
}

#[test]
fn target_is_black_before_the_first_frame() {
    let mut compositor = TestCompositor::new();
    let scaled = compositor.scale_frame(Size::new(33, 17)).unwrap();
    assert_all(&scaled, Rect::new(0, 0, 33, 17), BLACK, "empty");
}

#[test]
fn target_beyond_the_device_limit_is_refused() {
    let (mut compositor, _) = rendered(&gradient(SIZE));
    let limit = compositor.max_image_dimension();
    let error = compositor.scale_frame(Size::new(limit + 1, 4)).unwrap_err();
    assert!(matches!(error, VkError::Limit { .. }), "{error}");
    // The compositor still works.
    assert_eq!(compositor.scale_frame(SIZE).unwrap().size, SIZE);
}

// Sampling.

#[test]
fn whole_number_factors_show_each_pixel_as_a_block_of_its_own_colour() {
    let (mut compositor, frame) = rendered(&gradient(SIZE));
    for factor in [1, 2, 3, 5] {
        let size = Size::new(SIZE.width * factor, SIZE.height * factor);
        let scaled = compositor.scale_frame(size).unwrap();
        assert_eq!(scaled, blocks(&frame, factor), "factor {factor}");
    }
}

#[test]
fn every_channel_value_passes_through_unchanged() {
    // 256 columns: column x is (x, 255 - x, x xor 0x55).
    let size = Size::new(256, 2);
    let ops = (0..256u32)
        .map(|x| Op::Clear {
            clip: Rect::new(x as i32, 0, 1, 2),
            color: Color::new(x as u8, 255 - x as u8, x as u8 ^ 0x55, 255),
        })
        .collect();
    let (mut compositor, frame) = rendered(&packet(size, ops));
    for x in 0..256 {
        assert_eq!(
            pixel(&frame, x, 0),
            [x as u8, 255 - x as u8, x as u8 ^ 0x55, 255]
        );
    }
    assert_eq!(compositor.scale_frame(size).unwrap(), frame);
    assert_eq!(
        compositor.scale_frame(Size::new(512, 4)).unwrap(),
        blocks(&frame, 2)
    );
}

#[test]
fn fractional_factor_keeps_flat_interiors_exact() {
    let (left, right) = ([210, 40, 30, 255], [20, 60, 220, 255]);
    let (mut compositor, _) = rendered(&halves(SIZE, left, right));
    // 1.5 times 20 by 18. The halves meet at column 15.
    let scaled = compositor.scale_frame(Size::new(30, 27)).unwrap();
    assert_all(&scaled, Rect::new(0, 0, 13, 27), left, "left half");
    assert_all(&scaled, Rect::new(17, 0, 13, 27), right, "right half");
}

#[test]
fn fractional_factor_interpolates_only_across_pixel_borders() {
    let (mut compositor, _) = rendered(&columns(SIZE));
    let scaled = compositor.scale_frame(Size::new(30, 27)).unwrap();
    // Three target pixels per two columns. The first lies inside the black column, the third
    // inside the white one, and the middle one straddles their border.
    for y in 0..27 {
        for pair in 0..10 {
            let x = pair * 3;
            assert_eq!(pixel(&scaled, x, y), BLACK, "({x}, {y})");
            assert_eq!(pixel(&scaled, x + 2, y), WHITE, "({}, {y})", x + 2);
            let [r, g, b, a] = pixel(&scaled, x + 1, y);
            assert!((127..=128).contains(&r), "({}, {y}): {r}", x + 1);
            assert_eq!((g, b, a), (r, r, 255));
        }
    }
}

// Transition mix and freeze.

/// A compositor that holds a black frame and has a white one as its newest.
fn black_then_white() -> TestCompositor {
    let mut compositor = TestCompositor::new();
    compositor.render(&flat(SIZE, BLACK)).unwrap();
    compositor.freeze().unwrap();
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    compositor
}

#[test]
fn mix_weight_starts_at_one_and_is_clamped() {
    let mut compositor = TestCompositor::new();
    assert_eq!(compositor.mix(), 1.0);
    for (given, kept) in [(0.25, 0.25), (-3.0, 0.0), (7.0, 1.0), (f32::NAN, 1.0)] {
        compositor.set_mix(given);
        assert_eq!(compositor.mix(), kept, "{given}");
    }
}

#[test]
fn mix_end_points_show_the_held_frame_and_the_newest_frame() {
    let mut compositor = black_then_white();
    let whole = Rect::from_size(SIZE);
    compositor.set_mix(0.0);
    assert_all(&compositor.scale_frame(SIZE).unwrap(), whole, BLACK, "held");
    compositor.set_mix(1.0);
    assert_all(
        &compositor.scale_frame(SIZE).unwrap(),
        whole,
        WHITE,
        "newest",
    );
}

#[test]
fn mix_of_one_half_is_mid_grey_between_black_and_white() {
    let mut compositor = black_then_white();
    compositor.set_mix(0.5);
    for size in [SIZE, Size::new(40, 36), Size::new(30, 27)] {
        let scaled = compositor.scale_frame(size).unwrap();
        for y in 0..size.height {
            for x in 0..size.width {
                let [r, g, b, a] = pixel(&scaled, x, y);
                assert!((127..=128).contains(&r), "{size:?} ({x}, {y}): {r}");
                assert_eq!((g, b, a), (r, r, 255));
            }
        }
    }
}

#[test]
fn mix_is_applied_per_channel() {
    let mut compositor = TestCompositor::new();
    compositor.render(&flat(SIZE, [200, 0, 100, 255])).unwrap();
    compositor.freeze().unwrap();
    compositor.render(&flat(SIZE, [0, 100, 100, 255])).unwrap();
    compositor.set_mix(0.25);
    let scaled = compositor.scale_frame(SIZE).unwrap();
    assert_all(&scaled, Rect::from_size(SIZE), [150, 25, 100, 255], "mix");
}

#[test]
fn held_frame_stays_shown_while_later_frames_are_rendered() {
    let (mut compositor, first) = rendered(&gradient(SIZE));
    compositor.freeze().unwrap();
    compositor.set_mix(0.0);
    for color in [WHITE, [9, 8, 7, 255], [0, 255, 0, 255]] {
        compositor.render(&flat(SIZE, color)).unwrap();
        assert_eq!(compositor.scale_frame(SIZE).unwrap(), first);
        let doubled = compositor.scale_frame(Size::new(40, 36)).unwrap();
        assert_eq!(doubled, blocks(&first, 2));
    }
    // The newest frame was rendered all along.
    let newest = opaque(compositor.read_frame().unwrap().unwrap());
    assert_all(&newest, Rect::from_size(SIZE), [0, 255, 0, 255], "newest");
    compositor.set_mix(1.0);
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), newest);
}

#[test]
fn freezing_again_replaces_the_held_frame() {
    let mut compositor = black_then_white();
    compositor.freeze().unwrap();
    compositor.render(&flat(SIZE, [10, 20, 30, 255])).unwrap();
    compositor.set_mix(0.0);
    let scaled = compositor.scale_frame(SIZE).unwrap();
    assert_all(&scaled, Rect::from_size(SIZE), WHITE, "second held frame");
}

#[test]
fn freeze_during_a_mix_holds_the_mixed_image() {
    let whole = Rect::from_size(SIZE);
    let grey = [64, 64, 64, 255];
    let mut compositor = black_then_white();
    compositor.set_mix(0.25);
    let on_screen = compositor.scale_frame(SIZE).unwrap();
    assert_all(
        &on_screen,
        whole,
        grey,
        "a quarter of the way from black to white",
    );

    // Freeze in the middle of the fade, as a session does, and render on.
    compositor.freeze().unwrap();
    compositor.set_mix(0.0);
    for color in [[0, 255, 0, 255], [255, 0, 255, 255]] {
        compositor.render(&flat(SIZE, color)).unwrap();
        assert_eq!(compositor.scale_frame(SIZE).unwrap(), on_screen);
        let doubled = compositor.scale_frame(Size::new(40, 36)).unwrap();
        assert_eq!(doubled, blocks(&on_screen, 2));
    }

    // The next fade starts from the grey, not from the black or the white behind it.
    compositor.set_mix(0.5);
    let half = compositor.scale_frame(SIZE).unwrap();
    for y in 0..SIZE.height {
        for x in 0..SIZE.width {
            let [r, g, b, a] = pixel(&half, x, y);
            assert!((159..=160).contains(&r), "({x}, {y}): {r}");
            assert_eq!((g, b, a), (32, r, 255), "({x}, {y})");
        }
    }
}

#[test]
fn freeze_during_a_mix_holds_each_pixel_of_a_detailed_image() {
    let (mut compositor, first) = rendered(&gradient(SIZE));
    compositor.freeze().unwrap();
    compositor
        .render(&halves(SIZE, [250, 10, 90, 255], [5, 200, 40, 255]))
        .unwrap();
    compositor.set_mix(0.75);
    let on_screen = compositor.scale_frame(SIZE).unwrap();
    assert_ne!(on_screen, first);

    compositor.freeze().unwrap();
    compositor.set_mix(0.0);
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), on_screen);
}

#[test]
fn freeze_at_weight_zero_keeps_the_held_frame() {
    let mut compositor = black_then_white();
    compositor.set_mix(0.0);
    compositor.freeze().unwrap();
    compositor.render(&flat(SIZE, [1, 2, 3, 255])).unwrap();
    let scaled = compositor.scale_frame(SIZE).unwrap();
    assert_all(
        &scaled,
        Rect::from_size(SIZE),
        BLACK,
        "still the first held frame",
    );
}

#[test]
fn freeze_before_the_first_frame_holds_black() {
    let mut compositor = TestCompositor::new();
    compositor.freeze().unwrap();
    compositor.set_mix(0.0);
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    let scaled = compositor.scale_frame(SIZE).unwrap();
    assert_all(&scaled, Rect::from_size(SIZE), BLACK, "held");
}

/// Asserts that every channel of every pixel in `area` is within one step of `expected`.
fn assert_near(frame: &Frame, area: Rect, expected: [u8; 4], what: &str) {
    for y in area.y as u32..area.y as u32 + area.height {
        for x in area.x as u32..area.x as u32 + area.width {
            let found = pixel(frame, x, y);
            let near = found
                .iter()
                .zip(expected)
                .all(|(found, expected)| found.abs_diff(expected) <= 1);
            assert!(
                near,
                "{what} at ({x}, {y}): {found:?}, expected {expected:?}"
            );
        }
    }
}

#[test]
fn held_frame_stays_shown_when_later_frames_have_another_size() {
    let mut compositor = TestCompositor::new();
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    compositor.freeze().unwrap();
    compositor.set_mix(0.0);
    let doubled = Size::new(40, 36);
    let before = compositor.scale_frame(doubled).unwrap();
    assert_all(&before, Rect::from_size(doubled), WHITE, "held, before");

    // The logical size doubles, then changes shape, then comes back.
    for size in [doubled, Size::new(32, 24), Size::new(7, 50), SIZE] {
        compositor.render(&flat(size, [40, 80, 120, 255])).unwrap();
        assert_eq!(compositor.scale_frame(doubled).unwrap(), before, "{size:?}");
    }
}

#[test]
fn held_frame_keeps_every_pixel_when_later_frames_have_another_size() {
    let (mut compositor, first) = rendered(&gradient(SIZE));
    compositor.freeze().unwrap();
    compositor.set_mix(0.0);
    compositor
        .render(&halves(Size::new(64, 48), WHITE, BLACK))
        .unwrap();
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), first);
    let tripled = compositor.scale_frame(Size::new(60, 54)).unwrap();
    assert_eq!(tripled, blocks(&first, 3));
    // In a window of the new frame's shape the held frame keeps its own: 53 by 48, centred.
    let scaled = compositor.scale_frame(Size::new(64, 48)).unwrap();
    assert_all(&scaled, Rect::new(0, 0, 5, 48), BLACK, "left bar");
    assert_all(&scaled, Rect::new(58, 0, 6, 48), BLACK, "right bar");
    assert_ne!(pixel(&scaled, 5, 0), BLACK);
    assert_ne!(pixel(&scaled, 57, 47), BLACK);
}

#[test]
fn transition_after_a_size_change_fades_from_the_held_frame_to_the_newest() {
    let mut compositor = TestCompositor::new();
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    compositor.freeze().unwrap();
    let newest = [40, 80, 120, 255];
    compositor.render(&flat(Size::new(32, 24), newest)).unwrap();

    // In an 80 by 72 window the held 20 by 18 frame fills everything, and the newest 32 by 24
    // frame is 80 by 60 with bars of 6 rows above and below.
    let window = Size::new(80, 72);
    let (top, middle, bottom) = (
        Rect::new(0, 0, 80, 6),
        Rect::new(0, 6, 80, 60),
        Rect::new(0, 66, 80, 6),
    );

    compositor.set_mix(0.0);
    let start = compositor.scale_frame(window).unwrap();
    assert_all(&start, Rect::from_size(window), WHITE, "held frame alone");

    // A quarter of the way: white towards the newest frame where that lies, and towards the
    // black of its bars elsewhere.
    compositor.set_mix(0.25);
    let quarter = compositor.scale_frame(window).unwrap();
    assert_near(&quarter, middle, [201, 211, 221, 255], "quarter, frame");
    assert_near(&quarter, top, [191, 191, 191, 255], "quarter, top bar");
    assert_near(
        &quarter,
        bottom,
        [191, 191, 191, 255],
        "quarter, bottom bar",
    );

    compositor.set_mix(0.5);
    let half = compositor.scale_frame(window).unwrap();
    assert_near(&half, middle, [148, 168, 188, 255], "half, frame");
    assert_near(&half, top, [128, 128, 128, 255], "half, top bar");

    compositor.set_mix(1.0);
    let end = compositor.scale_frame(window).unwrap();
    assert_all(&end, middle, newest, "newest frame alone");
    assert_all(&end, top, BLACK, "top bar");
    assert_all(&end, bottom, BLACK, "bottom bar");
}

#[test]
fn freeze_during_a_transition_across_sizes_holds_what_is_shown() {
    let mut compositor = TestCompositor::new();
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    compositor.freeze().unwrap();
    let larger = Size::new(40, 36);
    compositor
        .render(&flat(larger, [40, 80, 120, 255]))
        .unwrap();
    compositor.set_mix(0.5);
    let on_screen = compositor.scale_frame(larger).unwrap();
    assert_near(
        &on_screen,
        Rect::from_size(larger),
        [148, 168, 188, 255],
        "half way",
    );

    compositor.freeze().unwrap();
    compositor.set_mix(0.0);
    for size in [larger, SIZE, Size::new(64, 48)] {
        compositor.render(&flat(size, [0, 255, 0, 255])).unwrap();
        assert_eq!(
            compositor.scale_frame(larger).unwrap(),
            on_screen,
            "{size:?}"
        );
    }
}

#[test]
fn freeze_at_weight_one_after_a_size_change_holds_the_newest_frame_at_its_size() {
    let mut compositor = black_then_white();
    let larger = Size::new(24, 20);
    let (_reference, expected) = rendered(&gradient(larger));
    compositor.render(&gradient(larger)).unwrap();
    compositor.freeze().unwrap();
    compositor.set_mix(0.0);
    compositor.render(&flat(SIZE, [9, 9, 9, 255])).unwrap();
    assert_eq!(compositor.scale_frame(larger).unwrap(), expected);
}

#[test]
fn rejected_packet_leaves_what_is_shown_unchanged() {
    let (mut compositor, frame) = rendered(&gradient(SIZE));
    let invalid = packet(
        SIZE,
        vec![Op::Plane {
            clip: Rect::from_size(SIZE),
        }],
    );
    assert!(compositor.render(&invalid).is_err());
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), frame);
}

// Swapchain lifetime, on a headless surface.

/// A compositor for a headless surface with validation. A missing capability is a failure with
/// a message that says what is missing, never a skip.
fn surface_compositor(log: &ValidationLog) -> VkCompositor {
    VkCompositor::with_headless_surface(test_device(), Some(log)).unwrap_or_else(|error| {
        panic!(
            "presentation cannot be tested here: no compositor for a headless surface on the \
             test device: {error}"
        )
    })
}

#[test]
fn frames_are_presented_resized_presented_again_and_torn_down_cleanly() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log);
    if !log.is_active() {
        eprintln!("validation layer not installed; presenting unvalidated");
    }

    // No size yet: nothing to present into, and no error.
    assert_eq!(compositor.present(), Ok(false));
    assert_eq!(compositor.swapchain_extent(), None);

    // A black window before the first frame.
    compositor.resize_surface(Size::new(64, 48));
    assert_eq!(compositor.present(), Ok(true));
    assert_eq!(compositor.swapchain_extent(), Some((64, 48)));

    for color in [WHITE, [200, 100, 50, 255], [1, 2, 3, 255]] {
        compositor.render(&flat(SIZE, color)).unwrap();
        assert_eq!(compositor.present(), Ok(true));
    }
    assert_clean(&log, "while presenting");

    compositor.resize_surface(Size::new(100, 75));
    compositor.render(&gradient(SIZE)).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    for color in [BLACK, WHITE] {
        compositor
            .render(&halves(SIZE, color, [7, 77, 177, 255]))
            .unwrap();
        assert_eq!(compositor.present(), Ok(true));
    }
    assert_eq!(compositor.swapchain_extent(), Some((100, 75)));
    assert_clean(&log, "after a resize");

    // The same path an out-of-date report takes.
    compositor.invalidate_swapchain();
    assert_eq!(compositor.present(), Ok(true));
    assert_eq!(compositor.swapchain_extent(), Some((100, 75)));

    // Freeze and a transition while presenting.
    compositor.freeze().unwrap();
    for step in 0..=4 {
        compositor.set_mix(step as f32 / 4.0);
        compositor.render(&flat(SIZE, WHITE)).unwrap();
        assert_eq!(compositor.present(), Ok(true));
    }

    // A frame of another logical size mid-session.
    let other = Size::new(32, 24);
    compositor.render(&halves(other, BLACK, WHITE)).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    assert_clean(&log, "after presenting everything");

    // What was presented is what the off-screen path gives for the same window.
    let shown = compositor.scale_frame(Size::new(100, 75)).unwrap();
    assert_eq!(shown.size, Size::new(100, 75));

    drop(compositor);
    assert_clean(&log, "while the presenting compositor was dropped");
}

#[test]
fn window_without_pixels_presents_nothing_and_raises_no_error() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log);
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    compositor.resize_surface(Size::new(64, 48));
    assert_eq!(compositor.present(), Ok(true));

    for minimised in [Size::new(0, 48), Size::new(64, 0), Size::new(0, 0)] {
        compositor.resize_surface(minimised);
        assert_eq!(compositor.present(), Ok(false), "{minimised:?}");
        // Rendering goes on while nothing is shown.
        compositor.render(&flat(SIZE, BLACK)).unwrap();
    }
    compositor.resize_surface(Size::new(80, 60));
    assert_eq!(compositor.present(), Ok(true));
    assert_eq!(compositor.swapchain_extent(), Some((80, 60)));

    assert_clean(&log, "around a minimised window");
    drop(compositor);
    assert_clean(&log, "while the compositor was dropped");
}

#[test]
fn presenting_compositor_is_created_and_dropped_repeatedly_without_leaks() {
    let log = ValidationLog::new();
    for round in 0..10 {
        let mut compositor = surface_compositor(&log);
        compositor.resize_surface(Size::new(48, 36));
        if round % 2 == 0 {
            compositor.render(&flat(SIZE, WHITE)).unwrap();
            compositor.freeze().unwrap();
        }
        assert_eq!(compositor.present(), Ok(true));
        drop(compositor);
        assert_clean(&log, &format!("in round {round}"));
    }
}
