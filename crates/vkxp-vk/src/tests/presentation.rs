//! The scenarios of `specs/vulkan-presentation/spec.md`: scaling and mixing through the
//! off-screen target, and the swapchain on a headless surface. No test opens a window.

use vkxp_compose_ref::fixtures::{self, bitmap, sprite};
use vkxp_proto::{BitmapId, Color, FramePacket, FrameRate, Op, Rect, Size};

use super::harness::{TestCompositor, assert_clean, require_isolation, test_device};
use crate::{
    DeviceChoice, DeviceError, DeviceKind, Filter, Frame, Mix, ValidationLog, VkCompositor, VkError,
};

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

// Scaling filters.

/// Whether each column of `frame` is black, white, or something in between (`None`), having
/// checked that it is the same grey from top to bottom.
fn column_kinds(frame: &Frame) -> Vec<Option<bool>> {
    (0..frame.size.width)
        .map(|x| {
            let first = pixel(frame, x, 0);
            assert_eq!((first[1], first[2], first[3]), (first[0], first[0], 255));
            for y in 0..frame.size.height {
                assert_eq!(pixel(frame, x, y), first, "({x}, {y})");
            }
            match first[0] {
                0 => Some(false),
                255 => Some(true),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn auto_filter_at_a_whole_number_factor_shows_blocks_of_exact_pixels() {
    let size = Size::new(4, 4);
    let ops = (0..16u8)
        .map(|i| Op::Clear {
            clip: Rect::new(i32::from(i % 4), i32::from(i / 4), 1, 1),
            color: Color::new(17 * i, 255 - 13 * i, 40 + 7 * i, 255),
        })
        .collect();
    let (mut compositor, frame) = rendered(&packet(size, ops));
    assert_eq!(compositor.filter(), Filter::Auto);
    let scaled = compositor.scale_frame(Size::new(12, 12)).unwrap();
    assert_eq!(scaled, blocks(&frame, 3));
}

#[test]
fn sharp_filter_at_a_fractional_factor_keeps_stripes_pure_with_one_mixed_column_between() {
    let (mut compositor, _) = rendered(&columns(SIZE));
    compositor.set_filter(Filter::Sharp);
    // 2.5 times 20 by 18.
    let kinds = column_kinds(&compositor.scale_frame(Size::new(50, 45)).unwrap());
    for stripe in 0..SIZE.width {
        // The output columns that lie wholly inside the stripe.
        let inside = (stripe * 5).div_ceil(2)..(stripe * 5 + 5) / 2;
        assert!(!inside.is_empty());
        for x in inside {
            assert_eq!(kinds[x as usize], Some(stripe % 2 == 1), "column {x}");
        }
    }
    for pair in kinds.windows(2) {
        assert!(pair[0].is_some() || pair[1].is_some(), "{kinds:?}");
    }
}

#[test]
fn linear_filter_mixes_columns_that_sharp_bilinear_keeps_pure() {
    let (mut compositor, _) = rendered(&columns(SIZE));
    let window = Size::new(50, 45);
    compositor.set_filter(Filter::Sharp);
    let sharp = column_kinds(&compositor.scale_frame(window).unwrap());
    compositor.set_filter(Filter::Linear);
    let linear = column_kinds(&compositor.scale_frame(window).unwrap());
    let softened = (0..sharp.len())
        .filter(|&x| sharp[x].is_some() && linear[x].is_none())
        .count();
    assert!(softened > 0, "{sharp:?} {linear:?}");
}

#[test]
fn nearest_filter_at_a_fractional_factor_shows_the_pixel_under_each_centre() {
    let (mut compositor, frame) = rendered(&gradient(SIZE));
    compositor.set_filter(Filter::Nearest);
    let scaled = compositor.scale_frame(Size::new(50, 45)).unwrap();
    for y in 0..45 {
        for x in 0..50 {
            // The centre of output pixel x lies at (x + 0.5) / 2.5 frame pixels.
            let expected = pixel(&frame, (2 * x + 1) / 5, (2 * y + 1) / 5);
            assert_eq!(pixel(&scaled, x, y), expected, "({x}, {y})");
        }
    }
}

#[test]
fn filter_does_not_change_a_mix_at_factor_one() {
    let mut compositor = detailed_fade();
    let shown = [Filter::Nearest, Filter::Linear, Filter::Sharp].map(|filter| {
        compositor.set_filter(filter);
        compositor.scale_frame(SIZE).unwrap()
    });
    assert_eq!(shown[0], shown[1]);
    assert_eq!(shown[0], shown[2]);
    // The mix is one: neither of the two frames alone.
    compositor.set_mix(Mix::Newest);
    assert_ne!(compositor.scale_frame(SIZE).unwrap(), shown[0]);
    compositor.set_mix(Mix::HELD);
    assert_ne!(compositor.scale_frame(SIZE).unwrap(), shown[0]);
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

/// The step of a cross-fade at which the newest frame weighs `weight` of 256.
fn fade(weight: u8) -> Mix {
    Mix::Step {
        weight,
        threshold: 0,
    }
}

/// The step of a masked transition with the threshold `threshold`.
fn under_mask(threshold: u8) -> Mix {
    Mix::Step {
        weight: threshold,
        threshold,
    }
}

#[test]
fn mix_starts_as_the_newest_frame_alone() {
    let mut compositor = TestCompositor::new();
    assert_eq!(compositor.mix(), Mix::Newest);
    compositor.set_mix(fade(64));
    assert_eq!(compositor.mix(), fade(64));
}

#[test]
fn mix_end_points_show_the_held_frame_and_the_newest_frame() {
    let mut compositor = black_then_white();
    let whole = Rect::from_size(SIZE);
    compositor.set_mix(Mix::HELD);
    assert_all(&compositor.scale_frame(SIZE).unwrap(), whole, BLACK, "held");
    compositor.set_mix(Mix::Newest);
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
    compositor.set_mix(fade(128));
    for size in [SIZE, Size::new(40, 36), Size::new(30, 27)] {
        let scaled = compositor.scale_frame(size).unwrap();
        // (0 * 128 + 255 * 128) >> 8, mixed before scaling.
        assert_all(&scaled, Rect::from_size(size), [127, 127, 127, 255], "mix");
    }
}

// Masked transition mix.

const MASK: BitmapId = BitmapId::new(7, 0);
const RED: [u8; 4] = [200, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 200, 255];

/// The held frame is red and the newest is blue, and a grey mask of `size` pixels whose level
/// is `level(x, y)` came with the newest frame.
fn red_then_blue_under(size: Size, level: impl Fn(u32, u32) -> u8) -> TestCompositor {
    red_then_blue(SIZE, size, level)
}

/// The same with frames of `frame` pixels.
fn red_then_blue(frame: Size, size: Size, level: impl Fn(u32, u32) -> u8) -> TestCompositor {
    let mut compositor = TestCompositor::new();
    compositor.render(&flat(frame, RED)).unwrap();
    compositor.freeze().unwrap();
    let mut rgba = Vec::new();
    for y in 0..size.height {
        for x in 0..size.width {
            // The alpha does not count.
            rgba.extend_from_slice(&[level(x, y), level(x, y), level(x, y), 0]);
        }
    }
    let mut newest = flat(frame, BLUE);
    newest.bitmaps = bitmap(MASK, size.width, size.height, rgba).to_vec();
    compositor.render(&newest).unwrap();
    compositor
}

/// Black on the left half and white on the right half.
fn left_dark(size: Size) -> impl Fn(u32, u32) -> u8 {
    move |x, _| if x < size.width / 2 { 0 } else { 255 }
}

#[test]
fn masked_mix_shows_the_newest_frame_on_the_dark_side_first() {
    let mut compositor = red_then_blue_under(SIZE, left_dark(SIZE));
    compositor.set_mask(Some((MASK, 1)));
    compositor.set_mix(under_mask(127));
    let half = SIZE.width / 2;
    // At two whole factors.
    for (size, factor) in [(SIZE, 1), (Size::new(40, 36), 2)] {
        let scaled = compositor.scale_frame(size).unwrap();
        let (left, right) = (
            Rect::new(0, 0, half * factor, size.height),
            Rect::new((half * factor) as i32, 0, half * factor, size.height),
        );
        assert_all(&scaled, left, BLUE, "newest");
        assert_all(&scaled, right, RED, "held");
    }
}

#[test]
fn masked_mix_has_a_soft_edge_as_wide_as_the_vagueness() {
    // A horizontal gradient, one level per column.
    let size = Size::new(256, 4);
    let mut compositor = TestCompositor::new();
    compositor.render(&flat(size, RED)).unwrap();
    compositor.freeze().unwrap();
    let rgba = (0..size.width * size.height)
        .flat_map(|index| {
            let level = (index % size.width) as u8;
            [level, level, level, 255]
        })
        .collect();
    let mut newest = flat(size, BLUE);
    newest.bitmaps = bitmap(MASK, size.width, size.height, rgba).to_vec();
    compositor.render(&newest).unwrap();
    compositor.set_mask(Some((MASK, 64)));
    compositor.set_mix(under_mask(64));

    let scaled = compositor.scale_frame(size).unwrap();
    let row: Vec<[u8; 4]> = (0..size.width).map(|x| pixel(&scaled, x, 0)).collect();
    let newest_end = row.iter().position(|pixel| *pixel != BLUE).unwrap();
    let held_start = row.iter().rposition(|pixel| *pixel != RED).unwrap() + 1;
    // Levels up to the threshold show the newest frame, levels from the threshold plus the
    // vagueness on show the held frame.
    assert_eq!(newest_end, 65);
    assert_eq!(held_start, 128);
    // In between the two are mixed, more of the held frame with every column.
    let band = &row[newest_end..held_start];
    assert!(band.iter().all(|[r, _, b, _]| *r > 0 && *b > 0));
    assert!(band.windows(2).all(|pair| pair[0][0] <= pair[1][0]));
    assert!(band[0][0] < 20 && band[band.len() - 1][0] > 180);
}

#[test]
fn masked_mix_at_the_end_is_the_newest_frame_for_any_mask() {
    type Level = fn(u32, u32) -> u8;
    let masks: [(Size, Level); 3] = [
        (SIZE, |_, _| 255),
        (SIZE, |x, y| (x * 13 + y * 7) as u8),
        (Size::new(3, 5), |x, _| (x * 127) as u8),
    ];
    for (size, level) in masks {
        let mut compositor = red_then_blue_under(size, level);
        compositor.set_mix(Mix::Newest);
        for vagueness in [0, 1, 64, 256] {
            compositor.set_mask(Some((MASK, vagueness)));
            for window in [SIZE, Size::new(30, 27)] {
                let scaled = compositor.scale_frame(window).unwrap();
                assert_all(&scaled, Rect::from_size(window), BLUE, "newest");
            }
        }
    }
}

#[test]
fn mask_smaller_than_the_frame_is_tiled() {
    let frame = Size::new(640, 480);
    // Dark in the top left and the bottom right quarter of the mask.
    let dark = |x: u32, y: u32| (x < 32) == (y < 24);
    let level = |x, y| if dark(x, y) { 0 } else { 255 };
    let mut compositor = red_then_blue(frame, Size::new(64, 48), level);
    compositor.set_mask(Some((MASK, 1)));
    compositor.set_mix(under_mask(127));

    let scaled = compositor.scale_frame(frame).unwrap();
    for y in 0..frame.height {
        for x in 0..frame.width {
            let expected = if dark(x % 64, y % 48) { BLUE } else { RED };
            assert_eq!(pixel(&scaled, x, y), expected, "({x}, {y})");
        }
    }
}

#[test]
fn vagueness_of_zero_is_a_hard_edge_that_keeps_the_threshold_level() {
    // A horizontal gradient, one level per column.
    let size = Size::new(256, 2);
    let mut compositor = red_then_blue(size, size, |x, _| x as u8);
    compositor.set_mask(Some((MASK, 0)));
    compositor.set_mix(under_mask(100));
    let scaled = compositor.scale_frame(size).unwrap();
    assert_all(&scaled, Rect::new(0, 0, 100, 2), BLUE, "newest");
    assert_all(&scaled, Rect::new(100, 0, 156, 2), RED, "held");
    // With a vagueness the level of the threshold shows the newest frame.
    compositor.set_mask(Some((MASK, 1)));
    let scaled = compositor.scale_frame(size).unwrap();
    assert_all(&scaled, Rect::new(0, 0, 101, 2), BLUE, "newest");
    assert_all(&scaled, Rect::new(101, 0, 155, 2), RED, "held");
}

// The samples measured on the original runtime.

const FADE_SAMPLES: &str = include_str!("data/transition-samples.txt");
const MASK_SAMPLES: &str = include_str!("data/transition-mask-samples.txt");

/// The numbers of each sample line of `text` whose first word is `kind`, or of every sample
/// line when `kind` is empty. Comment lines start with `#`.
fn samples(text: &str, kind: &str) -> Vec<Vec<u32>> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .filter(|line| kind.is_empty() || line.split(' ').next() == Some(kind))
        .map(|line| {
            line.split(' ')
                .filter_map(|word| word.parse().ok())
                .collect()
        })
        .collect()
}

/// A frame one row high per entry of `rows`, each row one colour per column.
fn rows(width: u32, rows: &[&dyn Fn(u32) -> [u8; 3]]) -> FramePacket {
    let size = Size::new(width, rows.len() as u32);
    let mut ops = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        for x in 0..width {
            let [r, g, b] = row(x);
            ops.push(Op::Clear {
                clip: Rect::new(x as i32, y as i32, 1, 1),
                color: Color::new(r, g, b, 255),
            });
        }
    }
    packet(size, ops)
}

/// The step at which `Graphics.transition(duration)` returns, where the samples were taken.
fn last_step(duration: u32) -> Mix {
    Mix::Step {
        weight: (255 * (duration - 1) / duration) as u8,
        threshold: ((255 * (duration - 1)).saturating_sub(1) / duration) as u8,
    }
}

#[test]
fn cross_fade_equals_every_measured_sample() {
    let all = samples(FADE_SAMPLES, "");
    assert!(all.len() >= 200, "{} samples", all.len());
    let mut compositor = TestCompositor::new();
    for duration in [1, 2, 3, 4, 7, 8, 20, 40, 100] {
        let group: Vec<&Vec<u32>> = all.iter().filter(|line| line[0] == duration).collect();
        assert!(!group.is_empty(), "no samples of duration {duration}");
        let width = group.len() as u32;
        let (old, new) = (
            |x: u32| [group[x as usize][1] as u8; 3],
            |x: u32| [group[x as usize][2] as u8; 3],
        );
        compositor.set_mix(Mix::Newest);
        compositor.render(&rows(width, &[&old])).unwrap();
        compositor.freeze().unwrap();
        compositor.render(&rows(width, &[&new])).unwrap();
        compositor.set_mix(last_step(duration));
        let shown = compositor.scale_frame(Size::new(width, 1)).unwrap();
        for (x, line) in group.iter().enumerate() {
            let expected = line[3] as u8;
            assert_eq!(
                pixel(&shown, x as u32, 0),
                [expected, expected, expected, 255],
                "{line:?}"
            );
        }
    }
}

/// Shows the step at which a transition of `duration` frames returns, from a held frame of
/// the rows `old` to a newest frame of the rows `new`, under a mask one row high with the
/// colours `mask`, which came as the bitmap `id`.
fn masked_last_step(
    compositor: &mut TestCompositor,
    id: BitmapId,
    (duration, vagueness): (u32, u32),
    mask: &[[u8; 3]],
    (old, new): ([[u8; 3]; 2], [[u8; 3]; 2]),
) -> Frame {
    let width = mask.len() as u32;
    compositor.set_mix(Mix::Newest);
    compositor.set_mask(None);
    let old = rows(width, &[&|_| old[0], &|_| old[1]]);
    compositor.render(&old).unwrap();
    compositor.freeze().unwrap();
    let mut new = rows(width, &[&|_| new[0], &|_| new[1]]);
    let rgba = mask
        .iter()
        .flat_map(|[r, g, b]| [*r, *g, *b, 255])
        .collect();
    new.bitmaps = bitmap(id, width, 1, rgba).to_vec();
    compositor.render(&new).unwrap();
    compositor.set_mask(Some((id, vagueness)));
    compositor.set_mix(last_step(duration));
    compositor.scale_frame(Size::new(width, 2)).unwrap()
}

/// The colours of the mask probe: white to black in the first row, and a pair of colours.
const PROBE: ([[u8; 3]; 2], [[u8; 3]; 2]) =
    ([[255, 255, 255], [200, 50, 0]], [[0, 0, 0], [50, 200, 255]]);

#[test]
fn masked_mix_equals_every_measured_sample_of_a_grey_mask() {
    // w N vague L -> a | r g b
    let all = samples(MASK_SAMPLES, "w");
    assert!(all.len() >= 300, "{} samples", all.len());
    let mut groups: Vec<(u32, u32)> = all.iter().map(|line| (line[0], line[1])).collect();
    groups.dedup();
    assert!(groups.len() >= 50, "{} groups", groups.len());
    let mut compositor = TestCompositor::new();
    for (index, step) in groups.into_iter().enumerate() {
        let group: Vec<&Vec<u32>> = all
            .iter()
            .filter(|line| (line[0], line[1]) == step)
            .collect();
        let mask: Vec<[u8; 3]> = group.iter().map(|line| [line[2] as u8; 3]).collect();
        let id = BitmapId::new(index as u32, 0);
        let shown = masked_last_step(&mut compositor, id, step, &mask, PROBE);
        for (x, line) in group.iter().enumerate() {
            let x = x as u32;
            let a = line[3] as u8;
            assert_eq!(pixel(&shown, x, 0), [a, a, a, 255], "{line:?}");
            let expected = [line[4] as u8, line[5] as u8, line[6] as u8, 255];
            assert_eq!(pixel(&shown, x, 1), expected, "{line:?}");
        }
    }
}

#[test]
fn mask_value_equals_every_selected_sample_of_a_coloured_mask() {
    // lum r g b -> L
    let all = samples(MASK_SAMPLES, "lum");
    assert!(all.len() >= 100, "{} samples", all.len());
    let mask: Vec<[u8; 3]> = all
        .iter()
        .map(|line| [line[0] as u8, line[1] as u8, line[2] as u8])
        .collect();
    let mut compositor = TestCompositor::new();
    // With a duration of 2 and a vagueness of 255 the first row shows the mask value less 127.
    let shown = masked_last_step(&mut compositor, MASK, (2, 255), &mask, PROBE);
    for (x, line) in all.iter().enumerate() {
        let expected = (line[3] - 127) as u8;
        assert_eq!(
            pixel(&shown, x as u32, 0),
            [expected, expected, expected, 255],
            "{line:?}"
        );
    }
}

#[test]
fn mask_that_is_not_a_live_bitmap_leaves_the_plain_mix() {
    let mut compositor = red_then_blue_under(SIZE, left_dark(SIZE));
    assert!(compositor.has_bitmap(MASK));
    let absent = BitmapId::new(9, 0);
    assert!(!compositor.has_bitmap(absent));
    compositor.set_mask(Some((absent, 1)));
    compositor.set_mix(Mix::Step {
        weight: 128,
        threshold: 127,
    });
    let plain = compositor.scale_frame(SIZE).unwrap();
    assert_all(&plain, Rect::from_size(SIZE), [100, 0, 100, 255], "mix");

    // The same once the mask in use has been destroyed, and again without a mask.
    compositor.set_mask(Some((MASK, 1)));
    assert_ne!(compositor.scale_frame(SIZE).unwrap(), plain);
    let mut next = flat(SIZE, BLUE);
    next.bitmaps = vec![vkxp_proto::BitmapDelta::Destroy { id: MASK }];
    compositor.render(&next).unwrap();
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), plain);
    compositor.set_mask(None);
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), plain);
}

#[test]
fn freeze_during_a_masked_mix_holds_the_image_on_screen() {
    let mut compositor = red_then_blue_under(SIZE, left_dark(SIZE));
    compositor.set_mask(Some((MASK, 1)));
    compositor.set_mix(under_mask(127));
    let shown = compositor.scale_frame(SIZE).unwrap();
    compositor.freeze().unwrap();
    compositor.set_mask(None);
    compositor.set_mix(Mix::HELD);
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), shown);
}

#[test]
fn mix_is_applied_per_channel() {
    let mut compositor = TestCompositor::new();
    compositor.render(&flat(SIZE, [200, 0, 100, 255])).unwrap();
    compositor.freeze().unwrap();
    compositor.render(&flat(SIZE, [0, 100, 100, 255])).unwrap();
    compositor.set_mix(fade(64));
    let scaled = compositor.scale_frame(SIZE).unwrap();
    assert_all(&scaled, Rect::from_size(SIZE), [150, 25, 100, 255], "mix");
}

#[test]
fn held_frame_stays_shown_while_later_frames_are_rendered() {
    let (mut compositor, first) = rendered(&gradient(SIZE));
    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    for color in [WHITE, [9, 8, 7, 255], [0, 255, 0, 255]] {
        compositor.render(&flat(SIZE, color)).unwrap();
        assert_eq!(compositor.scale_frame(SIZE).unwrap(), first);
        let doubled = compositor.scale_frame(Size::new(40, 36)).unwrap();
        assert_eq!(doubled, blocks(&first, 2));
    }
    // The newest frame was rendered all along.
    let newest = opaque(compositor.read_frame().unwrap().unwrap());
    assert_all(&newest, Rect::from_size(SIZE), [0, 255, 0, 255], "newest");
    compositor.set_mix(Mix::Newest);
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), newest);
}

#[test]
fn freezing_again_replaces_the_held_frame() {
    let mut compositor = black_then_white();
    compositor.freeze().unwrap();
    compositor.render(&flat(SIZE, [10, 20, 30, 255])).unwrap();
    compositor.set_mix(Mix::HELD);
    let scaled = compositor.scale_frame(SIZE).unwrap();
    assert_all(&scaled, Rect::from_size(SIZE), WHITE, "second held frame");
}

#[test]
fn freeze_during_a_mix_holds_the_mixed_image() {
    let whole = Rect::from_size(SIZE);
    let grey = [63, 63, 63, 255];
    let mut compositor = black_then_white();
    compositor.set_mix(fade(64));
    let on_screen = compositor.scale_frame(SIZE).unwrap();
    assert_all(
        &on_screen,
        whole,
        grey,
        "a quarter of the way from black to white",
    );

    // Freeze in the middle of the fade, as a session does, and render on.
    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    for color in [[0, 255, 0, 255], [255, 0, 255, 255]] {
        compositor.render(&flat(SIZE, color)).unwrap();
        assert_eq!(compositor.scale_frame(SIZE).unwrap(), on_screen);
        let doubled = compositor.scale_frame(Size::new(40, 36)).unwrap();
        assert_eq!(doubled, blocks(&on_screen, 2));
    }

    // The next fade starts from the grey, not from the black or the white behind it.
    compositor.set_mix(fade(128));
    let half = compositor.scale_frame(SIZE).unwrap();
    for y in 0..SIZE.height {
        for x in 0..SIZE.width {
            let [r, g, b, a] = pixel(&half, x, y);
            assert_eq!((r, g, b, a), (159, 31, 159, 255), "({x}, {y})");
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
    compositor.set_mix(fade(192));
    let on_screen = compositor.scale_frame(SIZE).unwrap();
    assert_ne!(on_screen, first);

    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    assert_eq!(compositor.scale_frame(SIZE).unwrap(), on_screen);
}

/// A frame in which every pixel has its own grey, different from its neighbours'.
fn detailed(seed: u32) -> FramePacket {
    let mut ops = Vec::new();
    for y in 0..SIZE.height {
        for x in 0..SIZE.width {
            let grey = ((x * 37 + y * 71 + seed * 53) % 256) as u8;
            ops.push(Op::Clear {
                clip: Rect::new(x as i32, y as i32, 1, 1),
                color: Color::new(grey, grey, grey, 255),
            });
        }
    }
    packet(SIZE, ops)
}

/// A compositor a quarter of the way from one detailed frame to another.
fn detailed_fade() -> TestCompositor {
    let mut compositor = TestCompositor::new();
    compositor.render(&detailed(0)).unwrap();
    compositor.freeze().unwrap();
    compositor.render(&detailed(1)).unwrap();
    compositor.set_mix(fade(64));
    compositor
}

/// Freezes in the middle of [`detailed_fade`] as shown in `window` and asserts that the
/// window shows the same pixels right after, after later frames and when it is drawn again.
fn assert_freeze_keeps(window: Size) {
    let mut compositor = detailed_fade();
    let on_screen = compositor.scale_frame(window).unwrap();

    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    assert!(
        compositor.scale_frame(window).unwrap() == on_screen,
        "{window:?}: the freeze changed what is shown"
    );
    for (round, later) in [
        detailed(2),
        flat(SIZE, WHITE),
        flat(Size::new(32, 24), BLACK),
    ]
    .iter()
    .enumerate()
    {
        compositor.render(later).unwrap();
        for redraw in 0..2 {
            assert!(
                compositor.scale_frame(window).unwrap() == on_screen,
                "{window:?}: changed after later frame {round}, redraw {redraw}"
            );
        }
    }

    // The next fade starts from those pixels and ends on the newest frame.
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    compositor.set_mix(fade(128));
    assert!(compositor.scale_frame(window).unwrap() != on_screen);
    compositor.set_mix(Mix::Newest);
    let end = compositor.scale_frame(window).unwrap();
    let rect = crate::letterbox(SIZE, window);
    assert_all(&end, rect, WHITE, "the newest frame alone");
}

#[test]
fn freeze_during_a_mix_keeps_the_pixels_shown_at_a_fractional_factor() {
    // 1.5 times: mixing first and scaling the rounded result would change some of these.
    assert_freeze_keeps(Size::new(30, 27));
}

#[test]
fn freeze_during_a_mix_keeps_the_pixels_shown_at_other_factors_and_with_bars() {
    for window in [
        // 1.5 times with bars left and right, then above and below.
        Size::new(50, 27),
        Size::new(30, 40),
        // 2.25 times, and a reduction.
        Size::new(45, 41),
        Size::new(13, 11),
        // Whole factors.
        SIZE,
        Size::new(40, 36),
        Size::new(70, 54),
    ] {
        assert_freeze_keeps(window);
    }
}

#[test]
fn freeze_during_a_mix_at_a_fractional_factor_keeps_what_the_swapchain_shows() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log);
    let window = Size::new(50, 27);
    compositor.resize_surface(window);
    compositor.render(&detailed(0)).unwrap();
    compositor.freeze().unwrap();
    compositor.render(&detailed(1)).unwrap();
    compositor.set_mix(fade(64));
    assert_eq!(compositor.present(), Ok(true));
    let on_screen = compositor.scale_frame(window).unwrap();
    // Looking at it off-screen at another size does not change what the window shows.
    compositor.scale_frame(Size::new(40, 36)).unwrap();

    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    for later in [detailed(2), flat(SIZE, WHITE)] {
        compositor.render(&later).unwrap();
        assert_eq!(compositor.present(), Ok(true));
        assert!(compositor.scale_frame(window).unwrap() == on_screen);
    }
    assert_clean(&log, "freezing a scaled mix while presenting");
    drop(compositor);
    assert_clean(&log, "while the compositor was dropped");
}

#[test]
fn freeze_at_weight_zero_keeps_the_held_frame() {
    let mut compositor = black_then_white();
    compositor.set_mix(Mix::HELD);
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
    compositor.set_mix(Mix::HELD);
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
    compositor.set_mix(Mix::HELD);
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
    compositor.set_mix(Mix::HELD);
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

    compositor.set_mix(Mix::HELD);
    let start = compositor.scale_frame(window).unwrap();
    assert_all(&start, Rect::from_size(window), WHITE, "held frame alone");

    // A quarter of the way: white towards the newest frame where that lies, and towards the
    // black of its bars elsewhere.
    compositor.set_mix(fade(64));
    let quarter = compositor.scale_frame(window).unwrap();
    assert_near(&quarter, middle, [201, 211, 221, 255], "quarter, frame");
    assert_near(&quarter, top, [191, 191, 191, 255], "quarter, top bar");
    assert_near(
        &quarter,
        bottom,
        [191, 191, 191, 255],
        "quarter, bottom bar",
    );

    compositor.set_mix(fade(128));
    let half = compositor.scale_frame(window).unwrap();
    assert_near(&half, middle, [148, 168, 188, 255], "half, frame");
    assert_near(&half, top, [128, 128, 128, 255], "half, top bar");

    compositor.set_mix(Mix::Newest);
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
    compositor.set_mix(fade(128));
    let on_screen = compositor.scale_frame(larger).unwrap();
    assert_near(
        &on_screen,
        Rect::from_size(larger),
        [148, 168, 188, 255],
        "half way",
    );

    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    for size in [larger, SIZE, Size::new(64, 48)] {
        compositor.render(&flat(size, [0, 255, 0, 255])).unwrap();
        assert_eq!(
            compositor.scale_frame(larger).unwrap(),
            on_screen,
            "{size:?}"
        );
    }
}

/// A compositor half way from a held white 20 by 18 frame to a 32 by 24 frame of another
/// shape, with the windows used to look at it.
fn fading_across_shapes(compositor: &mut VkCompositor) {
    compositor.render(&flat(SIZE, WHITE)).unwrap();
    compositor.freeze().unwrap();
    compositor
        .render(&flat(Size::new(32, 24), [40, 80, 120, 255]))
        .unwrap();
    compositor.set_mix(fade(128));
}

#[test]
fn freeze_during_a_transition_across_shapes_keeps_the_geometry_on_screen() {
    let mut compositor = TestCompositor::new();
    fading_across_shapes(&mut compositor);
    // The window has the shape of the held frame, so that fills it and the newest frame
    // leaves bars of 6 rows, where the held white fades towards black.
    let window = Size::new(80, 72);
    let on_screen = compositor.scale_frame(window).unwrap();
    assert_near(
        &on_screen,
        Rect::new(0, 0, 80, 6),
        [128, 128, 128, 255],
        "top bar",
    );
    assert_near(
        &on_screen,
        Rect::new(0, 6, 80, 60),
        [148, 168, 188, 255],
        "frame",
    );
    assert_near(
        &on_screen,
        Rect::new(0, 66, 80, 6),
        [128, 128, 128, 255],
        "bottom bar",
    );

    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    assert_eq!(
        compositor.scale_frame(window).unwrap(),
        on_screen,
        "right after"
    );

    // Later frames of any size, and drawing the window again, leave it as it is.
    for size in [Size::new(32, 24), SIZE, Size::new(64, 20), Size::new(9, 40)] {
        compositor.render(&flat(size, [0, 255, 0, 255])).unwrap();
        for redraw in 0..2 {
            assert_eq!(
                compositor.scale_frame(window).unwrap(),
                on_screen,
                "{size:?}, redraw {redraw}"
            );
        }
    }

    // The next transition starts from that picture.
    compositor.set_mix(fade(128));
    let fading = compositor.scale_frame(window).unwrap();
    assert_ne!(fading, on_screen);
    compositor.set_mix(Mix::HELD);
    assert_eq!(compositor.scale_frame(window).unwrap(), on_screen);
}

#[test]
fn freeze_during_a_transition_across_shapes_keeps_the_geometry_in_a_tall_window() {
    let mut compositor = TestCompositor::new();
    fading_across_shapes(&mut compositor);
    // Neither frame fills this one: both have bars, of different heights.
    let window = Size::new(60, 100);
    let on_screen = compositor.scale_frame(window).unwrap();
    assert_all(&on_screen, Rect::new(0, 0, 60, 23), BLACK, "above both");
    assert_near(
        &on_screen,
        Rect::new(0, 23, 60, 4),
        [128, 128, 128, 255],
        "held only",
    );
    assert_near(
        &on_screen,
        Rect::new(0, 27, 60, 45),
        [148, 168, 188, 255],
        "both",
    );

    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    compositor.render(&flat(Size::new(50, 50), WHITE)).unwrap();
    assert_eq!(compositor.scale_frame(window).unwrap(), on_screen);
}

#[test]
fn freeze_during_a_transition_across_shapes_keeps_what_the_swapchain_shows() {
    let log = ValidationLog::new();
    let mut compositor = surface_compositor(&log);
    let window = Size::new(100, 90);
    compositor.resize_surface(window);
    fading_across_shapes(&mut compositor);
    assert_eq!(compositor.present(), Ok(true));
    // An off-screen look at another size does not change what the window is taken to show.
    compositor.scale_frame(Size::new(13, 17)).unwrap();
    let on_screen = compositor.scale_frame(window).unwrap();
    compositor.scale_frame(Size::new(13, 17)).unwrap();

    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
    for size in [Size::new(32, 24), Size::new(48, 16)] {
        compositor.render(&flat(size, [0, 255, 0, 255])).unwrap();
        assert_eq!(compositor.present(), Ok(true));
        assert_eq!(
            compositor.scale_frame(window).unwrap(),
            on_screen,
            "{size:?}"
        );
    }
    assert_clean(&log, "freezing across shapes while presenting");
    drop(compositor);
    assert_clean(&log, "while the compositor was dropped");
}

#[test]
fn freeze_at_weight_one_after_a_size_change_holds_the_newest_frame_at_its_size() {
    let mut compositor = black_then_white();
    let larger = Size::new(24, 20);
    let (_reference, expected) = rendered(&gradient(larger));
    compositor.render(&gradient(larger)).unwrap();
    compositor.freeze().unwrap();
    compositor.set_mix(Mix::HELD);
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
        compositor.set_mix(if step < 4 {
            fade(step * 64)
        } else {
            Mix::Newest
        });
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
