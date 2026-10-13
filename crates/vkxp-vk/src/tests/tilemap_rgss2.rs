//! Tilemap plane ops of RGSS2: the topics of the measurement and generated packets rendered
//! by both backends, the errors, and the copies of the shape tables in the shader.

use vkxp_compose_ref::fixtures::tilemap_rgss2::{self as scenes, tilemap_plane};
use vkxp_compose_ref::fixtures::{bitmap, table};
use vkxp_proto::tiles_rgss2::{FALL_SHAPES, FLOOR_SHAPES, WALL_SHAPES};
use vkxp_proto::{
    BitmapDelta, BitmapId, Color, FramePacket, FrameRate, GuestToHost, Op, Plane, PlaneRules, Rect,
    Size, TableDelta, TableId, TilemapPlane,
};

use super::differential::Pair;
use super::harness::{Rng, TestCompositor};
use crate::{BitmapError, SlotState, TableError, VkError};

const COMPOSE_GLSL: &str = include_str!("../../shaders/compose.comp");

/// Every frame of a topic through both backends.
fn assert_topic(name: &str) {
    let topic = scenes::topic(name);
    assert!(!topic.frames.is_empty());
    let mut pair = Pair::new();
    for (index, frame) in topic.frames.iter().enumerate() {
        match pair.step(frame) {
            Ok(Ok(Some(_))) => {}
            Ok(other) => panic!("frame {index} of `{name}`: {other:?}"),
            Err(difference) => panic!("frame {index} of `{name}`: {difference}"),
        }
    }
}

// Requirement: Tiles of a tilemap plane of RGSS2: CPU reference and GPU agree bit for bit.

#[test]
fn every_number_and_shape_is_byte_identical() {
    assert_topic("ids");
}

#[test]
fn the_animation_steps_are_byte_identical() {
    assert_topic("anim");
}

#[test]
fn tables_and_shadows_are_byte_identical() {
    for name in ["tables1", "shadow1", "shadow2"] {
        assert_topic(name);
    }
}

#[test]
fn planes_among_sprites_and_scrolled_maps_are_byte_identical() {
    for name in ["z1", "scroll1"] {
        assert_topic(name);
    }
}

#[test]
fn translucent_tiles_and_the_flash_are_byte_identical() {
    for name in ["alpha1", "flash1"] {
        assert_topic(name);
    }
}

// Requirement: Whole tilemaps of RGSS2 as measured. Scenario "Random scenes".

#[test]
fn the_random_scenes_are_byte_identical() {
    assert_topic("rand1");
}

#[test]
fn every_topic_is_compared() {
    let compared = [
        "ids", "anim", "tables1", "shadow1", "shadow2", "z1", "scroll1", "alpha1", "flash1",
        "rand1",
    ];
    let mut compared = compared.to_vec();
    compared.sort_unstable();
    let mut all = scenes::TOPICS.to_vec();
    all.sort_unstable();
    assert_eq!(compared, all);
}

/// The numbers of the table `name` of the shader.
fn shader_table(name: &str) -> Vec<u8> {
    let start = COMPOSE_GLSL
        .find(&format!("const uint {name}["))
        .unwrap_or_else(|| panic!("the shader declares {name}"));
    let body = &COMPOSE_GLSL[start..];
    let body = &body[body.find("](").unwrap() + 2..body.find(");").unwrap()];
    body.split(',')
        .map(|number| number.trim().parse().expect("a coordinate"))
        .collect()
}

#[test]
fn shader_has_the_shared_shape_tables() {
    let flat = |shapes: &[[(u8, u8); 4]]| -> Vec<u8> {
        let pieces = shapes.iter().flatten();
        pieces.flat_map(|&(x, y)| [x, y]).collect()
    };
    assert_eq!(shader_table("FLOOR_SHAPES"), flat(&FLOOR_SHAPES));
    assert_eq!(shader_table("WALL_SHAPES"), flat(&WALL_SHAPES));
    assert_eq!(shader_table("FALL_SHAPES"), flat(&FALL_SHAPES));
}

const MAP: TableId = TableId::new(3);
const PASSAGES: TableId = TableId::new(1);
const FLASH: TableId = TableId::new(7);

fn packet(
    size: Size,
    bitmaps: Vec<BitmapDelta>,
    tables: Vec<TableDelta>,
    ops: Vec<Op>,
) -> GuestToHost {
    GuestToHost::Frame(FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::MAX,
        brightness: 255,
        bitmaps,
        tables,
        ops,
    })
}

/// Pixels that differ from texel to texel, with every alpha.
fn noise(random: &mut Rng, width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width * height {
        let [red, green, blue, alpha] = random.next_u32().to_le_bytes();
        // A third opaque, a sixth transparent.
        let alpha = match alpha % 6 {
            0 | 1 => 255,
            2 => 0,
            _ => alpha,
        };
        rgba.extend([red, green, blue, alpha]);
    }
    rgba
}

/// A tile number of any range, the undefined ones and those beyond the sheets among them.
fn number(random: &mut Rng) -> i16 {
    match random.below(12) {
        0 => 0,
        1 => random.next_u32() as i16,
        2 => 1 + random.below(1023) as i16,
        3 => 1536 + random.below(512) as i16,
        4 | 5 => 2048 + random.below(768) as i16,
        // Tables, of which a few shapes reach below their cell.
        6 | 7 => 2816 + 48 * (8 * random.below(4) + 7) as i16 + random.below(48) as i16,
        8 => 2816 + random.below(1536) as i16,
        9 => 4352 + random.below(1536) as i16,
        _ => 5888 + random.below(2304) as i16,
    }
}

/// Generated packets: maps of any size and number of layers, sheets of any size and none,
/// passages and flash tables of any size and none, planes anywhere with any clip, first
/// cell, animation step and flash level, over a ground of any alpha.
#[test]
fn generated_planes_match_the_reference() {
    let size = Size::new(150, 110);
    let mut random = Rng::new(0x5eed_2026_1012);
    let mut pair = Pair::new();
    for round in 0..120u32 {
        // Destroyed and created anew every round, with another generation.
        let mut bitmaps = Vec::new();
        let mut sheets = [None; 9];
        for (slot, sheet) in sheets.iter_mut().enumerate() {
            let id = |round| BitmapId::new(slot as u32, round);
            if round > 0 {
                bitmaps.push(BitmapDelta::Destroy { id: id(round - 1) });
            }
            let (width, height) = match random.below(6) {
                0 => (0, random.below(40)),
                1 => (1 + random.below(40), 1 + random.below(40)),
                _ => (64 + random.below(200), 64 + random.below(200)),
            };
            let rgba = noise(&mut random, width, height);
            bitmaps.extend(bitmap(id(round), width, height, rgba));
            // A slot in six goes without its sheet.
            *sheet = (random.below(6) != 0).then(|| id(round));
        }

        let (xsize, ysize) = match random.below(5) {
            0 => (0, random.below(3)),
            1 => (1, 1),
            _ => (1 + random.below(9), 1 + random.below(7)),
        };
        let zsize = random.below(5);
        let cells = (0..xsize * ysize * zsize).map(|_| number(&mut random));
        let mut tables = vec![table(MAP, (xsize, ysize, zsize), cells.collect())];
        let passages = (random.below(4) != 0).then(|| {
            let length = [0, 100, 3000, 8192][random.below(4) as usize];
            let flags = (0..length).map(|_| [0, 0, 0x10, -1][random.below(4) as usize]);
            tables.push(table(PASSAGES, (length, 1, 1), flags.collect()));
            PASSAGES
        });
        let flash = (random.below(3) != 0).then(|| {
            let (width, height) = (random.below(6), 1 + random.below(5));
            let entries = (0..width * height).map(|_| match random.below(4) {
                0 => 0,
                1 => random.next_u32() as i16,
                _ => random.below(4096) as i16,
            });
            tables.push(table(FLASH, (width, height, 1), entries.collect()));
            FLASH
        });

        let mut ops = vec![Op::Clear {
            clip: Rect::from_size(size),
            color: Color::from_rgba(random.next_u32().to_le_bytes()),
        }];
        for _ in 0..1 + random.below(3) {
            let near = |random: &mut Rng, length: u32| random.below(length + 80) as i32 - 60;
            let corner = match random.below(8) {
                0 => (random.next_u32() as i32, random.next_u32() as i32),
                1 => (near(&mut random, 150) - 600, near(&mut random, 110) - 450),
                _ => (near(&mut random, 150), near(&mut random, 110)),
            };
            let clip = match random.below(3) {
                0 => Rect::from_size(size),
                1 => Rect::new(random.next_u32() as i32 >> 20, -5, 4000, random.below(300)),
                _ => Rect::new(
                    near(&mut random, 150),
                    near(&mut random, 110),
                    random.below(130),
                    random.below(100),
                ),
            };
            let first_cell = match random.below(3) {
                0 => (random.next_u32(), random.next_u32()),
                _ => (random.below(12), random.below(12)),
            };
            let plane = match random.below(2) {
                0 => Plane::Lower,
                _ => Plane::Upper,
            };
            let animation_step = match random.below(3) {
                0 => u64::from(random.next_u32()) << 32 | u64::from(random.next_u32()),
                _ => u64::from(random.below(13)),
            };
            ops.push(Op::TilemapPlane(TilemapPlane {
                clip,
                plane,
                rules: PlaneRules::Rgss2,
                corner,
                first_cell,
                animation_step,
                flash_level: random.next_u32() as u8,
                map: MAP,
                passages,
                flash,
                bitmaps: sheets,
            }));
        }
        let message = packet(size, bitmaps, tables, ops);
        match pair.step(&message) {
            Ok(Ok(Some(_))) => {}
            Ok(other) => panic!("round {round}: {other:?}"),
            Err(difference) => panic!("round {round}: {difference}"),
        }
    }
}

/// A clip that covers a few pixels of a few tiles of the binning grid, and planes that
/// begin and end inside the screen: binning must not change what the op draws.
#[test]
fn clipped_planes_match_the_reference() {
    let size = Size::new(200, 150);
    let sheet = BitmapId::new(0, 0);
    let mut random = Rng::new(7);
    let cells: Vec<i16> = (0..6 * 5 * 3).map(|_| number(&mut random)).collect();
    let clips = [
        Rect::new(15, 15, 2, 2),
        Rect::new(-5, 30, 40, 3),
        Rect::new(31, 0, 3, 150),
        Rect::new(160, 100, 100, 100),
        Rect::new(400, 0, 5, 5),
        Rect::new(0, 0, 200, 150),
    ];
    let ops = clips.into_iter().enumerate().map(|(index, clip)| {
        Op::TilemapPlane(TilemapPlane {
            clip,
            corner: (-9 - 500 * (index as i32 / 5), 11 - 380 * (index as i32 / 5)),
            bitmaps: [Some(sheet); 9],
            ..tilemap_plane(Plane::Lower, MAP)
        })
    });
    let message = packet(
        size,
        bitmap(sheet, 512, 512, noise(&mut random, 512, 512)).to_vec(),
        vec![table(MAP, (6, 5, 3), cells)],
        ops.collect(),
    );
    let mut pair = Pair::new();
    let rendered = pair.step(&message).unwrap().unwrap().unwrap();
    assert_ne!(
        rendered.pixel(16, 16),
        [0, 0, 0, 255],
        "something was drawn"
    );
    // The last plane ends at column 163, 672 pixels from -509, and no other clip is here.
    assert_eq!(rendered.pixel(190, 20), [0, 0, 0, 255]);
}

// Requirement: Errors match the reference.

#[test]
fn the_tables_and_sheets_of_a_plane_are_checked_like_the_reference_checks_them() {
    let size = Size::new(64, 48);
    let sheet = BitmapId::new(2, 0);
    let plane = |change: fn(&mut TilemapPlane)| {
        let mut plane = tilemap_plane(Plane::Lower, MAP);
        change(&mut plane);
        packet(size, vec![], vec![], vec![Op::TilemapPlane(plane)])
    };
    let setup = packet(
        size,
        bitmap(sheet, 4, 4, vec![9; 64]).to_vec(),
        vec![table(MAP, (1, 1, 3), vec![1, 0, 0])],
        vec![],
    );
    let steps = [
        setup,
        plane(|_| {}),
        plane(|plane| plane.map = PASSAGES),
        plane(|plane| plane.passages = Some(PASSAGES)),
        plane(|plane| plane.flash = Some(FLASH)),
        plane(|plane| plane.bitmaps[8] = Some(BitmapId::new(2, 1))),
        // The sheets are looked at before the tables, and a plane beside the screen is
        // checked all the same.
        plane(|plane| {
            plane.bitmaps[0] = Some(BitmapId::new(30, 0));
            plane.map = FLASH;
            plane.clip = Rect::new(900, 900, 1, 1);
        }),
        plane(|plane| plane.bitmaps[5] = Some(BitmapId::new(2, 0))),
    ];
    let mut pair = Pair::new();
    let results: Vec<bool> = steps
        .iter()
        .map(|step| pair.step(step).unwrap().is_ok())
        .collect();
    assert_eq!(
        results,
        [true, true, false, false, false, false, false, true]
    );

    let mut compositor = TestCompositor::new();
    let mut unknown = |message: &GuestToHost| {
        let GuestToHost::Frame(frame) = message else {
            unreachable!()
        };
        compositor.render(frame)
    };
    assert_eq!(
        unknown(&steps[1]),
        Err(VkError::InvalidOpTable {
            index: 0,
            source: TableError::Unknown { id: MAP },
        })
    );
    assert_eq!(
        unknown(&steps[6]),
        Err(VkError::InvalidOpBitmap {
            index: 0,
            source: BitmapError::InvalidReference {
                id: BitmapId::new(30, 0),
                slot: SlotState::Empty,
            },
        })
    );
}
