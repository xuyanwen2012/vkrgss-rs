//! Generated message streams with committed expected frames.
//!
//! Every fixture is built in code from procedural patterns, so no image asset is needed to
//! test a backend. The expected frame of each fixture is committed as
//! `tests/expected/<name>.png` in this crate; any backend fed the same messages must produce
//! exactly those pixels.

use vkxp_proto::{
    BitmapDelta, BitmapId, BlendMode, Color, Fixed, FramePacket, FrameRate, GuestToHost, Op, Quad,
    Rect, RectEffect, Sampling, Size, Tone, Transform,
};

/// Logical screen size of every fixture.
pub const SIZE: Size = Size::new(64, 48);

/// A named message stream. Its result is the last frame the stream produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    pub name: &'static str,
    pub messages: Vec<GuestToHost>,
}

/// All fixtures, in a fixed order.
pub fn all() -> Vec<Fixture> {
    vec![
        checkerboard_fixture(),
        gradient_fixture(),
        alpha_shape_fixture(),
        overlap_fixture(),
        clip_fixture(),
        blend_fixture("blend_normal", BlendMode::Normal),
        blend_fixture("blend_add", BlendMode::Add),
        blend_fixture("blend_subtract", BlendMode::Subtract),
        rect_effect_fixture(),
        rotation_fixture(),
        persistent_bitmap_fixture(),
        quad_tone_fixture("quad_tone_normal", BlendMode::Normal),
        quad_tone_fixture("quad_tone_add", BlendMode::Add),
        quad_tone_fixture("quad_tone_subtract", BlendMode::Subtract),
        quad_color_fixture("quad_color_normal", BlendMode::Normal),
        quad_color_fixture("quad_color_add", BlendMode::Add),
        quad_color_fixture("quad_color_subtract", BlendMode::Subtract),
    ]
}

/// The fixture called `name`, if there is one.
pub fn by_name(name: &str) -> Option<Fixture> {
    all().into_iter().find(|fixture| fixture.name == name)
}

// Patterns. Each returns `width * height` RGBA pixels in row order.

/// Squares of `cell` pixels alternating between two colours, `first` at the top-left.
pub fn checkerboard(width: u32, height: u32, cell: u32, first: Color, second: Color) -> Vec<u8> {
    pattern(width, height, |x, y| {
        if (x / cell + y / cell).is_multiple_of(2) {
            first
        } else {
            second
        }
    })
}

/// Opaque gradient: red rises left to right, green rises top to bottom, blue is constant.
pub fn gradient(width: u32, height: u32) -> Vec<u8> {
    let ramp = |position: u32, length: u32| (position * 255 / (length - 1).max(1)) as u8;
    pattern(width, height, |x, y| {
        Color::new(ramp(x, width), ramp(y, height), 128, 255)
    })
}

/// A disc of one colour whose alpha is 255 in the middle, falls off linearly (in squared
/// distance) over the outer part and is 0 in the corners.
pub fn disc(size: u32, color: Color) -> Vec<u8> {
    // Squared distances from the centre, in half-pixel units.
    let outer = i64::from(size) * i64::from(size);
    let inner = outer / 2;
    pattern(size, size, |x, y| {
        let dx = 2 * i64::from(x) + 1 - i64::from(size);
        let dy = 2 * i64::from(y) + 1 - i64::from(size);
        let distance = dx * dx + dy * dy;
        let alpha = if distance <= inner {
            255
        } else if distance >= outer {
            0
        } else {
            (255 * (outer - distance) / (outer - inner)) as u8
        };
        Color::new(color.r, color.g, color.b, alpha)
    })
}

/// One colour everywhere.
pub fn solid(width: u32, height: u32, color: Color) -> Vec<u8> {
    pattern(width, height, |_, _| color)
}

fn pattern(width: u32, height: u32, pixel: impl Fn(u32, u32) -> Color) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&pixel(x, y).to_rgba());
        }
    }
    rgba
}

// Building blocks.

const SCREEN: Rect = Rect::from_size(SIZE);
const DARK: Color = Color::new(32, 32, 48, 255);
const LIGHT: Color = Color::new(224, 224, 208, 255);

/// The deltas that create a bitmap and fill it with `rgba`.
pub fn bitmap(id: BitmapId, width: u32, height: u32, rgba: Vec<u8>) -> [BitmapDelta; 2] {
    [
        BitmapDelta::Create { id, width, height },
        BitmapDelta::Update {
            id,
            rect: Rect::new(0, 0, width, height),
            rgba,
        },
    ]
}

/// An unscaled, fully opaque, normal-mode quad of a whole bitmap at (`x`, `y`), clipped to the
/// screen, with a neutral tone and a colour of alpha 0.
pub fn sprite(id: BitmapId, width: u32, height: u32, x: i16, y: i16) -> Quad {
    Quad {
        clip: SCREEN,
        bitmap: id,
        src: Rect::new(0, 0, width, height),
        sampling: Sampling::Affine(Transform::place_at(x, y)),
        tone: Tone::NEUTRAL,
        color: Color::new(0, 0, 0, 0),
        opacity: 255,
        blend: BlendMode::Normal,
    }
}

fn clear(color: Color) -> Op {
    Op::Clear {
        clip: SCREEN,
        color,
    }
}

fn frame(frame_index: u64, bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> GuestToHost {
    GuestToHost::Frame(FramePacket {
        frame_index,
        logical_size: SIZE,
        frame_rate: FrameRate::DEFAULT,
        bitmaps,
        ops,
    })
}

fn single_frame(name: &'static str, bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> Fixture {
    Fixture {
        name,
        messages: vec![frame(0, bitmaps, ops)],
    }
}

/// A gradient bitmap covering the screen, as slot 0, and the op that draws it.
fn gradient_background() -> (Vec<BitmapDelta>, Op) {
    let id = BitmapId::new(0, 0);
    let deltas = bitmap(
        id,
        SIZE.width,
        SIZE.height,
        gradient(SIZE.width, SIZE.height),
    );
    let op = Op::Quad(sprite(id, SIZE.width, SIZE.height, 0, 0));
    (deltas.into(), op)
}

// Fixtures.

/// A checkerboard sprite on a cleared screen: identity copy at an integer position.
fn checkerboard_fixture() -> Fixture {
    let id = BitmapId::new(0, 0);
    single_frame(
        "checkerboard",
        bitmap(id, 32, 32, checkerboard(32, 32, 4, DARK, LIGHT)).into(),
        vec![
            clear(Color::new(96, 96, 96, 255)),
            Op::Quad(sprite(id, 32, 32, 16, 8)),
        ],
    )
}

/// A half-size gradient magnified two times to fill the screen: nearest sampling.
fn gradient_fixture() -> Fixture {
    let id = BitmapId::new(0, 0);
    let (width, height) = (SIZE.width / 2, SIZE.height / 2);
    let magnified = Quad {
        sampling: Sampling::Affine(Transform {
            xx: Fixed::HALF,
            yy: Fixed::HALF,
            ..Transform::IDENTITY
        }),
        ..sprite(id, width, height, 0, 0)
    };
    single_frame(
        "gradient",
        bitmap(id, width, height, gradient(width, height)).into(),
        vec![Op::Quad(magnified)],
    )
}

/// A soft-edged disc over a checkerboard: per-texel alpha in normal mode.
fn alpha_shape_fixture() -> Fixture {
    let (board, shape) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let mut bitmaps: Vec<BitmapDelta> = bitmap(
        board,
        SIZE.width,
        SIZE.height,
        checkerboard(SIZE.width, SIZE.height, 8, DARK, LIGHT),
    )
    .into();
    bitmaps.extend(bitmap(
        shape,
        40,
        40,
        disc(40, Color::new(220, 40, 40, 255)),
    ));
    single_frame(
        "alpha_shape",
        bitmaps,
        vec![
            Op::Quad(sprite(board, SIZE.width, SIZE.height, 0, 0)),
            Op::Quad(sprite(shape, 40, 40, 12, 4)),
        ],
    )
}

/// Two opaque squares, the second partly over the first: op order is draw order.
fn overlap_fixture() -> Fixture {
    let (first, second) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let mut bitmaps: Vec<BitmapDelta> =
        bitmap(first, 28, 28, solid(28, 28, Color::new(200, 60, 40, 255))).into();
    bitmaps.extend(bitmap(
        second,
        28,
        28,
        checkerboard(28, 28, 7, Color::new(40, 80, 200, 255), LIGHT),
    ));
    single_frame(
        "overlap",
        bitmaps,
        vec![
            clear(DARK),
            Op::Quad(sprite(first, 28, 28, 8, 6)),
            Op::Quad(sprite(second, 28, 28, 24, 14)),
        ],
    )
}

/// A screen-sized gradient restricted by a smaller clip rectangle.
fn clip_fixture() -> Fixture {
    let (bitmaps, _) = gradient_background();
    let clipped = Quad {
        clip: Rect::new(16, 12, 32, 24),
        ..sprite(BitmapId::new(0, 0), SIZE.width, SIZE.height, 0, 0)
    };
    single_frame("clip", bitmaps, vec![clear(DARK), Op::Quad(clipped)])
}

/// A soft disc at reduced opacity over a gradient, in the given blend mode. The disc hangs
/// over the right edge of the screen.
fn blend_fixture(name: &'static str, blend: BlendMode) -> Fixture {
    let (mut bitmaps, background) = gradient_background();
    let shape = BitmapId::new(1, 0);
    bitmaps.extend(bitmap(
        shape,
        40,
        40,
        disc(40, Color::new(255, 160, 64, 255)),
    ));
    let quad = Quad {
        opacity: 192,
        blend,
        ..sprite(shape, 40, 40, 30, 4)
    };
    single_frame(name, bitmaps, vec![background, Op::Quad(quad)])
}

/// Three rectangle effects over a gradient: a tone, a colour, and both at once.
fn rect_effect_fixture() -> Fixture {
    let (bitmaps, background) = gradient_background();
    let effect = |clip, tone, color| Op::RectEffect(RectEffect { clip, tone, color });
    let none = Color::new(0, 0, 0, 0);
    single_frame(
        "rect_effect",
        bitmaps,
        vec![
            background,
            effect(Rect::new(4, 4, 16, 40), Tone::new(64, -48, 0, 0), none),
            effect(Rect::new(24, 4, 16, 40), Tone::new(0, 0, 0, 255), none),
            effect(
                Rect::new(44, 4, 16, 40),
                Tone::new(-32, 0, 48, 128),
                Color::new(255, 255, 0, 96),
            ),
        ],
    )
}

/// A checkerboard with one marked corner, turned a quarter turn clockwise: a transform whose
/// axes are swapped.
fn rotation_fixture() -> Fixture {
    let id = BitmapId::new(0, 0);
    let (width, height) = (40, 24);
    let mut rgba = checkerboard(width, height, 8, DARK, LIGHT);
    // Mark the top-left cell so the orientation is visible.
    for y in 0..8 {
        for x in 0..8 {
            let offset = ((y * width + x) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&[230, 60, 60, 255]);
        }
    }
    // Screen (x, y) reads source (y - 4, 43 - x): the source's left edge becomes the top.
    let quarter_turn = Transform {
        xx: Fixed::ZERO,
        xy: Fixed::ONE,
        yx: Fixed::from_int(-1),
        yy: Fixed::ZERO,
        tx: Fixed::from_int(-4),
        ty: Fixed::from_int(44),
    };
    let quad = Quad {
        sampling: Sampling::Affine(quarter_turn),
        ..sprite(id, width, height, 0, 0)
    };
    single_frame(
        "rotation",
        bitmap(id, width, height, rgba).into(),
        vec![clear(Color::new(96, 96, 96, 255)), Op::Quad(quad)],
    )
}

/// Three packets: a bitmap is created in the first, partly updated in the second, and a second
/// bitmap is destroyed and its slot reused in the third. The result is the third frame.
fn persistent_bitmap_fixture() -> Fixture {
    let board = BitmapId::new(0, 0);
    let old = BitmapId::new(1, 0);
    let new = BitmapId::new(1, 1);

    let mut first: Vec<BitmapDelta> =
        bitmap(board, 32, 32, checkerboard(32, 32, 8, DARK, LIGHT)).into();
    first.extend(bitmap(
        old,
        16,
        16,
        solid(16, 16, Color::new(200, 60, 40, 255)),
    ));
    let second = vec![BitmapDelta::Update {
        id: board,
        rect: Rect::new(8, 8, 16, 16),
        rgba: gradient(16, 16),
    }];
    let mut third = vec![BitmapDelta::Destroy { id: old }];
    third.extend(bitmap(new, 16, 16, disc(16, Color::new(40, 200, 80, 255))));

    Fixture {
        name: "persistent_bitmap",
        messages: vec![
            frame(
                0,
                first,
                vec![
                    clear(DARK),
                    Op::Quad(sprite(board, 32, 32, 0, 0)),
                    Op::Quad(sprite(old, 16, 16, 40, 0)),
                ],
            ),
            GuestToHost::SetTitle {
                title: "persistent bitmap".to_owned(),
            },
            frame(
                1,
                second,
                vec![clear(DARK), Op::Quad(sprite(board, 32, 32, 4, 4))],
            ),
            frame(
                2,
                third,
                vec![
                    clear(Color::new(96, 96, 96, 255)),
                    Op::Quad(sprite(board, 32, 32, 16, 8)),
                    Op::Quad(sprite(new, 16, 16, 44, 28)),
                ],
            ),
        ],
    }
}

/// A gradient with a soft disc over it, both recoloured by `tone` and `color` as quads and
/// drawn in the given blend mode: the gradient opaque on the left three quarters of a cleared
/// screen, the disc at reduced opacity and hanging over the right edge.
fn recoloured_fixture(name: &'static str, blend: BlendMode, tone: Tone, color: Color) -> Fixture {
    let (board, shape) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let (width, height) = (48, SIZE.height);
    let mut bitmaps: Vec<BitmapDelta> =
        bitmap(board, width, height, gradient(width, height)).into();
    bitmaps.extend(bitmap(
        shape,
        40,
        40,
        disc(40, Color::new(255, 160, 64, 255)),
    ));
    let recoloured = |quad: Quad, opacity| {
        Op::Quad(Quad {
            tone,
            color,
            opacity,
            blend,
            ..quad
        })
    };
    single_frame(
        name,
        bitmaps,
        vec![
            clear(Color::new(96, 96, 96, 255)),
            recoloured(sprite(board, width, height, 0, 0), 255),
            recoloured(sprite(shape, 40, 40, 30, 4), 192),
        ],
    )
}

/// Quads with a tone (offsets of both signs and partial grey) and no colour.
fn quad_tone_fixture(name: &'static str, blend: BlendMode) -> Fixture {
    let tone = Tone::new(-64, 32, 96, 160);
    recoloured_fixture(name, blend, tone, Color::new(0, 0, 0, 0))
}

/// Quads with a colour of partial alpha and a neutral tone.
fn quad_color_fixture(name: &'static str, blend: BlendMode) -> Fixture {
    let color = Color::new(40, 96, 255, 144);
    recoloured_fixture(name, blend, Tone::NEUTRAL, color)
}
