//! One test per scenario of `specs/frame-composition/spec.md`, plus the bitmap-reference and
//! op-kind scenarios of `specs/guest-host-protocol/spec.md` that a backend has to honour.
//!
//! Expected values are written out by hand here, independently of the committed fixture images.

use vkxp_compose_ref::fixtures::{self, bitmap, sprite};
use vkxp_compose_ref::{BACKGROUND, BitmapError, ComposeError, Compositor, Frame, SlotState};
use vkxp_pixel::Rgba;
use vkxp_proto::codec::{decode_stream, encode_stream};
use vkxp_proto::{
    BitmapDelta, BitmapId, BlendMode, Color, FramePacket, FrameRate, GuestToHost, Op, Quad, Rect,
    RectEffect, Sampling, Size, Tone,
};

const SIZE: Size = Size::new(16, 12);
const SCREEN: Rect = Rect::from_size(SIZE);
const GREY: Rgba = [100, 110, 120, 255];
const RED: Rgba = [255, 0, 0, 255];
const BLUE: Rgba = [0, 0, 255, 255];

fn packet(bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> FramePacket {
    FramePacket {
        frame_index: 0,
        logical_size: SIZE,
        frame_rate: FrameRate::DEFAULT,
        tables: vec![],
        bitmaps,
        ops,
    }
}

fn render(bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> Frame {
    Compositor::new().render(&packet(bitmaps, ops)).unwrap()
}

fn clear(color: Rgba) -> Op {
    Op::Clear {
        clip: SCREEN,
        color: Color::from_rgba(color),
    }
}

/// Deltas for a `width` x `height` bitmap of one colour.
fn solid(id: BitmapId, width: u32, height: u32, color: Rgba) -> Vec<BitmapDelta> {
    let rgba = fixtures::solid(width, height, Color::from_rgba(color));
    bitmap(id, width, height, rgba).into()
}

/// Asserts every pixel of `frame` against `expected(x, y)`.
fn assert_pixels(frame: &Frame, expected: impl Fn(u32, u32) -> Rgba) {
    assert_eq!(frame.size, SIZE);
    for y in 0..SIZE.height {
        for x in 0..SIZE.width {
            assert_eq!(frame.pixel(x, y), expected(x, y), "pixel ({x}, {y})");
        }
    }
}

fn within(x: u32, y: u32, rect: Rect) -> bool {
    rect.contains(x as i32, y as i32)
}

const ID: BitmapId = BitmapId::new(0, 0);

// Requirement: Ordered per-pixel evaluation

#[test]
fn later_op_draws_over_earlier_op() {
    let (first, second) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let mut bitmaps = solid(first, 6, 6, RED);
    bitmaps.extend(solid(second, 6, 6, BLUE));
    let frame = render(
        bitmaps,
        vec![
            clear(GREY),
            Op::Quad(sprite(first, 6, 6, 2, 2)),
            Op::Quad(sprite(second, 6, 6, 5, 4)),
        ],
    );
    assert_pixels(&frame, |x, y| {
        if within(x, y, Rect::new(5, 4, 6, 6)) {
            BLUE
        } else if within(x, y, Rect::new(2, 2, 6, 6)) {
            RED
        } else {
            GREY
        }
    });
    // The overlap is the second quad's.
    assert_eq!(frame.pixel(6, 5), BLUE);
}

#[test]
fn clip_rectangle_limits_an_op() {
    let clip = Rect::new(4, 3, 5, 4);
    let quad = Quad {
        clip,
        ..sprite(ID, 16, 12, 0, 0)
    };
    let frame = render(solid(ID, 16, 12, RED), vec![clear(GREY), Op::Quad(quad)]);
    assert_pixels(&frame, |x, y| if within(x, y, clip) { RED } else { GREY });
}

#[test]
fn clip_rectangle_limits_clear_and_effect_too() {
    let clip = Rect::new(-3, 8, 8, 100);
    let frame = render(
        vec![],
        vec![
            clear(GREY),
            Op::Clear {
                clip,
                color: Color::from_rgba(RED),
            },
            Op::RectEffect(RectEffect {
                clip: Rect::new(12, 0, 4, 2),
                tone: Tone::new(0, 0, 0, 0),
                color: Color::from_rgba(BLUE),
            }),
        ],
    );
    assert_pixels(&frame, |x, y| {
        if within(x, y, clip) {
            RED
        } else if within(x, y, Rect::new(12, 0, 4, 2)) {
            BLUE
        } else {
            GREY
        }
    });
}

// Requirement: Clear op

#[test]
fn full_screen_clear() {
    let color = [12, 34, 56, 78];
    let frame = render(vec![], vec![clear(color)]);
    assert_pixels(&frame, |_, _| color);
}

#[test]
fn frame_without_ops_is_the_background() {
    let frame = render(vec![], vec![]);
    assert_pixels(&frame, |_, _| BACKGROUND);
}

// Requirement: Quad op

#[test]
fn zero_opacity_draws_nothing() {
    for blend in [BlendMode::Normal, BlendMode::Add, BlendMode::Subtract] {
        let quad = Quad {
            opacity: 0,
            blend,
            ..sprite(ID, 8, 8, 2, 2)
        };
        let frame = render(solid(ID, 8, 8, RED), vec![clear(GREY), Op::Quad(quad)]);
        assert_pixels(&frame, |_, _| GREY);
    }
}

#[test]
fn transparent_source_texel_draws_nothing() {
    // Left half of the bitmap is transparent but carries colour; right half is opaque.
    let mut rgba = Vec::new();
    for _ in 0..4 {
        rgba.extend([[200, 150, 100, 0]; 2].concat());
        rgba.extend([[200, 150, 100, 255]; 2].concat());
    }
    let expected_right = [
        (BlendMode::Normal, [200, 150, 100, 255]),
        (BlendMode::Add, [255, 255, 220, 255]),
        (BlendMode::Subtract, [0, 0, 20, 255]),
    ];
    for (blend, right) in expected_right {
        let quad = Quad {
            blend,
            ..sprite(ID, 4, 4, 6, 4)
        };
        let frame = render(
            bitmap(ID, 4, 4, rgba.clone()).into(),
            vec![clear(GREY), Op::Quad(quad)],
        );
        assert_pixels(&frame, |x, y| {
            if within(x, y, Rect::new(8, 4, 2, 4)) {
                right
            } else {
                GREY
            }
        });
    }
}

#[test]
fn additive_and_subtractive_blending_saturate() {
    let base = [200, 100, 30, 255];
    let source = [100, 100, 100, 255];
    let cases = [
        (BlendMode::Add, [255, 200, 130, 255]),
        (BlendMode::Subtract, [100, 0, 0, 255]),
    ];
    for (blend, expected) in cases {
        let quad = Quad {
            blend,
            ..sprite(ID, 16, 12, 0, 0)
        };
        let frame = render(solid(ID, 16, 12, source), vec![clear(base), Op::Quad(quad)]);
        assert_pixels(&frame, |_, _| expected);
    }
}

#[test]
fn identity_transform_copies_texels() {
    // Every texel is distinct, so any shift or flip would show.
    let (width, height) = (7u32, 5u32);
    let texel = |x: u32, y: u32| [x as u8 * 30 + 1, y as u8 * 40 + 2, (x + y) as u8, 255];
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend(texel(x, y));
        }
    }
    let frame = render(
        bitmap(ID, width, height, rgba).into(),
        vec![clear(GREY), Op::Quad(sprite(ID, width, height, 3, 2))],
    );
    assert_pixels(&frame, |x, y| {
        if within(x, y, Rect::new(3, 2, width, height)) {
            texel(x - 3, y - 2)
        } else {
            GREY
        }
    });
}

/// Every mode. The quads of these tests have an even width, so the window mode has no pixel
/// that takes the normal formula.
const BLENDS: [BlendMode; 4] = [
    BlendMode::Normal,
    BlendMode::Add,
    BlendMode::Subtract,
    BlendMode::Window,
];

fn blend(mode: BlendMode, dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
    match mode {
        BlendMode::Normal => vkxp_pixel::blend_normal(dst, src, opacity),
        BlendMode::Add => vkxp_pixel::blend_add(dst, src, opacity),
        BlendMode::Subtract => vkxp_pixel::blend_sub(dst, src, opacity),
        BlendMode::Window => vkxp_pixel::blend_window(dst, src, opacity),
    }
}

#[test]
fn neutral_tone_and_zero_alpha_colour_change_nothing() {
    // Column `v` holds channel value `v` in red, its complement in green and a scrambled value
    // in blue; the rows differ in alpha. Every channel takes all 256 values.
    const ALPHAS: [u8; 4] = [255, 128, 1, 0];
    let size = Size::new(256, 4);
    let texel = |x: u32, y: u32| {
        let v = x as u8;
        [v, 255 - v, v.wrapping_mul(7), ALPHAS[y as usize]]
    };
    let mut rgba = Vec::new();
    for y in 0..size.height {
        for x in 0..size.width {
            rgba.extend(texel(x, y));
        }
    }
    for mode in BLENDS {
        for opacity in [255, 100] {
            let quad = Quad {
                clip: Rect::from_size(size),
                tone: Tone::NEUTRAL,
                // Colour channels are set; only the zero alpha makes it neutral.
                color: Color::new(255, 128, 64, 0),
                opacity,
                blend: mode,
                ..sprite(ID, size.width, size.height, 0, 0)
            };
            let frame = Compositor::new()
                .render(&FramePacket {
                    logical_size: size,
                    ..packet(
                        bitmap(ID, size.width, size.height, rgba.clone()).into(),
                        vec![
                            Op::Clear {
                                clip: Rect::from_size(size),
                                color: Color::from_rgba(GREY),
                            },
                            Op::Quad(quad),
                        ],
                    )
                })
                .unwrap();
            for y in 0..size.height {
                for x in 0..size.width {
                    // The texel drawn as it is.
                    let expected = blend(mode, GREY, texel(x, y), opacity);
                    assert_eq!(frame.pixel(x, y), expected, "{mode:?} {opacity} ({x}, {y})");
                    if mode == BlendMode::Normal && opacity == 255 && y == 0 {
                        assert_eq!(expected, texel(x, y));
                    }
                }
            }
        }
    }
}

#[test]
fn full_alpha_colour_replaces_the_texels_colour() {
    // Four columns of different alpha, drawn over transparent black so the drawn texel can be
    // read back, alpha included. Where nothing is underneath and nothing is drawn (alpha 0),
    // the pixel still takes the colour.
    let alphas = [255u8, 180, 3, 0];
    let mut rgba = Vec::new();
    for y in 0..4u8 {
        for (x, alpha) in alphas.iter().enumerate() {
            rgba.extend([x as u8 * 60, 255 - y * 50, 77, *alpha]);
        }
    }
    let quad = Quad {
        tone: Tone::NEUTRAL,
        color: Color::new(10, 200, 30, 255),
        ..sprite(ID, 4, 4, 5, 3)
    };
    let frame = render(
        bitmap(ID, 4, 4, rgba).into(),
        vec![clear([0, 0, 0, 0]), Op::Quad(quad)],
    );
    assert_pixels(&frame, |x, y| {
        if within(x, y, Rect::new(5, 3, 4, 4)) {
            [10, 200, 30, alphas[(x - 5) as usize]]
        } else {
            [0, 0, 0, 0]
        }
    });
}

#[test]
fn quad_applies_tone_before_colour() {
    // Tone first: red 255 - 100 = 155, green 0 + 50 = 50; then half-alpha red over that gives
    // red 155 + ((100 * 129) >> 8) = 205 and green 50 + ((-50 * 129) >> 8) = 24. The other
    // order would give red 255 - 100 = 155.
    let (tone, color) = (Tone::new(-100, 50, 0, 0), Color::new(255, 0, 0, 128));
    let toned = vkxp_pixel::apply_tone(RED, tone.red, tone.green, tone.blue, tone.gray);
    assert_eq!(toned, [155, 50, 0, 255]);
    assert_eq!(
        vkxp_pixel::blend_color(toned, color.to_rgba()),
        [205, 24, 0, 255]
    );

    let quad = Quad {
        tone,
        color,
        ..sprite(ID, 16, 12, 0, 0)
    };
    let frame = render(solid(ID, 16, 12, RED), vec![clear(GREY), Op::Quad(quad)]);
    assert_pixels(&frame, |_, _| [205, 24, 0, 255]);

    // The drawn texel keeps the sampled alpha: at alpha 128 it is blended half over the grey.
    let translucent = [255, 0, 0, 128];
    let frame = render(
        solid(ID, 16, 12, translucent),
        vec![clear(GREY), Op::Quad(quad)],
    );
    let expected = vkxp_pixel::blend_normal(GREY, [205, 24, 0, 128], 255);
    assert_eq!(expected, [152, 66, 59, 255]);
    assert_pixels(&frame, |_, _| expected);
}

#[test]
fn quad_tone_and_colour_do_not_reach_the_pixels_underneath() {
    let (tone, color) = (Tone::new(255, -255, 90, 200), Color::new(0, 255, 0, 255));
    for mode in BLENDS {
        // Opacity 0: nothing is drawn, so nothing is recoloured.
        let hidden = Quad {
            tone,
            color,
            opacity: 0,
            blend: mode,
            ..sprite(ID, 16, 12, 0, 0)
        };
        let frame = render(solid(ID, 16, 12, RED), vec![clear(GREY), Op::Quad(hidden)]);
        assert_pixels(&frame, |_, _| GREY);

        // The clip is the whole screen but the source rectangle covers only 4x4 pixels.
        let covered = Rect::new(6, 4, 4, 4);
        let small = Quad {
            tone,
            color,
            blend: mode,
            ..sprite(ID, 4, 4, 6, 4)
        };
        let frame = render(solid(ID, 4, 4, RED), vec![clear(GREY), Op::Quad(small)]);
        let drawn = blend(mode, GREY, [0, 255, 0, 255], 255);
        assert_ne!(drawn, GREY);
        assert_pixels(
            &frame,
            |x, y| if within(x, y, covered) { drawn } else { GREY },
        );
    }
}

#[test]
fn transparent_source_texel_draws_nothing_whatever_tone_and_colour() {
    let cases = [
        (
            Tone::new(255, 255, 255, 255),
            Color::new(255, 255, 255, 255),
        ),
        (Tone::new(-255, 40, 0, 128), Color::new(9, 8, 7, 100)),
        (Tone::NEUTRAL, Color::new(255, 0, 255, 255)),
    ];
    for mode in BLENDS {
        for (tone, color) in cases {
            let quad = Quad {
                tone,
                color,
                blend: mode,
                ..sprite(ID, 16, 12, 0, 0)
            };
            let frame = render(
                solid(ID, 16, 12, [200, 150, 100, 0]),
                vec![clear(GREY), Op::Quad(quad)],
            );
            assert_pixels(&frame, |_, _| GREY);
        }
    }
}

/// Clears the screen to `base`, then draws a full-screen normal-mode quad of `source` at
/// `opacity`, and returns the one colour the whole frame has.
fn normal_over(base: Rgba, source: Rgba, opacity: u8) -> Rgba {
    let quad = Quad {
        opacity,
        ..sprite(ID, 16, 12, 0, 0)
    };
    let frame = render(solid(ID, 16, 12, source), vec![clear(base), Op::Quad(quad)]);
    let pixel = frame.pixel(0, 0);
    assert_pixels(&frame, |_, _| pixel);
    pixel
}

const TRANSPARENT: Rgba = [0, 0, 0, 0];
const TRANSLUCENT_BLUE: Rgba = [0, 50, 255, 128];

#[test]
fn opaque_quad_copies_texels_over_a_transparent_clear() {
    // The identity scenario must hold whatever is underneath, alpha included.
    assert_eq!(
        normal_over(TRANSPARENT, [255, 0, 0, 255], 255),
        [255, 0, 0, 255]
    );
    assert_eq!(
        normal_over([9, 8, 7, 0], [1, 2, 3, 255], 255),
        [1, 2, 3, 255]
    );
}

#[test]
fn opaque_quad_copies_texels_over_a_partially_transparent_clear() {
    assert_eq!(
        normal_over(TRANSLUCENT_BLUE, [255, 0, 0, 255], 255),
        [255, 0, 0, 255]
    );
    assert_eq!(
        normal_over([200, 200, 200, 1], [1, 2, 3, 255], 255),
        [1, 2, 3, 255]
    );
}

#[test]
fn identity_transform_copies_texels_over_a_transparent_clear() {
    // The distinct-texel bitmap of `identity_transform_copies_texels`, on a transparent screen.
    let (width, height) = (7u32, 5u32);
    let texel = |x: u32, y: u32| [x as u8 * 30 + 1, y as u8 * 40 + 2, (x + y) as u8, 255];
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend(texel(x, y));
        }
    }
    let frame = render(
        bitmap(ID, width, height, rgba).into(),
        vec![
            clear(TRANSPARENT),
            Op::Quad(sprite(ID, width, height, 3, 2)),
        ],
    );
    assert_pixels(&frame, |x, y| {
        if within(x, y, Rect::new(3, 2, width, height)) {
            texel(x - 3, y - 2)
        } else {
            TRANSPARENT
        }
    });
}

#[test]
fn translucent_quad_over_a_transparent_clear_keeps_its_colour_and_drawn_alpha() {
    // Nothing is underneath, so the colour is the source's and the alpha is what was drawn.
    assert_eq!(
        normal_over(TRANSPARENT, [255, 0, 0, 128], 255),
        [255, 0, 0, 128]
    );
    assert_eq!(
        normal_over(TRANSPARENT, [200, 100, 7, 255], 128),
        [200, 100, 7, 128]
    );
    // Texel alpha 128 at opacity 128 draws with alpha div255(128 * 128) = 64.
    assert_eq!(
        normal_over(TRANSPARENT, [200, 100, 7, 128], 128),
        [200, 100, 7, 64]
    );
}

#[test]
fn translucent_quad_over_a_partially_transparent_clear_follows_the_measured_formula() {
    // Drawn alpha a = 128 over destination alpha 128:
    //   den   = 255 * 128 + 255 * 128 - 128 * 128 = 48896, alpha = den / 255 = 191
    //   w     = 128 * 65025 / den = 170
    //   red   = 0 + ((200 * 171) >> 8) = 133
    //   green = 50 + ((50 * 171) >> 8) = 83
    //   blue  = 255 + ((-255 * 171) >> 8) = 84
    let expected = [133, 83, 84, 191];
    assert_eq!(
        normal_over(TRANSLUCENT_BLUE, [200, 100, 0, 128], 255),
        expected
    );
    assert_eq!(
        normal_over(TRANSLUCENT_BLUE, [200, 100, 0, 255], 128),
        expected
    );
}

#[test]
fn nothing_drawn_leaves_transparent_and_translucent_clears_unchanged() {
    for base in [TRANSPARENT, [9, 8, 7, 0], TRANSLUCENT_BLUE, GREY] {
        for blend in [BlendMode::Normal, BlendMode::Add, BlendMode::Subtract] {
            // Zero opacity, then a transparent texel that still carries colour.
            for (source, opacity) in [([255, 128, 7, 255], 0), ([255, 128, 7, 0], 255)] {
                let quad = Quad {
                    opacity,
                    blend,
                    ..sprite(ID, 16, 12, 0, 0)
                };
                let frame = render(solid(ID, 16, 12, source), vec![clear(base), Op::Quad(quad)]);
                // The one exception, as measured: a normal blend at an opacity above 0 onto a
                // fully transparent pixel leaves the texel's colour there, with alpha 0.
                let expected = if blend == BlendMode::Normal && base[3] == 0 && opacity != 0 {
                    [source[0], source[1], source[2], 0]
                } else {
                    base
                };
                assert_pixels(&frame, |_, _| expected);
            }
        }
    }
}

#[test]
fn quad_draws_only_its_source_rectangle() {
    // A 4x4 bitmap split into four 2x2 colour blocks; only the bottom-right block is drawn.
    let green = [0, 255, 0, 255];
    let white = [255, 255, 255, 255];
    let block = |x: u32, y: u32| match (x / 2, y / 2) {
        (0, 0) => RED,
        (1, 0) => green,
        (0, 1) => BLUE,
        _ => white,
    };
    let mut rgba = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            rgba.extend(block(x, y));
        }
    }
    let quad = Quad {
        src: Rect::new(2, 2, 2, 2),
        ..sprite(ID, 4, 4, 5, 5)
    };
    let frame = render(
        bitmap(ID, 4, 4, rgba).into(),
        vec![clear(GREY), Op::Quad(quad)],
    );
    assert_pixels(&frame, |x, y| {
        if within(x, y, Rect::new(5, 5, 2, 2)) {
            white
        } else {
            GREY
        }
    });
}

#[test]
fn quad_partly_off_screen_draws_its_visible_part() {
    let frame = render(
        solid(ID, 8, 8, RED),
        vec![clear(GREY), Op::Quad(sprite(ID, 8, 8, -5, 9))],
    );
    assert_pixels(&frame, |x, y| if x < 3 && y >= 9 { RED } else { GREY });
}

// Requirement: Rectangle effect op

#[test]
fn neutral_effect_is_a_no_op() {
    let ops = |effect: Option<Op>| {
        let mut ops = vec![clear(GREY), Op::Quad(sprite(ID, 6, 6, 1, 1))];
        ops.extend(effect);
        ops
    };
    let neutral = Op::RectEffect(RectEffect {
        clip: SCREEN,
        tone: Tone::NEUTRAL,
        // Colour channels are set; only the zero alpha makes it neutral.
        color: Color::new(255, 128, 64, 0),
    });
    let rgba = fixtures::gradient(6, 6);
    let without = render(bitmap(ID, 6, 6, rgba.clone()).into(), ops(None));
    let with = render(bitmap(ID, 6, 6, rgba).into(), ops(Some(neutral)));
    assert_eq!(with, without);
}

#[test]
fn effect_applies_to_what_is_underneath() {
    let rect = Rect::new(4, 2, 8, 6);
    let effect = Op::RectEffect(RectEffect {
        clip: rect,
        tone: Tone::new(50, -200, 0, 0),
        color: Color::new(0, 0, 0, 0),
    });
    let frame = render(
        solid(ID, 4, 12, RED),
        vec![clear(GREY), Op::Quad(sprite(ID, 4, 12, 6, 0)), effect],
    );
    assert_pixels(&frame, |x, y| {
        let underneath = if within(x, y, Rect::new(6, 0, 4, 12)) {
            RED
        } else {
            GREY
        };
        match (within(x, y, rect), underneath) {
            (false, _) => underneath,
            (true, RED) => [255, 0, 0, 255],
            (true, _) => [150, 0, 120, 255],
        }
    });
}

#[test]
fn effect_applies_tone_before_colour() {
    // Full grey turns pure red into (76, 76, 76); half-alpha white then lifts it to 166.
    let effect = Op::RectEffect(RectEffect {
        clip: SCREEN,
        tone: Tone::new(0, 0, 0, 255),
        color: Color::new(255, 255, 255, 128),
    });
    let frame = render(vec![], vec![clear(RED), effect]);
    assert_pixels(&frame, |_, _| [166, 166, 166, 255]);
}

// Requirement: Backends agree bit for bit

#[test]
fn deterministic_reference_output() {
    for fixture in fixtures::all() {
        let first = vkxp_compose_ref::render_stream(&fixture.messages).unwrap();
        let second = vkxp_compose_ref::render_stream(&fixture.messages).unwrap();
        assert!(first.is_some(), "{} produces a frame", fixture.name);
        assert_eq!(first, second, "{}", fixture.name);
    }
}

// Requirement: Arithmetic provenance

#[test]
fn listing_tells_measured_functions_from_placeholders() {
    let listing = vkxp_pixel::provenance();
    let entry = |name: &str| {
        listing
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("`{name}` is not in the provenance listing"))
    };
    for used in [
        "blend_normal",
        "blend_normal_translucent",
        "blend_add",
        "blend_sub",
        "blend_color",
        "apply_gray",
        "apply_tone",
    ] {
        let provenance = entry(used).provenance;
        assert!(!provenance.is_placeholder(), "{used}");
        assert!(
            provenance.to_string().contains("oracle measurement"),
            "{used}"
        );
    }
    for unmeasured in ["blend_add_translucent", "blend_sub_translucent"] {
        assert!(
            entry(unmeasured).provenance.is_placeholder(),
            "{unmeasured}"
        );
    }
}

// guest-host-protocol, Requirement: Frame packet content

#[test]
fn bitmap_is_defined_before_use() {
    let mut compositor = Compositor::new();
    // Created in an earlier packet, used in a later one.
    compositor
        .render(&packet(solid(ID, 2, 2, RED), vec![]))
        .unwrap();
    let frame = compositor
        .render(&packet(
            vec![],
            vec![clear(GREY), Op::Quad(sprite(ID, 2, 2, 0, 0))],
        ))
        .unwrap();
    assert_eq!(frame.pixel(1, 1), RED);
    assert_eq!(frame.pixel(2, 2), GREY);

    // Created and used in the same packet.
    let other = BitmapId::new(1, 0);
    let frame = compositor
        .render(&packet(
            solid(other, 2, 2, BLUE),
            vec![Op::Quad(sprite(other, 2, 2, 4, 4))],
        ))
        .unwrap();
    assert_eq!(frame.pixel(5, 5), BLUE);
}

#[test]
fn stale_bitmap_reference_is_detectable() {
    let old = BitmapId::new(3, 0);
    let new = BitmapId::new(3, 1);
    let mut compositor = Compositor::new();
    compositor
        .render(&packet(solid(old, 2, 2, RED), vec![]))
        .unwrap();

    let mut deltas = vec![BitmapDelta::Destroy { id: old }];
    deltas.extend(solid(new, 2, 2, BLUE));
    // The op still names the destroyed bitmap, whose slot now holds another one.
    let result = compositor.render(&packet(deltas, vec![Op::Quad(sprite(old, 2, 2, 0, 0))]));
    assert_eq!(
        result,
        Err(ComposeError::InvalidOpBitmap {
            index: 0,
            source: BitmapError::InvalidReference {
                id: old,
                slot: SlotState::Occupied { generation: 1 },
            },
        })
    );

    // The new occupant is reachable through its own id.
    let frame = compositor
        .render(&packet(vec![], vec![Op::Quad(sprite(new, 2, 2, 0, 0))]))
        .unwrap();
    assert_eq!(frame.pixel(0, 0), BLUE);
}

#[test]
fn destroyed_bitmap_without_reuse_is_an_invalid_reference() {
    let mut compositor = Compositor::new();
    compositor
        .render(&packet(solid(ID, 2, 2, RED), vec![]))
        .unwrap();
    let result = compositor.render(&packet(
        vec![BitmapDelta::Destroy { id: ID }],
        vec![clear(GREY), Op::Quad(sprite(ID, 2, 2, 0, 0))],
    ));
    assert_eq!(
        result,
        Err(ComposeError::InvalidOpBitmap {
            index: 1,
            source: BitmapError::InvalidReference {
                id: ID,
                slot: SlotState::Empty,
            },
        })
    );
}

// guest-host-protocol, Requirement: Every op kind is implemented

#[test]
fn one_op_of_each_kind_round_trips_and_is_accepted() {
    let messages = [fixtures::one_op_of_each_kind()];
    let bytes = encode_stream(&messages).unwrap();
    let decoded = decode_stream::<GuestToHost>(&bytes).unwrap();
    assert_eq!(decoded, messages);
    vkxp_compose_ref::render_stream(&decoded).unwrap();
}

// Rejected input

#[test]
fn source_rectangle_outside_the_bitmap_is_rejected() {
    let quad = Quad {
        src: Rect::new(1, 0, 4, 4),
        ..sprite(ID, 4, 4, 0, 0)
    };
    let result = Compositor::new().render(&packet(solid(ID, 4, 4, RED), vec![Op::Quad(quad)]));
    assert_eq!(
        result,
        Err(ComposeError::SourceOutOfBounds {
            index: 0,
            bitmap: ID,
            src: Rect::new(1, 0, 4, 4),
            width: 4,
            height: 4,
        })
    );
}

#[test]
fn invalid_delta_is_reported_with_its_position() {
    let mut deltas = solid(ID, 2, 2, RED);
    deltas.push(BitmapDelta::Create {
        id: ID,
        width: 1,
        height: 1,
    });
    let result = Compositor::new().render(&packet(deltas, vec![]));
    assert_eq!(
        result,
        Err(ComposeError::InvalidDelta {
            index: 2,
            source: BitmapError::SlotOccupied {
                id: ID,
                occupant: 0
            },
        })
    );
}

#[test]
fn empty_or_oversized_logical_size_is_rejected() {
    for size in [Size::new(0, 10), Size::new(10, 0), Size::new(16385, 1)] {
        let result = Compositor::new().render(&FramePacket {
            frame_index: 0,
            logical_size: size,
            frame_rate: FrameRate::DEFAULT,
            tables: vec![],
            bitmaps: vec![],
            ops: vec![],
        });
        assert_eq!(
            result,
            Err(ComposeError::InvalidLogicalSize { size, limit: 16384 })
        );
    }
}

// Requirement: Stretch sampling of quads

/// A bitmap one row high whose texel `x` is [`column`]`(x)`.
fn row_bitmap(id: BitmapId, width: u32) -> Vec<BitmapDelta> {
    let rgba = (0..width).flat_map(column).collect();
    bitmap(id, width, 1, rgba).into()
}

fn column(x: u32) -> Rgba {
    [(x % 256) as u8, (x / 256) as u8, 77, 255]
}

fn stretch(clip: Rect, src: Rect, dest: Rect, mirror: bool) -> Op {
    Op::Quad(Quad {
        clip,
        bitmap: ID,
        src,
        sampling: Sampling::Stretch { dest, mirror },
        tone: Tone::NEUTRAL,
        color: Color::new(0, 0, 0, 0),
        opacity: 255,
        blend: BlendMode::Normal,
    })
}

/// Renders `ops` over a grey screen of `size`.
fn render_sized(size: Size, bitmaps: Vec<BitmapDelta>, mut ops: Vec<Op>) -> Frame {
    ops.insert(
        0,
        Op::Clear {
            clip: Rect::from_size(size),
            color: Color::from_rgba(GREY),
        },
    );
    let packet = FramePacket {
        logical_size: size,
        ..packet(bitmaps, ops)
    };
    Compositor::new().render(&packet).unwrap()
}

/// One line of `tests/data/sprite-zoom-samples.txt`: where the original runtime drew a zoomed
/// sprite 16 pixels wide, and the source column it showed at each pixel.
struct ZoomSample {
    mirror: bool,
    first: i32,
    columns: Vec<u32>,
}

fn zoom_samples() -> Vec<ZoomSample> {
    let number = |text: &str| text.parse::<i32>().unwrap();
    include_str!("data/sprite-zoom-samples.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            let mirror = parts[0].split_whitespace().nth(3) == Some("1");
            let placed: Vec<i32> = parts[1]
                .split_whitespace()
                .map_while(|w| w.parse().ok())
                .collect();
            let &[first, width] = placed.as_slice() else {
                // "none": the sprite drew nothing, which only the extraction can show.
                return None;
            };
            let columns: Vec<u32> = parts[2]
                .split_whitespace()
                .map(|w| number(w) as u32)
                .collect();
            assert_eq!(columns.len() as i32, width, "{line}");
            Some(ZoomSample {
                mirror,
                first,
                columns,
            })
        })
        .collect()
}

#[test]
fn stretch_quads_show_the_measured_texels() {
    let samples = zoom_samples();
    assert!(samples.len() > 60);
    let size = Size::new(640, 1);
    for sample in samples {
        let width = sample.columns.len() as u32;
        let dest = Rect::new(sample.first, 0, width, 1);
        let op = stretch(
            Rect::from_size(size),
            Rect::new(0, 0, 16, 1),
            dest,
            sample.mirror,
        );
        let frame = render_sized(size, row_bitmap(ID, 16), vec![op]);
        for x in 0..size.width {
            let expected = match (x as i32).checked_sub(sample.first) {
                Some(i) if (0..width as i32).contains(&i) => column(sample.columns[i as usize]),
                _ => GREY,
            };
            assert_eq!(frame.pixel(x, 0), expected, "{width} wide, pixel {x}");
        }
    }
}

#[test]
fn clipped_stretch_quad_counts_from_its_unclipped_corner() {
    // 300 texels into 450 pixels, of which the first 100 lie left of the clip.
    let size = Size::new(40, 3);
    let clip = Rect::new(4, 1, 20, 1);
    let dest = Rect::new(4 - 100, 1 - 2, 450, 3);
    let op = stretch(clip, Rect::new(0, 0, 300, 1), dest, false);
    let frame = render_sized(size, row_bitmap(ID, 300), vec![op]);
    let step = (300u64 << 20) / 450;
    assert_eq!(frame.pixel(4, 1), column(((100 * step) >> 20) as u32));
    assert_ne!(frame.pixel(4, 1), column(0));
    for y in 0..size.height {
        for x in 0..size.width {
            let expected = if clip.contains(x as i32, y as i32) {
                column(((u64::from(x + 96) * step) >> 20) as u32)
            } else {
                GREY
            };
            assert_eq!(frame.pixel(x, y), expected, "pixel ({x}, {y})");
        }
    }
}

#[test]
fn unscaled_stretch_quad_copies_texels() {
    let (width, height) = (7u32, 5u32);
    let texel = |x: u32, y: u32| [x as u8 * 30 + 1, y as u8 * 40 + 2, (x + y) as u8, 255];
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend(texel(x, y));
        }
    }
    let src = Rect::new(0, 0, width, height);
    for mirror in [false, true] {
        let frame = render(
            bitmap(ID, width, height, rgba.clone()).into(),
            vec![
                clear(GREY),
                stretch(SCREEN, src, Rect::new(3, 2, width, height), mirror),
            ],
        );
        assert_pixels(&frame, |x, y| {
            if !within(x, y, Rect::new(3, 2, width, height)) {
                GREY
            } else if mirror {
                texel(width - 1 - (x - 3), y - 2)
            } else {
                texel(x - 3, y - 2)
            }
        });
    }
}

#[test]
fn stretch_quad_takes_its_texels_from_the_source_rectangle() {
    // The middle 4 by 2 texels of an 8 by 4 bitmap, doubled.
    let texel = |x: u32, y: u32| [x as u8 + 1, y as u8 + 1, 9, 255];
    let rgba = (0..4)
        .flat_map(|y| (0..8).flat_map(move |x| texel(x, y)))
        .collect();
    let op = stretch(SCREEN, Rect::new(2, 1, 4, 2), Rect::new(1, 1, 8, 4), false);
    let frame = render(bitmap(ID, 8, 4, rgba).into(), vec![clear(GREY), op]);
    assert_pixels(&frame, |x, y| {
        if within(x, y, Rect::new(1, 1, 8, 4)) {
            texel(2 + (x - 1) / 2, 1 + (y - 1) / 2)
        } else {
            GREY
        }
    });
}

#[test]
fn stretch_quad_with_an_empty_destination_draws_nothing() {
    for dest in [
        Rect::new(2, 2, 0, 6),
        Rect::new(2, 2, 6, 0),
        Rect::new(2, 2, 0, 0),
    ] {
        let op = stretch(SCREEN, Rect::new(0, 0, 4, 4), dest, false);
        let frame = render(solid(ID, 4, 4, RED), vec![clear(GREY), op]);
        assert_pixels(&frame, |_, _| GREY);
    }
}

#[test]
fn stretch_quad_far_larger_than_the_screen_shows_its_part_on_screen() {
    // 16 texels into the widest rectangle there is, the screen somewhere in its middle.
    let size = Size::new(40, 2);
    let dest = Rect::new(i32::MIN, i32::MIN, u32::MAX, u32::MAX);
    let op = stretch(Rect::from_size(size), Rect::new(0, 0, 16, 1), dest, false);
    let frame = render_sized(size, row_bitmap(ID, 16), vec![op]);
    let step = (16u64 << 20) / u64::from(u32::MAX);
    for x in 0..size.width {
        let i = u64::from(x) + (1 << 31);
        assert_eq!(frame.pixel(x, 0), column(((i * step) >> 20) as u32));
    }
}

// Requirement: Window blend arithmetic of the original

/// A window-mode quad showing a `width` by 1 bitmap of grey texels (`source`, alpha `alpha`)
/// with its left edge at `x`, over a grey `destination`, on a frame of `size`.
fn window_row(size: Size, clip: Rect, x: i16, width: u32, [d, s, a]: [u8; 3]) -> Frame {
    let rgba = fixtures::solid(width, 1, Color::new(s, s, s, a));
    let quad = Op::Quad(Quad {
        clip,
        blend: BlendMode::Window,
        ..sprite(ID, width, 1, x, 0)
    });
    let packet = FramePacket {
        logical_size: size,
        ..packet(
            bitmap(ID, width, 1, rgba).into(),
            vec![
                Op::Clear {
                    clip: Rect::from_size(size),
                    color: Color::new(d, d, d, 255),
                },
                quad,
            ],
        )
    };
    Compositor::new().render(&packet).unwrap()
}

/// The samples of one kind of `tests/data/window-blend-samples.txt`, as their numbers.
fn window_blend_samples(kind: &str) -> Vec<Vec<u8>> {
    include_str!("data/window-blend-samples.txt")
        .lines()
        .filter_map(|line| line.strip_prefix(kind)?.strip_prefix(' '))
        .map(|row| {
            row.split_whitespace()
                .filter(|word| *word != "|")
                .map(|word| word.parse().unwrap())
                .collect()
        })
        .collect()
}

#[test]
fn window_blend_reproduces_the_measured_samples() {
    let size = Size::new(64, 1);
    let screen = Rect::from_size(size);
    let samples = window_blend_samples("blend");
    assert!(samples.len() >= 30);
    for sample in samples {
        let &[d, s, a, out] = sample.as_slice() else {
            panic!("{sample:?}");
        };
        let frame = window_row(size, screen, 10, 2, [d, s, a]);
        for x in [10, 11] {
            assert_eq!(frame.pixel(x, 0), [out, out, out, 255], "{sample:?}");
        }
        assert_eq!(frame.pixel(12, 0), [d, d, d, 255]);
    }
    // The last pixel of a row of 33: the window of the probe, at x = 10.
    let samples = window_blend_samples("tail");
    assert!(samples.len() >= 8);
    for sample in samples {
        let &[x, d, s, a, out] = sample.as_slice() else {
            panic!("{sample:?}");
        };
        let frame = window_row(size, screen, 10, 33, [d, s, a]);
        assert_eq!(
            frame.pixel(u32::from(x), 0),
            [out, out, out, 255],
            "{sample:?}"
        );
    }
}

#[test]
fn window_blend_uses_the_normal_formula_for_the_last_pixel_of_an_odd_row() {
    // Destination 161, source 0, alpha 4: the window formula gives 158, the normal one 157.
    let size = Size::new(16, 1);
    let screen = Rect::from_size(size);
    let (window, normal) = ([158, 158, 158, 255], [157, 157, 157, 255]);
    let untouched = [161, 161, 161, 255];
    let expect = |frame: &Frame, tail: Option<u32>, covered: std::ops::Range<u32>| {
        for x in 0..size.width {
            let expected = if Some(x) == tail {
                normal
            } else if covered.contains(&x) {
                window
            } else {
                untouched
            };
            assert_eq!(frame.pixel(x, 0), expected, "pixel {x}");
        }
    };
    // Five pixels wide: four by the window formula, the fifth by the normal one.
    expect(&window_row(size, screen, 3, 5, [161, 0, 4]), Some(7), 3..8);
    // Six wide: no such pixel, until the clip or the frame leaves an odd part of it.
    expect(&window_row(size, screen, 3, 6, [161, 0, 4]), None, 3..9);
    let clip = Rect::new(0, 0, 8, 1);
    expect(&window_row(size, clip, 3, 6, [161, 0, 4]), Some(7), 3..8);
    let clip = Rect::new(4, 0, 12, 1);
    expect(&window_row(size, clip, 3, 6, [161, 0, 4]), Some(8), 4..9);
    expect(
        &window_row(size, screen, 13, 6, [161, 0, 4]),
        Some(15),
        13..16,
    );
    expect(&window_row(size, screen, -3, 6, [161, 0, 4]), Some(2), 0..3);
}

#[test]
fn window_blend_of_a_scaled_quad_has_no_exception() {
    let size = Size::new(16, 1);
    let rgba = fixtures::solid(5, 1, Color::new(0, 0, 0, 4));
    let quad = Op::Quad(Quad {
        clip: Rect::from_size(size),
        bitmap: ID,
        src: Rect::new(0, 0, 5, 1),
        sampling: Sampling::Stretch {
            dest: Rect::new(3, 0, 5, 1),
            mirror: false,
        },
        tone: Tone::NEUTRAL,
        color: Color::new(0, 0, 0, 0),
        opacity: 255,
        blend: BlendMode::Window,
    });
    let clear = Op::Clear {
        clip: Rect::from_size(size),
        color: Color::new(161, 161, 161, 255),
    };
    let packet = FramePacket {
        logical_size: size,
        ..packet(bitmap(ID, 5, 1, rgba).into(), vec![clear, quad])
    };
    let frame = Compositor::new().render(&packet).unwrap();
    for x in 3..8 {
        assert_eq!(frame.pixel(x, 0), [158, 158, 158, 255], "pixel {x}");
    }
}
