use super::*;

#[test]
fn frame_that_fits_exactly_fills_the_window() {
    let rect = letterbox(Size::new(640, 480), Size::new(1280, 960));
    assert_eq!(rect, Rect::new(0, 0, 1280, 960));
    assert!(is_whole_factor(Size::new(640, 480), &rect));
    assert_eq!(
        letterbox(Size::new(640, 480), Size::new(640, 480)),
        Rect::new(0, 0, 640, 480)
    );
}

#[test]
fn wide_window_gets_bars_left_and_right() {
    let rect = letterbox(Size::new(640, 480), Size::new(1920, 1080));
    assert_eq!(rect, Rect::new(240, 0, 1440, 1080));
    assert!(!is_whole_factor(Size::new(640, 480), &rect), "2.25");
}

#[test]
fn tall_window_gets_bars_above_and_below() {
    let rect = letterbox(Size::new(640, 480), Size::new(1280, 1200));
    assert_eq!(rect, Rect::new(0, 120, 1280, 960));
    assert!(is_whole_factor(Size::new(640, 480), &rect));
}

#[test]
fn smaller_window_scales_the_frame_down() {
    assert_eq!(
        letterbox(Size::new(640, 480), Size::new(320, 300)),
        Rect::new(0, 30, 320, 240)
    );
    // Odd leftovers are split with the extra pixel after the image.
    assert_eq!(
        letterbox(Size::new(4, 4), Size::new(7, 4)),
        Rect::new(1, 0, 4, 4)
    );
    // A side that does not divide is rounded down.
    assert_eq!(
        letterbox(Size::new(640, 480), Size::new(1000, 1000)),
        Rect::new(0, 125, 1000, 750)
    );
    assert_eq!(
        letterbox(Size::new(3, 2), Size::new(100, 10)),
        Rect::new(42, 0, 15, 10)
    );
}

#[test]
fn nothing_is_shown_without_pixels() {
    let frame = Size::new(640, 480);
    for window in [Size::new(0, 480), Size::new(640, 0), Size::new(0, 0)] {
        assert!(letterbox(frame, window).is_empty(), "{window:?}");
        assert_eq!(
            push_constants(same(frame), window, Mix::Newest, false, Filter::Auto),
            None
        );
    }
    assert!(letterbox(Size::new(0, 0), Size::new(640, 480)).is_empty());
    // One pixel wide: the scaled frame would be less than a pixel high.
    assert!(letterbox(frame, Size::new(1, 1000)).is_empty());
    assert!(letterbox(Size::new(u32::MAX, 1), Size::new(u32::MAX, u32::MAX)).width > 0);
}

#[test]
fn whole_factor_needs_the_same_whole_number_both_ways() {
    let frame = Size::new(4, 3);
    assert!(is_whole_factor(frame, &Rect::new(0, 0, 4, 3)));
    assert!(is_whole_factor(frame, &Rect::new(5, 0, 12, 9)));
    assert!(!is_whole_factor(frame, &Rect::new(0, 0, 6, 4)));
    assert!(!is_whole_factor(frame, &Rect::new(0, 0, 8, 9)));
    assert!(!is_whole_factor(frame, &Rect::new(0, 0, 2, 1)));
    assert!(!is_whole_factor(frame, &Rect::default()));
}

#[test]
fn filter_selects_the_sampling_flags() {
    let frame = Size::new(4, 4);
    let flags = |window: u32, filter| {
        words(push_constants(
            same(frame),
            Size::new(window, window),
            Mix::Newest,
            false,
            filter,
        ))[13]
    };
    let nearest = FLAG_NEAREST | FLAG_HELD_NEAREST | FLAG_SAME_GRID;
    assert_eq!(flags(12, Filter::Auto), nearest);
    assert_eq!(flags(10, Filter::Auto), FLAG_SAME_GRID);
    assert_eq!(flags(10, Filter::Nearest), nearest);
    assert_eq!(flags(12, Filter::Sharp), FLAG_SAME_GRID);
    assert_eq!(flags(12, Filter::Linear), FLAG_SAME_GRID | FLAG_LINEAR);
}

/// Both frames of one size, as when the logical size never changes.
fn same(size: Size) -> Sources {
    Sources {
        newest: size,
        held: size,
        vagueness: None,
    }
}

/// A step whose weight and threshold differ, so that neither is taken for the other.
const STEP: Mix = Mix::Step {
    weight: 64,
    threshold: 63,
};

fn words(bytes: Option<[u8; PUSH_CONSTANT_BYTES]>) -> Vec<u32> {
    crate::resources::read_words(&bytes.expect("something is visible"))
}

#[test]
fn push_constants_carry_the_rectangles_the_frame_sizes_the_weight_and_the_flags() {
    let frame = Size::new(640, 480);
    let whole = push_constants(
        same(frame),
        Size::new(1920, 1440),
        Mix::Newest,
        false,
        Filter::Auto,
    );
    let rect = [0, 0, 1920, 1440, 640, 480];
    assert_eq!(
        words(whole),
        [
            rect.as_slice(),
            rect.as_slice(),
            &[0, FLAG_NEAREST | FLAG_HELD_NEAREST | FLAG_SAME_GRID, 0, 0]
        ]
        .concat()
    );
    let fractional = push_constants(same(frame), Size::new(1920, 1080), STEP, true, Filter::Auto);
    let rect = [240, 0, 1440, 1080, 640, 480];
    assert_eq!(
        words(fractional),
        [
            rect.as_slice(),
            rect.as_slice(),
            &[64, FLAG_SRGB_TARGET | FLAG_SAME_GRID | FLAG_MIX, 63, 0]
        ]
        .concat()
    );
}

#[test]
fn push_constants_of_a_masked_draw_carry_the_flag_the_threshold_and_the_vagueness() {
    let frame = Size::new(640, 480);
    let masked = Sources {
        vagueness: Some(64),
        ..same(frame)
    };
    let constants = words(push_constants(masked, frame, STEP, false, Filter::Auto));
    assert_eq!(
        constants[12..],
        [
            64,
            FLAG_NEAREST | FLAG_HELD_NEAREST | FLAG_SAME_GRID | FLAG_MIX | FLAG_MASK,
            63,
            64
        ]
    );
    // Outside a transition the mask is not read.
    let alone = words(push_constants(
        masked,
        frame,
        Mix::Newest,
        false,
        Filter::Auto,
    ));
    assert_eq!(alone[13], FLAG_NEAREST | FLAG_HELD_NEAREST | FLAG_SAME_GRID);
    let filling = words(Some(filling_constants(masked, frame, STEP, Filter::Auto)));
    assert_eq!(filling, constants);
}

#[test]
fn filling_constants_are_those_of_the_letterbox_rectangle_moved_to_the_origin() {
    let frame = Size::new(20, 18);
    // Windows in which the frame is scaled by a factor that is not whole, with bars on
    // either axis, including one whose rectangle would not be found again from its size.
    for window in [
        Size::new(30, 27),
        Size::new(50, 27),
        Size::new(30, 40),
        Size::new(100, 10),
        Size::new(1920, 1080),
    ] {
        let rect = letterbox(frame, window);
        let in_window = words(push_constants(
            same(frame),
            window,
            STEP,
            false,
            Filter::Auto,
        ));
        let filling = words(Some(filling_constants(
            same(frame),
            Size::new(rect.width, rect.height),
            STEP,
            Filter::Auto,
        )));
        // The same sizes, weight and flags; only the origins differ.
        assert_eq!(filling[2..6], in_window[2..6], "{window:?}");
        assert_eq!(filling[8..], in_window[8..], "{window:?}");
        assert_eq!([filling[0], filling[1], filling[6], filling[7]], [0; 4]);
        // Shown again in that window, the captured rectangle lies where it was.
        assert_eq!(
            letterbox(Size::new(rect.width, rect.height), window),
            rect,
            "{window:?}"
        );
    }
}

#[test]
fn held_frame_of_another_size_gets_its_own_rectangle() {
    let sources = Sources {
        newest: Size::new(32, 24),
        held: Size::new(20, 18),
        vagueness: None,
    };
    // The held frame fills the target four times over; the newest is 2.5 times as wide
    // and leaves bars above and below.
    assert_eq!(
        words(push_constants(
            sources,
            Size::new(80, 72),
            STEP,
            false,
            Filter::Auto
        )),
        [
            0,
            6,
            80,
            60,
            32,
            24,
            0,
            0,
            80,
            72,
            20,
            18,
            64,
            FLAG_HELD_NEAREST | FLAG_MIX,
            63,
            0
        ]
    );
    // One of the two may have no pixel while the other still has.
    let thin = Sources {
        newest: Size::new(640, 480),
        held: Size::new(2, 100),
        vagueness: None,
    };
    let constants = words(push_constants(
        thin,
        Size::new(1, 50),
        STEP,
        false,
        Filter::Auto,
    ));
    assert_eq!(constants[..6], [0, 0, 0, 0, 640, 480]);
    assert_eq!(constants[6..12], [0, 0, 1, 50, 2, 100]);
}

fn format(format: vk::Format) -> vk::SurfaceFormatKHR {
    vk::SurfaceFormatKHR {
        format,
        color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
    }
}

fn plain(surface: vk::Format) -> Option<PresentFormat> {
    Some(PresentFormat::plain(format(surface), false))
}

#[test]
fn unorm_surface_format_is_preferred_over_srgb() {
    let offered = [
        format(vk::Format::B8G8R8A8_SRGB),
        format(vk::Format::R8G8B8A8_UNORM),
        format(vk::Format::B8G8R8A8_UNORM),
    ];
    // The same with and without the extension: nothing changes where a UNORM format is.
    for mutable_format in [false, true] {
        assert_eq!(
            choose_format(&offered, mutable_format),
            plain(vk::Format::B8G8R8A8_UNORM)
        );
        assert_eq!(
            choose_format(&offered[..2], mutable_format),
            plain(vk::Format::R8G8B8A8_UNORM)
        );
    }
}

/// Scenario "sRGB only, views supported".
#[test]
fn srgb_surface_format_is_drawn_through_its_unorm_twin_with_the_extension() {
    let chosen = choose_format(&[format(vk::Format::B8G8R8A8_SRGB)], true).unwrap();
    assert_eq!(
        chosen,
        PresentFormat {
            surface: format(vk::Format::B8G8R8A8_SRGB),
            view: vk::Format::B8G8R8A8_UNORM,
            srgb_target: false,
        }
    );
    assert!(chosen.is_mutable());

    for (srgb, twin) in std::iter::zip(SRGB_FORMATS, UNORM_FORMATS) {
        let offered = [format(vk::Format::R16G16B16A16_SFLOAT), format(srgb)];
        let chosen = choose_format(&offered, true).unwrap();
        assert_eq!(
            (chosen.surface, chosen.view, chosen.srgb_target),
            (format(srgb), twin, false)
        );
    }
}

/// Scenario "sRGB only, views not supported".
#[test]
fn srgb_surface_format_is_taken_with_the_inverse_transfer_without_the_extension() {
    let chosen = choose_format(&[format(vk::Format::B8G8R8A8_SRGB)], false).unwrap();
    assert_eq!(
        chosen,
        PresentFormat::plain(format(vk::Format::B8G8R8A8_SRGB), true)
    );
    assert!(!chosen.is_mutable());

    let offered = [
        format(vk::Format::R16G16B16A16_SFLOAT),
        format(vk::Format::R8G8B8A8_SRGB),
    ];
    assert_eq!(
        choose_format(&offered, false),
        Some(PresentFormat::plain(
            format(vk::Format::R8G8B8A8_SRGB),
            true
        ))
    );
    // A UNORM format in another colour space is not the pass-through one.
    let wide = vk::SurfaceFormatKHR {
        format: vk::Format::B8G8R8A8_UNORM,
        color_space: vk::ColorSpaceKHR::DISPLAY_P3_NONLINEAR_EXT,
    };
    assert_eq!(
        choose_format(&[wide, format(vk::Format::B8G8R8A8_SRGB)], false),
        Some(PresentFormat::plain(
            format(vk::Format::B8G8R8A8_SRGB),
            true
        ))
    );
}

/// Scenario "Unknown formats".
#[test]
fn last_resort_prefers_a_format_in_the_srgb_non_linear_colour_space() {
    let wide = vk::SurfaceFormatKHR {
        format: vk::Format::R16G16B16A16_SFLOAT,
        color_space: vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT,
    };
    let ten_bit = format(vk::Format::A2B10G10R10_UNORM_PACK32);
    for mutable_format in [false, true] {
        assert_eq!(
            choose_format(&[wide, ten_bit], mutable_format),
            Some(PresentFormat::plain(ten_bit, false))
        );
        assert_eq!(
            choose_format(&[wide], mutable_format),
            Some(PresentFormat::plain(wide, false))
        );
        assert_eq!(choose_format(&[], mutable_format), None);
    }
}

#[test]
fn swapchain_extent_follows_the_surface_or_the_window() {
    let extent = |width, height| vk::Extent2D { width, height };
    let fixed = vk::SurfaceCapabilitiesKHR {
        current_extent: extent(800, 600),
        min_image_extent: extent(800, 600),
        max_image_extent: extent(800, 600),
        ..Default::default()
    };
    assert_eq!(
        swapchain_extent(&fixed, Size::new(640, 480)),
        extent(800, 600)
    );

    let free = vk::SurfaceCapabilitiesKHR {
        current_extent: extent(u32::MAX, u32::MAX),
        min_image_extent: extent(1, 1),
        max_image_extent: extent(4096, 4096),
        ..Default::default()
    };
    assert_eq!(
        swapchain_extent(&free, Size::new(640, 480)),
        extent(640, 480)
    );
    assert_eq!(
        swapchain_extent(&free, Size::new(8000, 480)),
        extent(4096, 480)
    );
    let minimised = vk::SurfaceCapabilitiesKHR {
        current_extent: extent(0, 0),
        ..Default::default()
    };
    assert_eq!(
        swapchain_extent(&minimised, Size::new(640, 480)),
        extent(0, 0)
    );
}
