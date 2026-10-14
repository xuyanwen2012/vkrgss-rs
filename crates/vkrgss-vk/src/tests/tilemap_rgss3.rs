//! Tilemap plane ops of the rule set of RGSS3: the topics of the measurement and generated
//! packets rendered by both backends.

use vkrgss_compose_ref::fixtures::tilemap_rgss2::tilemap_plane;
use vkrgss_compose_ref::fixtures::tilemap_rgss3 as scenes;
use vkrgss_compose_ref::fixtures::{bitmap, table};
use vkrgss_proto::{
    BitmapDelta, BitmapId, Color, Op, Plane, PlaneRules, Rect, Size, TableId, TilemapPlane,
};

use super::differential::Pair;
use super::harness::Rng;
use super::tilemap_rgss2::{noise, number, packet};

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

// Requirement: Tilemap planes of RGSS3 match the reference. Scenario "Topics of RGSS3".

#[test]
fn tables_by_flag_and_the_shadow_mask_are_byte_identical() {
    for name in ["tables2", "shadow3"] {
        assert_topic(name);
    }
}

#[test]
fn edges_under_tables_are_byte_identical() {
    assert_topic("edge1");
}

#[test]
fn the_random_scenes_of_rgss3_are_byte_identical() {
    assert_topic("rand4");
}

#[test]
fn every_topic_is_compared() {
    let mut compared = ["tables2", "shadow3", "edge1", "rand4"].to_vec();
    compared.sort_unstable();
    let mut all = scenes::TOPICS.to_vec();
    all.sort_unstable();
    assert_eq!(compared, all);
}

const MAP: TableId = TableId::new(3);
const FLAGS: TableId = TableId::new(1);
const FLASH: TableId = TableId::new(7);

/// The shapes of a floor autotile that have the lower edge.
const EDGE_SHAPES: [i16; 13] = [28, 29, 30, 31, 33, 38, 39, 40, 41, 43, 44, 45, 46];

/// An element of layer `layer` of a map: a tile number of any range, in the second layer
/// often a tile of A2 with the lower edge, and in the fourth layer and beyond any value.
fn element(random: &mut Rng, layer: u32) -> i16 {
    match (layer, random.below(2)) {
        (3.., 0) => random.below(16) as i16,
        (3.., _) => random.next_u32() as i16,
        (1, 0) => {
            let shape = EDGE_SHAPES[random.below(EDGE_SHAPES.len() as u32) as usize];
            2816 + 48 * random.below(32) as i16 + shape
        }
        _ => number(random),
    }
}

/// A flags entry: the bits 0x10 and 0x80 are common.
fn flag(random: &mut Rng) -> i16 {
    match random.below(8) {
        0 | 1 => 0,
        2 => 0x10,
        3 | 4 => 0x80,
        5 => 0x90,
        6 => -1,
        _ => random.next_u32() as i16,
    }
}

/// Scenario "Generated planes of RGSS3": maps of one to five layers, flags tables of any
/// size and none, elements of layer 3 of any value, numbers of every range, sheets of any
/// size and none, any clip, corner, first cell, animation step and flash level.
#[test]
fn generated_planes_of_rgss3_match_the_reference() {
    let size = Size::new(150, 110);
    let mut random = Rng::new(0x5eed_2026_1013);
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

        let (xsize, ysize) = match random.below(6) {
            0 => (0, random.below(3)),
            1 => (1, 1),
            _ => (1 + random.below(9), 1 + random.below(7)),
        };
        let zsize = 1 + random.below(5);
        let mut cells = Vec::new();
        for layer in 0..zsize {
            cells.extend((0..xsize * ysize).map(|_| element(&mut random, layer)));
        }
        let mut tables = vec![table(MAP, (xsize, ysize, zsize), cells)];
        let flags = (random.below(5) != 0).then(|| {
            let length = [0, 100, 3000, 4352, 8192][random.below(5) as usize];
            let entries = (0..length).map(|_| flag(&mut random));
            tables.push(table(FLAGS, (length, 1, 1), entries.collect()));
            FLAGS
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
            let plane = match random.below(3) {
                0 => Plane::Upper,
                _ => Plane::Lower,
            };
            let animation_step = match random.below(3) {
                0 => u64::from(random.next_u32()) << 32 | u64::from(random.next_u32()),
                _ => u64::from(random.below(13)),
            };
            ops.push(Op::TilemapPlane(TilemapPlane {
                clip,
                plane,
                rules: PlaneRules::Rgss3,
                corner,
                first_cell,
                animation_step,
                flash_level: random.next_u32() as u8,
                map: MAP,
                passages: flags,
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

/// Scenario "Clipped planes of RGSS3": clips that cover a few pixels of a few tiles of the
/// binning grid, over a map with tables, edges and shadows.
#[test]
fn clipped_planes_of_rgss3_match_the_reference() {
    let size = Size::new(200, 150);
    let sheet = BitmapId::new(0, 0);
    let mut random = Rng::new(7);
    let mut cells = Vec::new();
    for layer in 0..4 {
        cells.extend((0..6 * 5).map(|_| element(&mut random, layer)));
    }
    // A table with the lower edge over a cell with a shadow, at the map's corner: under
    // the first clip.
    (cells[30], cells[30 + 6], cells[90 + 6]) = (2844, 0, 15);
    let flags: Vec<i16> = (0..8192).map(|_| flag(&mut random)).collect();
    let flags = flags.into_iter().enumerate();
    let flags = flags.map(|(number, flag)| if number == 2844 { 0x80 } else { flag });
    let clips = [
        Rect::new(15, 40, 2, 9),
        Rect::new(-5, 30, 40, 3),
        Rect::new(31, 0, 3, 150),
        Rect::new(160, 100, 100, 100),
        Rect::new(400, 0, 5, 5),
        Rect::new(0, 0, 200, 150),
    ];
    let ops = clips.into_iter().enumerate().map(|(index, clip)| {
        Op::TilemapPlane(TilemapPlane {
            clip,
            rules: PlaneRules::Rgss3,
            passages: Some(FLAGS),
            corner: (-9 - 500 * (index as i32 / 5), 11 - 380 * (index as i32 / 5)),
            bitmaps: [Some(sheet); 9],
            ..tilemap_plane(Plane::Lower, MAP)
        })
    });
    let message = packet(
        size,
        bitmap(sheet, 512, 512, noise(&mut random, 512, 512)).to_vec(),
        vec![
            table(MAP, (6, 5, 4), cells),
            table(FLAGS, (8192, 1, 1), flags.collect()),
        ],
        ops.collect(),
    );
    let mut pair = Pair::new();
    let rendered = pair.step(&message).unwrap().unwrap().unwrap();
    assert_ne!(
        rendered.pixel(16, 44),
        [0, 0, 0, 255],
        "something was drawn"
    );
    // The last plane ends at column 163, 672 pixels from -509, and no other clip is here.
    assert_eq!(rendered.pixel(190, 20), [0, 0, 0, 255]);
}
