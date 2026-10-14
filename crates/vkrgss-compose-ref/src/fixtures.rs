//! Generated message streams with committed expected frames.
//!
//! Every fixture is built in code from procedural patterns, so no image asset is needed to
//! test a backend. The expected frame of each fixture is committed as
//! `tests/expected/<name>.png` in this crate; any backend fed the same messages must produce
//! exactly those pixels.

use vkrgss_proto::{
    BitmapDelta, BitmapId, BlendMode, Color, Fixed, FramePacket, FrameRate, GuestToHost, Op, Plane,
    Quad, Rect, RectEffect, Sampling, Size, TableDelta, TableId, TilemapLayer, TilemapPlane, Tone,
    Transform,
};

pub mod tilemap_rgss2;
pub mod tilemap_rgss3;

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
        brightness_fixture(),
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
        brightness: 255,
        tables: vec![],
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

/// Frames at several brightness values, the last of which is a gradient on a translucent
/// background at brightness 100.
fn brightness_fixture() -> Fixture {
    let (deltas, gradient) = gradient_background();
    let ops = |translucent: bool| {
        let mut ops = vec![clear(Color::new(250, 130, 10, 77))];
        if !translucent {
            ops.push(gradient.clone());
        }
        ops.push(Op::Quad(Quad {
            clip: Rect::new(8, 6, 48, 36),
            opacity: 128,
            ..sprite(BitmapId::new(0, 0), SIZE.width, SIZE.height, 0, 0)
        }));
        ops
    };
    let at = |frame_index, brightness, bitmaps, ops| {
        GuestToHost::Frame(FramePacket {
            frame_index,
            logical_size: SIZE,
            frame_rate: FrameRate::DEFAULT,
            brightness,
            tables: vec![],
            bitmaps,
            ops,
        })
    };
    Fixture {
        name: "brightness",
        messages: vec![
            at(0, 0, deltas, ops(false)),
            at(1, 254, vec![], ops(false)),
            at(2, 255, vec![], ops(true)),
            at(3, 1, vec![], ops(false)),
            at(4, 100, vec![], ops(true)),
        ],
    }
}

/// A frame of 16 by 16 pixels at `brightness` whose colour channels hold every value from 0
/// to 255, each in a different order, over alphas that differ too.
pub fn every_level(brightness: u8) -> GuestToHost {
    let size = Size::new(16, 16);
    let ops = (0..=255u8).map(|level| Op::Clear {
        clip: Rect::new(i32::from(level % 16), i32::from(level / 16), 1, 1),
        color: Color::new(level, 255 - level, level.wrapping_mul(77), level ^ 0x5A),
    });
    GuestToHost::Frame(FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        brightness,
        tables: vec![],
        bitmaps: vec![],
        ops: ops.collect(),
    })
}

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

// Tilemap scenes. They have no committed frames: their pixels are checked by the scenario
// tests of this crate and compared between backends.

/// Pixel (`x`, `y`) of the tileset of the tilemap scenes: opaque, naming its tile and its
/// place in the tile.
pub fn tileset_pixel(x: u32, y: u32) -> Color {
    let tile = x / 32 + y / 32 * 8;
    Color::new(tile as u8 + 1, (x % 32) as u8 * 8, (y % 32) as u8 * 8, 255)
}

/// A tileset of 8 by `rows` tiles of [`tileset_pixel`]s.
pub fn tileset(rows: u32) -> Vec<u8> {
    pattern(256, rows * 32, tileset_pixel)
}

/// Pixel (`x`, `y`) of an autotile sheet of the tilemap scenes: opaque, naming its animation
/// frame, its 16 by 16 quarter and its place in the quarter.
pub fn autotile_pixel(x: u32, y: u32) -> Color {
    let quarter = x % 96 / 16 + y / 16 * 6;
    Color::new(
        quarter as u8 + 1,
        (x % 16) as u8 * 16 + (y % 16) as u8,
        (x / 96) as u8 + 100,
        255,
    )
}

/// An autotile sheet of [`autotile_pixel`]s.
pub fn autotile_sheet(width: u32, height: u32) -> Vec<u8> {
    pattern(width, height, autotile_pixel)
}

/// The delta that sets table `id` to `data`, with the sizes of its three dimensions.
pub fn table(id: TableId, sizes: (u32, u32, u32), data: Vec<i16>) -> TableDelta {
    TableDelta::Set {
        id,
        xsize: sizes.0,
        ysize: sizes.1,
        zsize: sizes.2,
        data,
    }
}

/// A tilemap layer op over the whole of a screen of `size`: layer 0, priority 0, at the
/// origin, without a sheet.
pub fn tilemap_layer(size: Size, map: TableId, priorities: TableId) -> TilemapLayer {
    TilemapLayer {
        clip: Rect::from_size(size),
        tileset: None,
        autotiles: [None; 7],
        map,
        priorities,
        origin: (0, 0),
        layer: 0,
        priority: 0,
        animation_step: 0,
    }
}

const MAP: TableId = TableId::new(0);
const PRIORITIES: TableId = TableId::new(1);
const NO_PRIORITIES: TableId = TableId::new(2);
const TILESET: BitmapId = BitmapId::new(0, 0);
const AUTOTILE: BitmapId = BitmapId::new(1, 0);
const OTHER_AUTOTILE: BitmapId = BitmapId::new(2, 0);

fn scene(name: &'static str, size: Size, frames: Vec<SceneFrame>) -> Fixture {
    let messages = frames.into_iter().enumerate().map(|(index, frame)| {
        let mut ops = vec![Op::Clear {
            clip: Rect::from_size(size),
            color: Color::new(40, 90, 160, 255),
        }];
        ops.extend(frame.2.into_iter().map(Op::TilemapLayer));
        GuestToHost::Frame(FramePacket {
            frame_index: index as u64,
            logical_size: size,
            frame_rate: FrameRate::DEFAULT,
            brightness: 255,
            tables: frame.1,
            bitmaps: frame.0,
            ops,
        })
    });
    Fixture {
        name,
        messages: messages.collect(),
    }
}

/// The bitmap deltas, table deltas and tilemap ops of one frame of a scene.
type SceneFrame = (Vec<BitmapDelta>, Vec<TableDelta>, Vec<TilemapLayer>);

/// A frame that holds one op of each kind the protocol has, all of them valid.
pub fn one_op_of_each_kind() -> GuestToHost {
    let size = Size::new(96, 96);
    let mut bitmaps = bitmap(TILESET, 256, 160, tileset(5)).to_vec();
    bitmaps.extend(bitmap(AUTOTILE, 8, 8, gradient(8, 8)));
    GuestToHost::Frame(FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        brightness: 255,
        tables: vec![
            table(MAP, (4, 4, 1), (384..400).collect()),
            table(NO_PRIORITIES, (1, 1, 1), vec![0]),
        ],
        bitmaps,
        ops: vec![
            Op::Clear {
                clip: Rect::from_size(size),
                color: DARK,
            },
            Op::TilemapLayer(TilemapLayer {
                tileset: Some(TILESET),
                ..tilemap_layer(size, MAP, NO_PRIORITIES)
            }),
            Op::Quad(sprite(AUTOTILE, 8, 8, 20, 30)),
            Op::RectEffect(RectEffect {
                clip: Rect::new(8, 8, 40, 40),
                tone: Tone::new(40, -40, 0, 128),
                color: Color::new(255, 255, 0, 64),
            }),
            // In RGSS2 the numbers of the map name tiles of the second sheet of whole tiles.
            Op::TilemapPlane(TilemapPlane {
                clip: Rect::new(48, 48, 48, 48),
                bitmaps: [
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(TILESET),
                    None,
                    None,
                ],
                ..tilemap_rgss2::tilemap_plane(Plane::Lower, MAP)
            }),
        ],
    })
}

/// Streams of frames drawn with tilemap layer ops: every autotile pattern and animation,
/// wrapped origins, priorities, translucent stacked tiles and the inputs that draw nothing.
pub fn tilemap_scenes() -> Vec<Fixture> {
    vec![
        tilemap_patterns(),
        tilemap_wrap(),
        tilemap_priorities(),
        tilemap_translucent(),
        tilemap_edges(),
    ]
}

/// All 48 patterns of two autotiles, one of them animated, at several animation steps.
fn tilemap_patterns() -> Fixture {
    let size = Size::new(256, 192);
    let mut bitmaps = bitmap(AUTOTILE, 384, 128, autotile_sheet(384, 128)).to_vec();
    bitmaps.extend(bitmap(OTHER_AUTOTILE, 96, 128, autotile_sheet(96, 128)));
    // Layer 0 holds the patterns of the first autotile, layer 1 those of the seventh.
    let cells = (0..48).map(|p| 48 + p).chain((0..48).map(|p| 7 * 48 + p));
    let tables = vec![
        table(MAP, (8, 6, 2), cells.collect()),
        table(NO_PRIORITIES, (0, 1, 1), vec![]),
    ];
    let layer = |layer, animation_step| TilemapLayer {
        autotiles: [
            Some(AUTOTILE),
            None,
            None,
            None,
            None,
            None,
            Some(OTHER_AUTOTILE),
        ],
        layer,
        animation_step,
        ..tilemap_layer(size, MAP, NO_PRIORITIES)
    };
    let mut frames = vec![(bitmaps, tables, vec![layer(0, 0)])];
    for step in [1, 3, 5, u64::MAX] {
        frames.push((vec![], vec![], vec![layer(0, step)]));
    }
    frames.push((vec![], vec![], vec![layer(1, 2)]));
    scene("tilemap_patterns", size, frames)
}

/// A map of tileset tiles at origins inside, beyond and before the map.
fn tilemap_wrap() -> Fixture {
    let size = Size::new(100, 70);
    let cells = (0..5 * 4).map(|cell| 384 + cell * 3 % 40);
    let tables = vec![
        table(MAP, (5, 4, 1), cells.collect()),
        table(NO_PRIORITIES, (1, 1, 1), vec![0]),
    ];
    let at = |origin| TilemapLayer {
        tileset: Some(TILESET),
        origin,
        ..tilemap_layer(size, MAP, NO_PRIORITIES)
    };
    let bitmaps = bitmap(TILESET, 256, 160, tileset(5)).to_vec();
    let mut frames = vec![(bitmaps, tables, vec![at((0, 0))])];
    for origin in [
        (7, 13),
        (160, 128),
        (-1, -1),
        (-333, 1000),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
    ] {
        frames.push((vec![], vec![], vec![at(origin)]));
    }
    scene("tilemap_wrap", size, frames)
}

/// Three layers of tiles of every priority, drawn priority by priority and, from priority 1
/// on, row by row.
fn tilemap_priorities() -> Fixture {
    let size = Size::new(96, 96);
    // The priority of tile 384 + n is n mod 7: 6 is one no op draws.
    let priorities = (0..400).map(|id| if id < 384 { 0 } else { (id - 384) % 7 });
    let cells = (0..4 * 4 * 3).map(|cell| 384 + cell * 5 % 31);
    let tables = vec![
        table(MAP, (4, 4, 3), cells.collect()),
        table(PRIORITIES, (400, 1, 1), priorities.collect()),
    ];
    let mut ops = Vec::new();
    for priority in 0..=5 {
        for row in 0..3 {
            for layer in 0..3 {
                let clip = match priority {
                    0 if row > 0 => continue,
                    0 => Rect::from_size(size),
                    _ => Rect::new(0, row * 32 - 5, 96, 32),
                };
                ops.push(TilemapLayer {
                    clip,
                    tileset: Some(TILESET),
                    origin: (3, 5),
                    layer,
                    priority,
                    ..tilemap_layer(size, MAP, PRIORITIES)
                });
            }
        }
    }
    let bitmaps = bitmap(TILESET, 256, 160, tileset(5)).to_vec();
    scene("tilemap_priorities", size, vec![(bitmaps, tables, ops)])
}

/// Translucent tiles of one priority stacked in three layers.
fn tilemap_translucent() -> Fixture {
    let size = Size::new(64, 64);
    let translucent = pattern(256, 64, |x, y| {
        let opaque = tileset_pixel(x, y);
        let alpha = [142, 0, 255, 1, 254, 77][(x / 32 + y / 32 + x % 3) as usize % 6];
        Color::new(opaque.r * 9, opaque.g, opaque.b, alpha)
    });
    let cells = (0..2 * 2 * 3).map(|cell| 384 + cell * 7 % 16);
    let tables = vec![
        table(MAP, (2, 2, 3), cells.collect()),
        table(NO_PRIORITIES, (0, 0, 0), vec![]),
    ];
    let ops = (0..3).map(|layer| TilemapLayer {
        tileset: Some(TILESET),
        layer,
        ..tilemap_layer(size, MAP, NO_PRIORITIES)
    });
    let bitmaps = bitmap(TILESET, 256, 64, translucent).to_vec();
    scene(
        "tilemap_translucent",
        size,
        vec![(bitmaps, tables, ops.collect())],
    )
}

/// What draws nothing or only in part: tile numbers below 48, tiles beyond a short or narrow
/// tileset, absent and empty sheets, a layer the map does not have, an empty map; and the
/// sheets of whole tiles, 32 pixels high or less.
fn tilemap_edges() -> Fixture {
    let size = Size::new(128, 96);
    let (strip, low, narrow, empty) = (
        BitmapId::new(3, 0),
        BitmapId::new(4, 0),
        BitmapId::new(5, 0),
        BitmapId::new(6, 0),
    );
    let mut bitmaps = bitmap(TILESET, 200, 48, pattern(200, 48, tileset_pixel)).to_vec();
    bitmaps.extend(bitmap(AUTOTILE, 96, 64, autotile_sheet(96, 64)));
    bitmaps.extend(bitmap(strip, 100, 32, autotile_sheet(100, 32)));
    bitmaps.extend(bitmap(low, 70, 20, autotile_sheet(70, 20)));
    bitmaps.extend(bitmap(narrow, 50, 128, autotile_sheet(50, 128)));
    bitmaps.push(BitmapDelta::Create {
        id: empty,
        width: 0,
        height: 7,
    });
    #[rustfmt::skip]
    let cells = vec![
        0, 47, -1, i16::MIN,
        384, 384 + 6, 384 + 7, 384 + 8,
        384 + 14, 384 + 16, i16::MAX, 48 + 47,
        // A second layer: every autotile.
        48, 96 + 5, 144 + 9, 192 + 47,
        240 + 20, 288 + 33, 336 + 46, 383,
        95, 143, 191, 239,
    ];
    let empty_map = TableId::new(7);
    let tables = vec![
        table(MAP, (4, 3, 2), cells),
        table(NO_PRIORITIES, (3, 0, 1), vec![]),
        table(empty_map, (0, 4, 1), vec![]),
    ];
    let layer = |layer, animation_step| TilemapLayer {
        tileset: Some(TILESET),
        autotiles: [
            Some(AUTOTILE),
            Some(strip),
            Some(low),
            Some(narrow),
            None,
            Some(empty),
            Some(strip),
        ],
        layer,
        animation_step,
        ..tilemap_layer(size, MAP, NO_PRIORITIES)
    };
    let frames = vec![
        (bitmaps, tables, vec![layer(0, 0), layer(1, 0)]),
        (
            vec![],
            vec![],
            vec![layer(1, 1), layer(2, 0), layer(255, 0)],
        ),
        (vec![], vec![], vec![layer(1, 5)]),
        (
            vec![],
            vec![],
            vec![
                TilemapLayer {
                    tileset: Some(empty),
                    ..layer(0, 0)
                },
                TilemapLayer {
                    map: empty_map,
                    ..layer(0, 0)
                },
                TilemapLayer {
                    priority: 1,
                    ..layer(0, 0)
                },
            ],
        ),
    ];
    scene("tilemap_edges", size, frames)
}
