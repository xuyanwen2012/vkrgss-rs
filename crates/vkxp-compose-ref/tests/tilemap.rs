//! One test per scenario of the tilemap requirements of `specs/frame-composition/spec.md`,
//! plus the unknown-table scenario of `specs/guest-host-protocol/spec.md`.
//!
//! Expected pixels are derived here from the patterns of the sheets, not from the evaluation
//! under test.

use vkxp_compose_ref::fixtures::{
    self, autotile_pixel, autotile_sheet, bitmap, table, tilemap_layer, tileset, tileset_pixel,
};
use vkxp_compose_ref::{ComposeError, Compositor, Frame, TableError};
use vkxp_pixel::Rgba;
use vkxp_proto::tiles::AUTOTILE_QUARTERS;
use vkxp_proto::{
    BitmapDelta, BitmapId, BlendMode, Color, FramePacket, FrameRate, Op, Quad, Rect, Sampling,
    Size, TableDelta, TableId, TilemapLayer, Tone,
};

const SIZE: Size = Size::new(96, 64);
const GROUND: Rgba = [40, 90, 160, 255];
const MAP: TableId = TableId::new(0);
const PRIORITIES: TableId = TableId::new(1);
const TILESET: BitmapId = BitmapId::new(0, 0);
const AUTOTILE: BitmapId = BitmapId::new(1, 0);

fn packet(
    size: Size,
    bitmaps: Vec<BitmapDelta>,
    tables: Vec<TableDelta>,
    layers: Vec<TilemapLayer>,
) -> FramePacket {
    let mut ops = vec![Op::Clear {
        clip: Rect::from_size(size),
        color: Color::from_rgba(GROUND),
    }];
    ops.extend(layers.into_iter().map(Op::TilemapLayer));
    FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        tables,
        bitmaps,
        ops,
    }
}

/// A 3 by 2 map with one layer whose cells are `cells`, and the priorities of the tiles from
/// 384 on, `priorities`.
fn tables(cells: [i16; 6], priorities: &[i16]) -> Vec<TableDelta> {
    let mut entries = vec![0; 384];
    entries.extend_from_slice(priorities);
    vec![
        table(MAP, (3, 2, 1), cells.to_vec()),
        table(PRIORITIES, (entries.len() as u32, 1, 1), entries),
    ]
}

fn layer() -> TilemapLayer {
    TilemapLayer {
        tileset: Some(TILESET),
        autotiles: [Some(AUTOTILE), None, None, None, None, None, None],
        ..tilemap_layer(SIZE, MAP, PRIORITIES)
    }
}

/// The frame of `layers` over the ground, with a tileset of 5 rows and a one-frame autotile.
fn render(tables: Vec<TableDelta>, layers: Vec<TilemapLayer>) -> Frame {
    let mut bitmaps = bitmap(TILESET, 256, 160, tileset(5)).to_vec();
    bitmaps.extend(bitmap(AUTOTILE, 96, 128, autotile_sheet(96, 128)));
    Compositor::new()
        .render(&packet(SIZE, bitmaps, tables, layers))
        .unwrap()
}

/// Asserts every pixel of `frame` against `expected(x, y)`, where `None` is the ground.
fn assert_pixels(frame: &Frame, mut expected: impl FnMut(u32, u32) -> Option<Rgba>) {
    for y in 0..frame.size.height {
        for x in 0..frame.size.width {
            let expected = expected(x, y).unwrap_or(GROUND);
            assert_eq!(frame.pixel(x, y), expected, "pixel ({x}, {y})");
        }
    }
}

/// Pixel (`x`, `y`) of the tileset block at (`column`, `row`).
fn block(column: u32, row: u32, x: u32, y: u32) -> Rgba {
    tileset_pixel(column * 32 + x % 32, row * 32 + y % 32).to_rgba()
}

// Requirement: Tilemap layer evaluation

#[test]
fn tileset_tile_shows_its_block_of_the_tileset() {
    let frame = render(tables([0, 0, 0, 0, 384 + 9, 0], &[]), vec![layer()]);
    assert_pixels(&frame, |x, y| {
        (x / 32 == 1 && y / 32 == 1).then(|| block(1, 1, x, y))
    });
}

#[test]
fn tile_of_another_priority_is_left_alone() {
    // Tile 384 has priority 2, tile 385 priority 0.
    let tables = || tables([384, 385, 0, 0, 0, 0], &[2, 0]);
    let frame = render(tables(), vec![layer()]);
    assert_pixels(&frame, |x, y| {
        (x / 32 == 1 && y / 32 == 0).then(|| block(1, 0, x, y))
    });
    let two = TilemapLayer {
        priority: 2,
        ..layer()
    };
    let frame = render(tables(), vec![two]);
    assert_pixels(&frame, |x, y| {
        (x / 32 == 0 && y / 32 == 0).then(|| block(0, 0, x, y))
    });
    // A tile number beyond the priorities has priority 0.
    let frame = render(tables(), vec![layer(), two]);
    let beyond = self::tables([384 + 30, 0, 0, 0, 0, 0], &[2, 0]);
    assert_eq!(frame.pixel(0, 0), block(0, 0, 0, 0));
    let frame = render(beyond, vec![layer()]);
    assert_eq!(frame.pixel(5, 6), block(6, 3, 5, 6));
}

#[test]
fn map_repeats_in_both_directions() {
    let cells = [384, 385, 386, 387, 388, 389];
    for origin in [
        (0, 0),
        (96, 64),
        (96 * 5 + 7, 64 * 3 + 9),
        (-1, -1),
        (-96 * 4 - 50, -64 * 9 - 33),
        (i32::MAX, i32::MIN),
    ] {
        let frame = render(tables(cells, &[]), vec![TilemapLayer { origin, ..layer() }]);
        assert_pixels(&frame, |x, y| {
            let (map_x, map_y) = (
                i64::from(x) + i64::from(origin.0),
                i64::from(y) + i64::from(origin.1),
            );
            let column = map_x.div_euclid(32).rem_euclid(3) as u32;
            let row = map_y.div_euclid(32).rem_euclid(2) as u32;
            let (tx, ty) = (map_x.rem_euclid(32) as u32, map_y.rem_euclid(32) as u32);
            Some(block(column + row * 3, 0, tx, ty))
        });
    }
}

#[test]
fn translucent_texel_blends_like_a_sprite_pixel() {
    let sheet: Vec<u8> = (0..32 * 32)
        .flat_map(|i| [200, (i % 256) as u8, 30, if i % 2 == 0 { 142 } else { 0 }])
        .collect();
    let bitmaps = || bitmap(TILESET, 32, 32, sheet.clone()).to_vec();
    let size = Size::new(32, 32);
    let tables = vec![
        table(MAP, (1, 1, 1), vec![384]),
        table(PRIORITIES, (0, 1, 1), vec![]),
    ];
    let op = TilemapLayer {
        tileset: Some(TILESET),
        ..tilemap_layer(size, MAP, PRIORITIES)
    };
    let tiles = Compositor::new()
        .render(&packet(size, bitmaps(), tables, vec![op]))
        .unwrap();
    let mut sprite = packet(size, bitmaps(), vec![], vec![]);
    sprite.ops.push(Op::Quad(Quad {
        clip: Rect::from_size(size),
        bitmap: TILESET,
        src: Rect::new(0, 0, 32, 32),
        sampling: Sampling::place_at(0, 0),
        tone: Tone::NEUTRAL,
        color: Color::new(0, 0, 0, 0),
        opacity: 255,
        blend: BlendMode::Normal,
    }));
    assert_eq!(tiles, Compositor::new().render(&sprite).unwrap());
    assert_eq!(
        tiles.pixel(0, 0),
        vkxp_pixel::blend_normal(GROUND, [200, 0, 30, 142], 255)
    );
    assert_ne!(tiles.pixel(0, 0), GROUND);
    assert_eq!(tiles.pixel(1, 0), GROUND);
}

// Requirement: Tileset tiles in the tilemap op

#[test]
fn tile_number_selects_the_block_by_column_and_row() {
    let frame = render(
        tables([384 + 17, 0, 384 + 39, 384, 0, 384 + 7], &[]),
        vec![layer()],
    );
    assert_pixels(&frame, |x, y| match (x / 32, y / 32) {
        (0, 0) => Some(block(1, 2, x, y)),
        (2, 0) => Some(block(7, 4, x, y)),
        (0, 1) => Some(block(0, 0, x, y)),
        (2, 1) => Some(block(7, 0, x, y)),
        _ => None,
    });
}

#[test]
fn tile_beyond_a_short_tileset_and_numbers_below_48_draw_nothing() {
    // The tileset has 5 rows: tile 384 + 40 is in the sixth.
    let frame = render(
        tables([384 + 40, i16::MAX, 47, 1, -1, 0], &[]),
        vec![layer()],
    );
    assert_pixels(&frame, |_, _| None);
    let absent = TilemapLayer {
        tileset: None,
        ..layer()
    };
    let frame = render(tables([384, 385, 386, 387, 388, 389], &[]), vec![absent]);
    assert_pixels(&frame, |_, _| None);
}

// Requirement: Autotiles in the tilemap op

#[test]
fn every_autotile_pattern_is_made_of_the_quarters_the_table_names() {
    let size = Size::new(256, 192);
    let tables = vec![
        table(MAP, (8, 6, 1), (48 * 3..48 * 4).collect()),
        table(PRIORITIES, (0, 1, 1), vec![]),
    ];
    let op = TilemapLayer {
        autotiles: [None, None, Some(AUTOTILE), None, None, None, None],
        ..tilemap_layer(size, MAP, PRIORITIES)
    };
    let bitmaps = bitmap(AUTOTILE, 96, 128, autotile_sheet(96, 128)).to_vec();
    let frame = Compositor::new()
        .render(&packet(size, bitmaps, tables, vec![op]))
        .unwrap();
    let mut seen = std::collections::BTreeSet::new();
    assert_pixels(&frame, |x, y| {
        let pattern = (x / 32 + y / 32 * 8) as usize;
        let corner = (x % 32 / 16 + y % 32 / 16 * 2) as usize;
        let quarter = u32::from(AUTOTILE_QUARTERS[pattern][corner]) - 1;
        seen.insert(quarter);
        // The quarter's red channel is its number, counted from 1.
        let pixel = autotile_pixel(quarter % 6 * 16 + x % 16, quarter / 6 * 16 + y % 16);
        assert_eq!(u32::from(pixel.r), quarter + 1);
        Some(pixel.to_rgba())
    });
    assert!(seen.len() > 40, "the patterns use most quarters");
}

#[test]
fn animation_step_selects_the_frame_modulo_their_number() {
    let cells = [48, 48 + 47, 0, 0, 0, 0];
    for (step, frame_shown) in [(0, 0), (1, 1), (5, 1), (3, 3), (u64::MAX, 3)] {
        let mut bitmaps = bitmap(TILESET, 256, 160, tileset(5)).to_vec();
        bitmaps.extend(bitmap(AUTOTILE, 384, 128, autotile_sheet(384, 128)));
        let op = TilemapLayer {
            animation_step: step,
            ..layer()
        };
        let frame = Compositor::new()
            .render(&packet(SIZE, bitmaps, tables(cells, &[]), vec![op]))
            .unwrap();
        assert_pixels(&frame, |x, y| {
            let pattern = match (x / 32, y / 32) {
                (0, 0) => 0,
                (1, 0) => 47,
                _ => return None,
            };
            let corner = (x % 32 / 16 + y % 32 / 16 * 2) as usize;
            let quarter = u32::from(AUTOTILE_QUARTERS[pattern][corner]) - 1;
            let pixel = autotile_pixel(
                frame_shown * 96 + quarter % 6 * 16 + x % 16,
                quarter / 6 * 16 + y % 16,
            );
            assert_eq!(u32::from(pixel.b), 100 + frame_shown, "step {step}");
            Some(pixel.to_rgba())
        });
    }
}

#[test]
fn autotile_without_a_bitmap_draws_nothing() {
    // Autotiles 2 to 7 are absent.
    let frame = render(tables([96, 143, 144, 383, 240, 300], &[]), vec![layer()]);
    assert_pixels(&frame, |_, _| None);
}

#[test]
fn sheet_32_pixels_high_holds_whole_tiles() {
    let bitmaps = bitmap(AUTOTILE, 96, 32, autotile_sheet(96, 32)).to_vec();
    for (step, shown) in [(0, 0), (4, 1), (2, 2)] {
        let op = TilemapLayer {
            tileset: None,
            animation_step: step,
            ..layer()
        };
        let cells = [48, 60, 95, 0, 0, 0];
        let frame = Compositor::new()
            .render(&packet(SIZE, bitmaps.clone(), tables(cells, &[]), vec![op]))
            .unwrap();
        assert_pixels(&frame, |x, y| {
            (y < 32).then(|| autotile_pixel(shown * 32 + x % 32, y).to_rgba())
        });
    }
}

// guest-host-protocol, Requirement: Table resources

#[test]
fn unknown_table_is_a_typed_error_and_no_frame() {
    let bitmaps = || bitmap(TILESET, 256, 160, tileset(5)).to_vec();
    let only_map = vec![table(MAP, (3, 2, 1), vec![384; 6])];
    let op = TilemapLayer {
        autotiles: [None; 7],
        ..layer()
    };
    let result = Compositor::new().render(&packet(SIZE, bitmaps(), only_map, vec![op]));
    assert_eq!(
        result,
        Err(ComposeError::InvalidOpTable {
            index: 1,
            source: TableError::Unknown { id: PRIORITIES },
        })
    );
    assert_eq!(
        result.unwrap_err().to_string(),
        "op 1 is invalid: unknown table 1"
    );

    // A destroyed table is unknown again.
    let mut compositor = Compositor::new();
    compositor
        .render(&packet(SIZE, bitmaps(), tables([384; 6], &[]), vec![op]))
        .unwrap();
    let destroy = vec![TableDelta::Destroy { id: MAP }];
    assert_eq!(
        compositor.render(&packet(SIZE, vec![], destroy, vec![op])),
        Err(ComposeError::InvalidOpTable {
            index: 1,
            source: TableError::Unknown { id: MAP },
        })
    );
}

#[test]
fn invalid_table_delta_is_a_typed_error_and_no_frame() {
    let short = vec![table(MAP, (3, 2, 1), vec![384; 5])];
    assert_eq!(
        Compositor::new().render(&packet(SIZE, vec![], short, vec![])),
        Err(ComposeError::InvalidTableDelta {
            index: 0,
            source: TableError::LengthMismatch {
                id: MAP,
                xsize: 3,
                ysize: 2,
                zsize: 1,
                found: 5,
            },
        })
    );
}

#[test]
fn table_replaced_by_a_later_packet_shows_in_its_frame() {
    let mut compositor = Compositor::new();
    let bitmaps = bitmap(TILESET, 256, 160, tileset(5)).to_vec();
    let op = TilemapLayer {
        autotiles: [None; 7],
        ..layer()
    };
    let first = packet(SIZE, bitmaps, tables([384; 6], &[]), vec![op]);
    assert_eq!(
        compositor.render(&first).unwrap().pixel(40, 3),
        block(0, 0, 8, 3)
    );
    let replaced = vec![table(MAP, (3, 2, 1), vec![384, 390, 384, 384, 384, 384])];
    let second = packet(SIZE, vec![], replaced, vec![op]);
    assert_eq!(
        compositor.render(&second).unwrap().pixel(40, 3),
        block(6, 0, 8, 3)
    );
}

// The scenes other backends are compared on render here too.

#[test]
fn tilemap_scenes_render_and_draw_something() {
    for scene in fixtures::tilemap_scenes() {
        let mut compositor = Compositor::new();
        let mut drawn = false;
        for message in &scene.messages {
            let frame = compositor.handle(message).unwrap().unwrap();
            drawn |= frame
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| *pixel != GROUND);
        }
        assert!(drawn, "scene `{}` shows only the ground", scene.name);
    }
}
