//! Tilemap layer ops: the scenes of the reference compositor rendered by both backends, the
//! table errors, and the copy of the pattern table in the shader.

use vkxp_compose_ref::fixtures::{self, bitmap, table, tilemap_layer, tileset};
use vkxp_compose_ref::{self as reference, AUTOTILE_QUARTERS};
use vkxp_proto::{
    BitmapId, FramePacket, FrameRate, GuestToHost, Op, Rect, Size, TableDelta, TableId,
    TilemapLayer,
};

use super::differential::{Pair, compare_stream};
use super::harness::TestCompositor;
use crate::{TableError, VkError};

const COMPOSE_GLSL: &str = include_str!("../../shaders/compose.comp");

#[test]
fn every_tilemap_scene_is_byte_identical() {
    let scenes = fixtures::tilemap_scenes();
    assert!(
        scenes.len() >= 5,
        "the scene list shrank to {}",
        scenes.len()
    );
    let failures: Vec<String> = scenes
        .iter()
        .filter_map(|scene| {
            compare_stream(&scene.messages)
                .err()
                .map(|difference| format!("{}: {difference}", scene.name))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "scenes differ:\n{}",
        failures.join("\n")
    );
}

#[test]
fn shader_has_the_pattern_table_of_the_reference() {
    let start = COMPOSE_GLSL
        .find("const uint AUTOTILE_QUARTERS[192] = uint[192](")
        .expect("the shader declares the table");
    let body = &COMPOSE_GLSL[start..];
    let body = &body[body.find("](").unwrap() + 2..body.find(");").unwrap()];
    let found: Vec<u8> = body
        .split(',')
        .map(|number| number.trim().parse().expect("a quarter number"))
        .collect();
    assert_eq!(found, AUTOTILE_QUARTERS.concat());
}

const SIZE: Size = Size::new(96, 64);
const MAP: TableId = TableId::new(4);
const PRIORITIES: TableId = TableId::new(2);
const TILESET: BitmapId = BitmapId::new(0, 0);

fn packet(tables: Vec<TableDelta>, ops: Vec<TilemapLayer>) -> GuestToHost {
    GuestToHost::Frame(FramePacket {
        frame_index: 0,
        logical_size: SIZE,
        frame_rate: FrameRate::DEFAULT,
        tables,
        bitmaps: vec![],
        ops: ops.into_iter().map(Op::TilemapLayer).collect(),
    })
}

fn layer() -> TilemapLayer {
    tilemap_layer(SIZE, MAP, PRIORITIES)
}

// guest-host-protocol, Requirement: Table resources

#[test]
fn unknown_table_is_a_typed_error_and_no_frame() {
    let mut compositor = TestCompositor::new();
    let GuestToHost::Frame(frame) = packet(vec![table(MAP, (1, 1, 1), vec![0])], vec![layer()])
    else {
        unreachable!()
    };
    assert_eq!(
        compositor.render(&frame),
        Err(VkError::InvalidOpTable {
            index: 0,
            source: TableError::Unknown { id: PRIORITIES },
        })
    );
    assert_eq!(compositor.read_frame(), Ok(None));
}

/// Tables set, replaced by longer and shorter ones, destroyed and set again, with frames in
/// between and with deltas and ops the backends must refuse alike.
#[test]
fn tables_follow_their_deltas_like_the_reference() {
    let map = |cells: Vec<i16>| table(MAP, (cells.len() as u32, 1, 1), cells);
    let priorities = |entries: Vec<i16>| table(PRIORITIES, (entries.len() as u32, 1, 1), entries);
    let tiles = TilemapLayer {
        tileset: Some(TILESET),
        ..layer()
    };
    let first = GuestToHost::Frame(FramePacket {
        frame_index: 0,
        logical_size: SIZE,
        frame_rate: FrameRate::DEFAULT,
        tables: vec![priorities(vec![]), map(vec![384, 385, 390])],
        bitmaps: bitmap(TILESET, 256, 160, tileset(5)).to_vec(),
        ops: vec![Op::TilemapLayer(tiles)],
    });
    let steps = [
        first,
        // A table before the map grows: the map moves in the buffer.
        packet(vec![priorities(vec![0; 400])], vec![tiles]),
        packet(vec![map(vec![400, 384])], vec![tiles]),
        // Refused: the data is short. The delta before it stays applied.
        packet(
            vec![map(vec![391]), table(MAP, (2, 2, 1), vec![0; 3])],
            vec![tiles],
        ),
        packet(vec![], vec![tiles]),
        packet(vec![TableDelta::Destroy { id: PRIORITIES }], vec![tiles]),
        packet(vec![TableDelta::Destroy { id: PRIORITIES }], vec![]),
        packet(vec![priorities(vec![0; 385])], vec![tiles]),
        packet(vec![], vec![]),
    ];
    let mut pair = Pair::new();
    let results: Vec<bool> = steps
        .iter()
        .map(|step| pair.step(step).unwrap().is_ok())
        .collect();
    assert_eq!(
        results,
        [true, true, true, false, true, false, false, true, true]
    );
}

/// A clip that covers a few pixels of a few tiles of the binning grid, at every corner of
/// a cell: binning must not change what the op draws.
#[test]
fn clipped_tilemap_layers_match_the_reference() {
    let size = Size::new(70, 50);
    let cells: Vec<i16> = (0..6).map(|cell| 384 + cell * 7).collect();
    let tables = vec![
        table(MAP, (3, 2, 1), cells),
        table(PRIORITIES, (0, 1, 1), vec![]),
    ];
    let clips = [
        Rect::new(15, 15, 2, 2),
        Rect::new(-5, 30, 40, 3),
        Rect::new(31, 0, 2, 50),
        Rect::new(60, 40, 100, 100),
        Rect::new(200, 0, 5, 5),
    ];
    let ops = clips.map(|clip| {
        Op::TilemapLayer(TilemapLayer {
            clip,
            tileset: Some(TILESET),
            origin: (-9, 11),
            ..tilemap_layer(size, MAP, PRIORITIES)
        })
    });
    let frame = GuestToHost::Frame(FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        tables,
        bitmaps: bitmap(TILESET, 256, 160, tileset(5)).to_vec(),
        ops: ops.to_vec(),
    });
    let mut pair = Pair::new();
    let rendered: reference::Frame = pair.step(&frame).unwrap().unwrap().unwrap();
    assert_ne!(
        rendered.pixel(16, 16),
        [0, 0, 0, 255],
        "something was drawn"
    );
}
