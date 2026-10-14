//! The scenarios of the tilemap plane op of RGSS2 of `specs/frame-composition/spec.md`.
//!
//! The topics are compared with digests of the frames that the model of the measurement
//! renders (`tests/data/tilemap-rgss2-digests.txt`: topic, frame, digest). The other tests derive
//! their expected pixels here from the patterns of the sheets.

use vkrgss_compose_ref::fixtures::table;
use vkrgss_compose_ref::fixtures::tilemap_rgss2::{
    self as scenes, SIZE, Set, sheet, sheet_pixel, tilemap_plane,
};
use vkrgss_compose_ref::{Compositor, Frame};
use vkrgss_pixel::Rgba;
use vkrgss_proto::{
    BitmapDelta, BitmapId, Color, FramePacket, FrameRate, GuestToHost, Op, Plane, Rect, TableDelta,
    TableId, TilemapPlane,
};

const EXPECTED: &str = include_str!("data/tilemap-rgss2-digests.txt");

/// The committed digests of a topic, in frame order.
fn expected(topic: &str) -> Vec<u64> {
    let mut digests = Vec::new();
    for line in EXPECTED.lines().filter(|line| !line.starts_with('#')) {
        let mut words = line.split(' ');
        if words.next() != Some(topic) {
            continue;
        }
        let frame: usize = words.next().unwrap().parse().unwrap();
        assert_eq!(frame, digests.len(), "{line}");
        digests.push(u64::from_str_radix(words.next().unwrap(), 16).unwrap());
    }
    digests
}

/// Renders every frame of a topic and compares it with the model's.
fn assert_topic(name: &str, frames: usize) {
    let topic = scenes::topic(name);
    let expected = expected(name);
    assert_eq!((topic.frames.len(), expected.len()), (frames, frames));
    let mut compositor = Compositor::new();
    let mut wrong = Vec::new();
    for (index, (message, expected)) in topic.frames.iter().zip(expected).enumerate() {
        let frame = compositor
            .handle(message)
            .unwrap_or_else(|error| panic!("frame {index} of `{name}` failed: {error}"))
            .expect("a frame message");
        assert_eq!(frame.size, SIZE);
        if scenes::digest(&frame.rgba) != expected {
            wrong.push(index);
        }
    }
    assert!(wrong.is_empty(), "frames of `{name}` differ: {wrong:?}");
}

// Requirement: Tiles of a tilemap plane of RGSS2. Scenario "Every number and shape".

#[test]
fn every_number_and_shape_is_drawn_as_measured() {
    assert_topic("ids", 366);
}

// Scenario "Animation, measured on the original".

#[test]
fn the_animation_steps_are_drawn_as_measured() {
    assert_topic("anim", 6);
}

// Requirement: Planes, tables and shadows of RGSS2. Scenario "Cases of the measurement".

#[test]
fn tables_are_drawn_as_measured() {
    assert_topic("tables1", 6);
}

#[test]
fn shadows_beside_walls_are_drawn_as_measured() {
    assert_topic("shadow1", 60);
}

#[test]
fn shadow_targets_and_givers_are_as_measured() {
    assert_topic("shadow2", 22);
}

#[test]
fn the_planes_lie_among_sprites_as_measured() {
    assert_topic("z1", 24);
}

#[test]
fn scrolled_and_wrapped_maps_are_drawn_as_measured() {
    assert_topic("scroll1", 66);
}

// Requirement: Blend and flash of a tilemap plane of RGSS2. Scenario "Translucent tiles,
// measured on the original".

#[test]
fn translucent_tiles_are_blended_as_measured() {
    assert_topic("alpha1", 12);
}

#[test]
fn the_flash_sequence_is_drawn_as_measured() {
    assert_topic("flash1", 59);
}

// Requirement: Whole tilemaps of RGSS2 as measured. Scenario "Random scenes".

#[test]
fn forty_random_scenes_are_drawn_as_measured() {
    assert_topic("rand1", 40);
}

const MAP: TableId = TableId::new(0);
const PASSAGES: TableId = TableId::new(1);
const FLASH: TableId = TableId::new(2);
const GROUND: Rgba = [200, 200, 200, 255];

/// The bitmap of sheet `index` of the set `Code`.
const fn code(index: usize) -> BitmapId {
    BitmapId::new(index as u32, 0)
}

/// The frame of `planes` over the ground, with the nine sheets of the set `Code`.
fn render(tables: Vec<TableDelta>, planes: Vec<TilemapPlane>) -> Frame {
    let mut bitmaps: Vec<BitmapDelta> = Vec::new();
    for index in 0..9 {
        bitmaps.extend(sheet(code(index), Set::Code, index));
    }
    let mut ops = vec![Op::Clear {
        clip: Rect::from_size(SIZE),
        color: Color::from_rgba(GROUND),
    }];
    ops.extend(planes.into_iter().map(Op::TilemapPlane));
    Compositor::new()
        .render(&FramePacket {
            frame_index: 0,
            logical_size: SIZE,
            frame_rate: FrameRate::MAX,
            brightness: 255,
            bitmaps,
            tables,
            ops,
        })
        .unwrap()
}

/// A plane with every sheet of the set `Code`.
fn plane(plane: Plane) -> TilemapPlane {
    let mut op = tilemap_plane(plane, MAP);
    for (index, bitmap) in op.bitmaps.iter_mut().enumerate() {
        *bitmap = Some(code(index));
    }
    op
}

/// A map of 4 by 3 cells whose first layer is `cells`, the others empty.
fn map(cells: [i16; 12]) -> TableDelta {
    let mut data = cells.to_vec();
    data.resize(36, 0);
    table(MAP, (4, 3, 3), data)
}

/// The pixel of the set `Code`.
fn texel(index: usize, x: u32, y: u32) -> Rgba {
    sheet_pixel(Set::Code, index, x, y).to_rgba()
}

// Scenario "Animation, measured on the original", pixel by pixel.

#[test]
fn a1_comes_from_the_block_moved_by_the_step() {
    // Autotiles 0 (animated floor), 2 (still floor) and 5 (waterfall) of A1, shape 47 and
    // shape 3: both show the block's top-left tile.
    let cells = [
        2048 + 47,
        2048 + 96 + 47,
        2048 + 240 + 3,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let moved = [(0, 0), (64, 32), (128, 64), (64, 0), (0, 32), (64, 64)];
    for (step, (floor, fall)) in moved.into_iter().enumerate() {
        let op = TilemapPlane {
            animation_step: step as u64,
            ..plane(Plane::Lower)
        };
        let frame = render(vec![map(cells)], vec![op]);
        for (x, y) in [(0, 0), (17, 5), (31, 31)] {
            assert_eq!(frame.pixel(x, y), texel(0, floor + x, y), "step {step}");
            assert_eq!(frame.pixel(32 + x, y), texel(0, 192 + x, y), "step {step}");
            // Shape 3 of a waterfall: the pieces at x 0 and 48 of the block.
            let column = if x < 16 { x } else { 32 + x };
            let expected = texel(0, 448 + column, fall + y % 32);
            assert_eq!(frame.pixel(64 + x, y), expected, "step {step}");
        }
    }
}

// Scenario "Undefined numbers".

#[test]
fn undefined_numbers_leave_their_cells_unchanged() {
    let cells = [1024, 1535, -256, 37, 0, 0, 0, 0, 0, 0, 0, 0];
    for which in [Plane::Lower, Plane::Upper] {
        let frame = render(vec![map(cells)], vec![plane(which)]);
        for y in 0..32 {
            for x in 0..96 {
                assert_eq!(frame.pixel(x, y), GROUND, "pixel ({x}, {y})");
            }
        }
        // The map repeats every 4 cells: a drawn tile and the undefined ones again.
        let drawn = which == Plane::Lower;
        let tile = if drawn { texel(5, 160, 128) } else { GROUND };
        assert_eq!(frame.pixel(96, 0), tile);
        assert_eq!(frame.pixel(128, 0), GROUND);
    }
}

// Scenario "Shadow, measured on the original".

#[test]
fn a_floor_beside_two_walls_takes_a_shadow_on_its_left_half() {
    let (wall, floor) = (5888, 2816 + 48);
    let cells = [wall, 0, 0, 0, wall, floor, 0, 0, 0, 0, 0, 0];
    let frame = render(vec![map(cells)], vec![plane(Plane::Lower)]);
    // Shape 0 of the second autotile of A2: its pieces at (32, 64), (16, 64), (32, 48),
    // (16, 48) of the block at (64, 0).
    let source = |x: u32, y: u32| {
        let (px, py) = [(32, 64), (16, 64), (32, 48), (16, 48)][(y / 16 * 2 + x / 16) as usize];
        texel(1, 64 + px + x % 16, py + y % 16)
    };
    for y in 0..32 {
        for x in 0..32 {
            let mut expected = source(x, y);
            if x < 16 {
                for channel in &mut expected[..3] {
                    *channel = ((u16::from(*channel) * 127) >> 8) as u8;
                }
            }
            assert_eq!(frame.pixel(32 + x, 32 + y), expected, "pixel ({x}, {y})");
        }
    }

    // Without a sheet for the floor the shadow falls on the frame: 200 becomes 99.
    let mut op = plane(Plane::Lower);
    op.bitmaps[1] = None;
    let frame = render(vec![map(cells)], vec![op]);
    assert_eq!(frame.pixel(32, 32), [99, 99, 99, 255]);
    assert_eq!(frame.pixel(47, 63), [99, 99, 99, 255]);
    assert_eq!(frame.pixel(48, 32), GROUND);
    // The upper plane draws no shadow.
    let frame = render(vec![map(cells)], vec![plane(Plane::Upper)]);
    assert_eq!(frame.pixel(32, 32), GROUND);
}

// Scenario "Star tile in a lower layer, measured on the original".

#[test]
fn a_star_tile_is_drawn_by_the_upper_plane_from_the_third_layer_only() {
    // Tile 37 in all three layers of one cell each: columns 0, 1, 2.
    let mut data = vec![0; 36];
    (data[0], data[12 + 1], data[24 + 2]) = (37, 37, 37);
    let mut passages = vec![0; 64];
    passages[37] = 0x10;
    let star = |plane_kind| TilemapPlane {
        passages: Some(PASSAGES),
        ..plane(plane_kind)
    };
    let tables = || {
        vec![
            table(MAP, (4, 3, 3), data.clone()),
            table(PASSAGES, (64, 1, 1), passages.clone()),
        ]
    };
    let tile = texel(5, 160, 128);
    let lower = render(tables(), vec![star(Plane::Lower)]);
    assert_eq!(
        [lower.pixel(0, 0), lower.pixel(32, 0), lower.pixel(64, 0)],
        [GROUND; 3]
    );
    let upper = render(tables(), vec![star(Plane::Upper)]);
    assert_eq!(
        [upper.pixel(0, 0), upper.pixel(32, 0), upper.pixel(64, 0)],
        [GROUND, GROUND, tile]
    );
    // Without the bit the lower plane draws all three and the upper none.
    let lower = render(tables(), vec![plane(Plane::Lower)]);
    assert_eq!(
        [lower.pixel(0, 0), lower.pixel(32, 0), lower.pixel(64, 0)],
        [tile; 3]
    );
    let upper = render(tables(), vec![plane(Plane::Upper)]);
    assert_eq!(upper.pixel(64, 0), GROUND);
}

// Requirement: Planes, tables and shadows of RGSS2: nothing outside the plane.

#[test]
fn nothing_is_drawn_outside_the_672_by_512_pixels_of_a_plane() {
    let cells = [37; 12];
    let op = TilemapPlane {
        corner: (-200, -150),
        ..plane(Plane::Lower)
    };
    let frame = render(vec![map(cells)], vec![op]);
    assert_ne!(frame.pixel(471, 361), GROUND);
    assert_eq!(frame.pixel(472, 361), GROUND);
    assert_eq!(frame.pixel(471, 362), GROUND);
    let op = TilemapPlane {
        corner: (40, 30),
        clip: Rect::new(0, 0, 100, 100),
        ..plane(Plane::Lower)
    };
    let frame = render(vec![map(cells)], vec![op]);
    assert_eq!(frame.pixel(39, 30), GROUND);
    assert_eq!(frame.pixel(40, 29), GROUND);
    assert_eq!(frame.pixel(40, 30), texel(5, 160, 128));
    assert_eq!(frame.pixel(99, 99), texel(5, 160 + 27, 128 + 5));
    assert_eq!(frame.pixel(100, 99), GROUND);
}

// Scenario "Flash twice".

#[test]
fn both_planes_add_the_flash_of_a_cell() {
    let ground = Op::Clear {
        clip: Rect::from_size(SIZE),
        color: Color::new(100, 100, 100, 255),
    };
    let op = |which| {
        Op::TilemapPlane(TilemapPlane {
            flash: Some(FLASH),
            flash_level: 4,
            ..tilemap_plane(which, MAP)
        })
    };
    let frame = Compositor::new()
        .render(&FramePacket {
            frame_index: 0,
            logical_size: SIZE,
            frame_rate: FrameRate::MAX,
            brightness: 255,
            bitmaps: vec![],
            tables: vec![map([0; 12]), table(FLASH, (2, 1, 1), vec![0x111, 0x0f3])],
            ops: vec![ground, op(Plane::Lower), op(Plane::Upper)],
        })
        .unwrap();
    assert_eq!(frame.pixel(0, 0), [108, 108, 108, 255]);
    assert_eq!(frame.pixel(31, 31), [108, 108, 108, 255]);
    assert_eq!(frame.pixel(32, 0), [100, 220, 124, 255]);
    // The flash table repeats by its own size.
    assert_eq!(frame.pixel(64, 40), [108, 108, 108, 255]);
}

/// A stream of the topics decodes as it was encoded: the ops are plain data.
#[test]
fn a_topic_survives_the_encoding() {
    use vkrgss_proto::codec::{decode_stream, encode_stream};

    let topic = scenes::topic("anim");
    let bytes = encode_stream(&topic.frames).unwrap();
    assert_eq!(decode_stream::<GuestToHost>(&bytes).unwrap(), topic.frames);
}
