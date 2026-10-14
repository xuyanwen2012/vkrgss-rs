//! The scenarios of the tilemap plane op of the rule set of RGSS3 of
//! `specs/frame-composition/spec.md`.
//!
//! The topics are compared with digests of the frames that the model of the measurement
//! renders (`tests/data/tilemap-rgss3-digests.txt`: topic, frame, digest). Every other test
//! builds the small map of the scenarios, 4 by 3 cells and four layers, and derives its
//! expected pixels here from the patterns of the generated sheets or from the stated
//! arithmetic.

use vkrgss_compose_ref::fixtures::table;
use vkrgss_compose_ref::fixtures::tilemap_rgss2::{
    SIZE, Set, digest, sheet, sheet_pixel, tilemap_plane,
};
use vkrgss_compose_ref::fixtures::tilemap_rgss3 as scenes;
use vkrgss_compose_ref::{Compositor, Frame};
use vkrgss_pixel::Rgba;
use vkrgss_proto::{
    BitmapDelta, BitmapId, Color, FramePacket, FrameRate, Op, Plane, PlaneRules, Rect, TableDelta,
    TableId, TilemapPlane,
};

const EXPECTED: &str = include_str!("data/tilemap-rgss3-digests.txt");

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
        if digest(&frame.rgba) != expected {
            wrong.push(index);
        }
    }
    assert!(wrong.is_empty(), "frames of `{name}` differ: {wrong:?}");
}

// Requirement: Whole tilemaps of RGSS3 as measured. Scenario "Topics".

#[test]
fn tables_by_flag_are_drawn_as_measured() {
    assert_topic("tables2", 6);
}

#[test]
fn the_shadow_mask_is_drawn_as_measured() {
    assert_topic("shadow3", 22);
}

#[test]
fn edges_under_tables_are_drawn_as_measured() {
    assert_topic("edge1", 48);
}

#[test]
fn forty_random_scenes_are_drawn_as_measured() {
    assert_topic("rand4", 40);
}

/// Scenario "Nothing of the original": the file of digests holds only comments, topic
/// names, frame numbers and digests.
#[test]
fn the_file_of_digests_holds_only_topics_frames_and_digests() {
    let mut lines = 0;
    for line in EXPECTED.lines().filter(|line| !line.starts_with('#')) {
        let words: Vec<&str> = line.split(' ').collect();
        assert_eq!(words.len(), 3, "{line}");
        assert!(scenes::TOPICS.contains(&words[0]), "{line}");
        assert!(words[1].parse::<usize>().is_ok(), "{line}");
        assert_eq!(words[2].len(), 16, "{line}");
        assert!(u64::from_str_radix(words[2], 16).is_ok(), "{line}");
        lines += 1;
    }
    assert_eq!(lines, 6 + 22 + 48 + 40);
}

const MAP: TableId = TableId::new(0);
const FLAGS: TableId = TableId::new(1);
const FLASH: TableId = TableId::new(2);
const BLACK: Rgba = [0, 0, 0, 255];

/// The slots of the sheets.
const A1: usize = 0;
const A2: usize = 1;
const A3: usize = 2;
const A5: usize = 4;
const B: usize = 5;
const C: usize = 6;

/// The bitmap of sheet `index`.
const fn slot(index: usize) -> BitmapId {
    BitmapId::new(index as u32, 0)
}

/// A compositor that holds the nine sheets of one set.
struct Stage {
    compositor: Compositor,
    set: Set,
    frames: u64,
}

impl Stage {
    fn new(set: Set) -> Stage {
        let mut bitmaps: Vec<BitmapDelta> = Vec::new();
        for index in 0..9 {
            bitmaps.extend(sheet(slot(index), set, index));
        }
        let mut stage = Stage {
            compositor: Compositor::new(),
            set,
            frames: 0,
        };
        stage.packet(bitmaps, vec![], vec![]);
        stage
    }

    fn packet(
        &mut self,
        bitmaps: Vec<BitmapDelta>,
        tables: Vec<TableDelta>,
        ops: Vec<Op>,
    ) -> Frame {
        self.frames += 1;
        self.compositor
            .render(&FramePacket {
                frame_index: self.frames,
                logical_size: SIZE,
                frame_rate: FrameRate::MAX,
                brightness: 255,
                bitmaps,
                tables,
                ops,
            })
            .expect("no error is reported")
    }

    /// The frame of `planes` over a ground of one colour.
    fn render(
        &mut self,
        ground: Rgba,
        tables: Vec<TableDelta>,
        planes: Vec<TilemapPlane>,
    ) -> Frame {
        let mut ops = vec![Op::Clear {
            clip: Rect::from_size(SIZE),
            color: Color::from_rgba(ground),
        }];
        ops.extend(planes.into_iter().map(Op::TilemapPlane));
        self.packet(vec![], tables, ops)
    }

    /// The frame of one plane over black, the small map being `cells` and the flags table
    /// having the entries `flags`.
    fn small(&mut self, which: Plane, cells: &[Cell], flags: &[(i16, i16)]) -> Frame {
        self.render(
            BLACK,
            vec![map(cells), flags_table(flags)],
            vec![plane(which)],
        )
    }

    /// The pixel (`x`, `y`) of sheet `index`.
    fn texel(&self, index: usize, x: u32, y: u32) -> Rgba {
        sheet_pixel(self.set, index, x, y).to_rgba()
    }
}

/// An element of a map: ((x, y, layer), value).
type Cell = ((u32, u32, u32), i16);

/// A map of 4 by 3 cells and `layers` layers, all elements 0 except `cells`.
fn map_of(layers: u32, cells: &[Cell]) -> TableDelta {
    let mut data = vec![0; (12 * layers) as usize];
    for &((x, y, layer), value) in cells {
        data[(x + 4 * (y + 3 * layer)) as usize] = value;
    }
    table(MAP, (4, 3, layers), data)
}

/// The small map.
fn map(cells: &[Cell]) -> TableDelta {
    map_of(4, cells)
}

/// A flags table of 8192 elements, all 0 except the entries (number, flag) given.
fn flags_table(entries: &[(i16, i16)]) -> TableDelta {
    let mut data = vec![0; 8192];
    for &(number, flag) in entries {
        data[number as usize] = flag;
    }
    table(FLAGS, (8192, 1, 1), data)
}

/// A plane of the rule set of RGSS3 with every sheet and the flags table.
fn plane(which: Plane) -> TilemapPlane {
    let mut op = TilemapPlane {
        rules: PlaneRules::Rgss3,
        passages: Some(FLAGS),
        ..tilemap_plane(which, MAP)
    };
    for (index, bitmap) in op.bitmaps.iter_mut().enumerate() {
        *bitmap = Some(slot(index));
    }
    op
}

/// A colour under a shadow.
fn dark(mut pixel: Rgba) -> Rgba {
    for channel in &mut pixel[..3] {
        *channel = ((u16::from(*channel) * 127) >> 8) as u8;
    }
    pixel
}

/// Whether every pixel of the frame is `ground`.
fn is_ground(frame: &Frame, ground: Rgba) -> bool {
    frame
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| *pixel == ground)
}

// Requirement: Rule set of a tilemap plane op.

/// Scenario "Common numbering, measured on the original".
#[test]
fn whole_tiles_are_numbered_as_in_rgss2() {
    let mut stage = Stage::new(Set::Code);
    let frame = stage.small(Plane::Lower, &[((1, 1, 0), 1545), ((2, 1, 2), 37)], &[]);
    assert_eq!(frame.pixel(35, 40), stage.texel(A5, 35, 40));
    assert_eq!(frame.pixel(67, 40), stage.texel(B, 163, 136));
}

/// Scenario "Common animation, measured on the original".
#[test]
fn the_first_sheet_is_animated_as_in_rgss2() {
    let mut stage = Stage::new(Set::Code);
    for (step, x) in [(0, 35), (1, 99), (2, 163), (3, 99)] {
        let op = TilemapPlane {
            animation_step: step,
            ..plane(Plane::Lower)
        };
        let tables = vec![map(&[((1, 1, 0), 2048)]), flags_table(&[])];
        let frame = stage.render(BLACK, tables, vec![op]);
        assert_eq!(frame.pixel(35, 40), stage.texel(A1, x, 72), "step {step}");
    }
}

/// Scenario "Common blend, measured on the original".
#[test]
fn a_translucent_tile_is_blended_as_in_rgss2() {
    let mut stage = Stage::new(Set::Half);
    let tables = vec![map(&[((1, 1, 0), 1545)]), flags_table(&[])];
    let frame = stage.render([200, 200, 200, 255], tables, vec![plane(Plane::Lower)]);
    let texel = stage.texel(A5, 35, 40);
    assert_eq!(texel, [81, 24, 163, 128]);
    let blended = texel.map(|channel| ((u32::from(channel) * 129 + 200 * 128) >> 8) as u8);
    assert_eq!(blended[..3], [140, 112, 182]);
    assert_eq!(frame.pixel(35, 40), [140, 112, 182, 255]);
}

/// Scenario "Common flash, measured on the original".
#[test]
fn both_planes_add_the_flash_as_in_rgss2() {
    let mut stage = Stage::new(Set::Code);
    let mut entries = vec![0; 12];
    entries[0] = 0x111;
    let op = |which| TilemapPlane {
        flash: Some(FLASH),
        flash_level: 4,
        ..plane(which)
    };
    let tables = vec![map(&[]), flags_table(&[]), table(FLASH, (4, 3, 1), entries)];
    let planes = vec![op(Plane::Lower), op(Plane::Upper)];
    let frame = stage.render([100, 100, 100, 255], tables, planes);
    assert_eq!(frame.pixel(4, 4), [108, 108, 108, 255]);
    assert_eq!(frame.pixel(40, 4), [100, 100, 100, 255]);
}

// Requirement: Tables of RGSS3.

/// Scenario "A table by its flag, measured on the original".
#[test]
fn a_tile_of_a2_with_the_flag_is_a_table() {
    let mut stage = Stage::new(Set::Code);
    let frame = stage.small(Plane::Lower, &[((1, 0, 0), 2844)], &[(2844, 0x80)]);
    let expected = [
        ((35, 4), (35, 68)),
        ((35, 16), (35, 48)),
        ((52, 23), (20, 55)),
        ((35, 24), (35, 80)),
        ((52, 31), (20, 87)),
        ((35, 32), (35, 88)),
        ((52, 39), (20, 95)),
    ];
    for ((x, y), (sx, sy)) in expected {
        assert_eq!(frame.pixel(x, y), stage.texel(A2, sx, sy), "({x}, {y})");
    }
    assert_eq!(frame.pixel(35, 40), BLACK);
}

/// Scenario "Without the flag, measured on the original".
#[test]
fn without_the_flag_the_tile_is_a_floor() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 0, 0), 2844)];
    let unflagged = stage.small(Plane::Lower, &cells, &[]);
    let op = TilemapPlane {
        passages: None,
        ..plane(Plane::Lower)
    };
    let no_table = stage.render(BLACK, vec![map(&cells)], vec![op]);
    // The number is not below the x size of the table.
    let short = table(FLAGS, (100, 1, 1), vec![0x80; 100]);
    let short = stage.render(BLACK, vec![map(&cells), short], vec![plane(Plane::Lower)]);
    for frame in [unflagged, no_table, short] {
        assert_eq!(frame.pixel(35, 16), stage.texel(A2, 35, 80));
        assert_eq!(frame.pixel(35, 24), stage.texel(A2, 35, 88));
        assert_eq!(frame.pixel(35, 32), BLACK);
    }
}

/// Scenario "Column 7 is no table, measured on the original".
#[test]
fn the_last_column_of_a2_is_no_table_by_itself() {
    let mut stage = Stage::new(Set::Code);
    let frame = stage.small(Plane::Lower, &[((1, 0, 0), 3180)], &[]);
    assert_eq!(frame.pixel(35, 16), stage.texel(A2, 483, 80));
    assert_eq!(frame.pixel(35, 24), stage.texel(A2, 483, 88));
    assert_eq!(frame.pixel(35, 32), BLACK);
}

/// Scenario "The flag on another sheet, measured on the original".
#[test]
fn the_flag_does_nothing_on_another_sheet() {
    let mut stage = Stage::new(Set::Code);
    let frame = stage.small(Plane::Lower, &[((1, 0, 0), 4380)], &[(4380, 0x80)]);
    assert_eq!(frame.pixel(35, 16), stage.texel(A3, 3, 0));
    assert_eq!(frame.pixel(35, 24), stage.texel(A3, 3, 8));
    assert_eq!(frame.pixel(35, 32), BLACK);
}

/// Scenario "A table in layer 2, measured on the original".
#[test]
fn a_table_of_the_third_layer_is_a_table() {
    let mut stage = Stage::new(Set::Code);
    let frame = stage.small(Plane::Lower, &[((1, 0, 2), 2844)], &[(2844, 0x80)]);
    assert_eq!(frame.pixel(35, 16), stage.texel(A2, 35, 48));
    assert_eq!(frame.pixel(35, 24), stage.texel(A2, 35, 80));
    assert_eq!(frame.pixel(35, 32), stage.texel(A2, 35, 88));
}

/// Scenario "Band of a table, measured on the original".
#[test]
fn a_table_reaches_below_its_cell_up_to_the_next_multiple_of_8() {
    let mut stage = Stage::new(Set::Code);
    let op = TilemapPlane {
        corner: (0, -3),
        ..plane(Plane::Lower)
    };
    let tables = vec![map(&[((1, 0, 0), 2844)]), flags_table(&[(2844, 0x80)])];
    let frame = stage.render(BLACK, tables, vec![op]);
    assert_eq!(frame.pixel(35, 28), stage.texel(A2, 35, 87));
    assert_eq!(frame.pixel(35, 29), stage.texel(A2, 35, 88));
    assert_eq!(frame.pixel(35, 31), stage.texel(A2, 35, 90));
    assert_eq!(frame.pixel(35, 32), BLACK);
}

// Requirement: Edge under a table of RGSS3.

const TABLE: [(i16, i16); 1] = [(2844, 0x80)];

/// Scenario "The edge, measured on the original".
#[test]
fn a_table_of_the_second_layer_gives_the_cell_below_its_edge() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 0, 1), 2844), ((1, 1, 0), 1545)];
    let frame = stage.small(Plane::Lower, &cells, &TABLE);
    assert_eq!(frame.pixel(35, 32), stage.texel(A2, 35, 88));
    assert_eq!(frame.pixel(52, 39), stage.texel(A2, 20, 95));
    assert_eq!(frame.pixel(35, 40), stage.texel(A5, 35, 40));
}

/// Scenario "Tables of layers 0 and 2 give none, measured on the original".
#[test]
fn tables_of_the_first_and_third_layer_give_no_edge() {
    let mut stage = Stage::new(Set::Code);
    for layer in [0, 2] {
        let cells = [((1, 0, layer), 2844), ((1, 1, 0), 1545)];
        let frame = stage.small(Plane::Lower, &cells, &TABLE);
        assert_eq!(frame.pixel(35, 32), stage.texel(A5, 35, 32), "{layer}");
        assert_eq!(frame.pixel(52, 39), stage.texel(A5, 52, 39), "{layer}");
    }
}

/// Scenario "A shape without the lower edge, measured on the original".
#[test]
fn a_shape_without_the_lower_edge_gives_no_edge() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 0, 1), 2816), ((1, 1, 0), 1545)];
    let frame = stage.small(Plane::Lower, &cells, &[(2816, 0x80)]);
    assert_eq!(frame.pixel(35, 32), stage.texel(A5, 35, 32));
}

/// Scenario "The tile of layer 1 of the cell, measured on the original".
#[test]
fn a_table_in_the_second_layer_of_the_cell_takes_no_edge() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 0, 1), 2844), ((1, 1, 0), 1545), ((1, 1, 1), 2844)];
    let frame = stage.small(Plane::Lower, &cells, &TABLE);
    assert_eq!(frame.pixel(35, 32), stage.texel(A2, 35, 64));
    let cells = [((1, 0, 1), 2844), ((1, 1, 0), 1545), ((1, 1, 1), 2864)];
    let frame = stage.small(Plane::Lower, &cells, &TABLE);
    assert_eq!(frame.pixel(35, 32), stage.texel(A2, 35, 88));
    assert_eq!(frame.pixel(35, 40), stage.texel(A2, 99, 72));
}

/// Scenario "The tile of layer 0 of the cell, measured on the original".
#[test]
fn a_wall_in_the_first_layer_of_the_cell_takes_no_edge() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 0, 1), 2844), ((1, 1, 0), 4352)];
    let frame = stage.small(Plane::Lower, &cells, &TABLE);
    assert_eq!(frame.pixel(35, 32), stage.texel(A3, 35, 32));
    // A table there takes it.
    let cells = [((1, 0, 1), 2844), ((1, 1, 0), 2892)];
    let frame = stage.small(Plane::Lower, &cells, &[(2844, 0x80), (2892, 0x80)]);
    assert_eq!(frame.pixel(35, 32), stage.texel(A2, 35, 88));
}

/// Scenario "Layer 2 covers the edge, measured on the original".
#[test]
fn the_third_layer_covers_the_edge() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 0, 1), 2844), ((1, 1, 0), 1545), ((1, 1, 2), 37)];
    let frame = stage.small(Plane::Lower, &cells, &TABLE);
    assert_eq!(frame.pixel(35, 32), stage.texel(B, 163, 128));
}

/// Scenario "An empty cell takes the edge, measured on the original".
#[test]
fn an_empty_cell_takes_the_edge() {
    let mut stage = Stage::new(Set::Code);
    let frame = stage.small(Plane::Lower, &[((1, 0, 1), 2844)], &TABLE);
    assert_eq!(frame.pixel(35, 32), stage.texel(A2, 35, 88));
    assert_eq!(frame.pixel(52, 39), stage.texel(A2, 20, 95));
    assert_eq!(frame.pixel(35, 40), BLACK);
}

/// Scenario "No look-up across the edge of the map, measured on the original".
#[test]
fn the_cell_above_is_not_looked_up_across_the_edge_of_the_map() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 2, 1), 2844), ((1, 0, 0), 1545)];
    let frame = stage.small(Plane::Lower, &cells, &TABLE);
    assert_eq!(frame.pixel(35, 0), stage.texel(A5, 35, 32));
    assert_eq!(frame.pixel(35, 96), stage.texel(A5, 35, 32));
    assert_eq!(frame.pixel(35, 95), stage.texel(A2, 35, 87));
}

/// Scenario "A star table in layer 1, measured on the original".
#[test]
fn a_star_table_of_the_second_layer_is_drawn_below_with_its_edge() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 0, 1), 2844), ((1, 1, 0), 1545)];
    let frame = stage.small(Plane::Lower, &cells, &[(2844, 0x90)]);
    assert_eq!(frame.pixel(35, 4), stage.texel(A2, 35, 68));
    assert_eq!(frame.pixel(35, 32), stage.texel(A2, 35, 88));
}

// Requirement: Shadow mask of RGSS3.

/// Scenario "Quarters, measured on the original".
#[test]
fn a_bit_of_the_fourth_layer_darkens_its_quarter() {
    let mut stage = Stage::new(Set::Code);
    let (light, shade) = ([200, 200, 200, 255], [99, 99, 99, 255]);
    let cells = [((1, 1, 3), -1), ((2, 1, 3), 1), ((3, 1, 3), 8)];
    let tables = vec![map(&cells), flags_table(&[])];
    let frame = stage.render(light, tables.clone(), vec![plane(Plane::Lower)]);
    for y in 32..64 {
        for x in 32..64 {
            assert_eq!(frame.pixel(x, y), shade, "({x}, {y})");
        }
    }
    assert_eq!(frame.pixel(64, 32), shade);
    assert_eq!(frame.pixel(80, 32), light);
    assert_eq!(frame.pixel(64, 48), light);
    assert_eq!(frame.pixel(127, 63), shade);
    assert_eq!(frame.pixel(112, 47), light);
    assert_eq!(frame.pixel(111, 48), light);
    assert_eq!(frame.pixel(35, 31), light);
    // The upper plane draws no shadow.
    let frame = stage.render(light, tables, vec![plane(Plane::Upper)]);
    assert!(is_ground(&frame, light));
}

/// Scenario "Low bits only, measured on the original".
#[test]
fn only_the_four_low_bits_of_the_fourth_layer_darken() {
    let mut stage = Stage::new(Set::Code);
    let mut frame = |mask| {
        let cells = [((1, 1, 0), 1545), ((1, 1, 3), mask)];
        stage.small(Plane::Lower, &cells, &[])
    };
    let (left, right, plain) = (frame(5), frame(0x3f0a), frame(16));
    let texel = |x, y| stage.texel(A5, x, y);
    assert_eq!(left.pixel(35, 40), [40, 11, 80, 255]);
    assert_eq!(left.pixel(35, 56), [40, 11, 112, 255]);
    assert_eq!(left.pixel(52, 40), texel(52, 40));
    assert_eq!(left.pixel(52, 56), texel(52, 56));
    assert_eq!(right.pixel(52, 40), [40, 79, 80, 255]);
    assert_eq!(right.pixel(52, 56), [40, 79, 112, 255]);
    assert_eq!(right.pixel(35, 40), texel(35, 40));
    assert_eq!(right.pixel(35, 56), texel(35, 56));
    for y in 32..64 {
        for x in 32..64 {
            assert_eq!(plain.pixel(x, y), texel(x, y), "({x}, {y})");
            let shaded = if x < 48 {
                left.pixel(x, y)
            } else {
                right.pixel(x, y)
            };
            assert_eq!(shaded, dark(texel(x, y)), "({x}, {y})");
        }
    }
}

/// Scenario "Layer 1 is darkened and layer 2 is not, measured on the original".
#[test]
fn the_shadow_lies_between_the_second_and_the_third_layer() {
    let mut stage = Stage::new(Set::Code);
    assert_eq!(stage.texel(B, 163, 136), [101, 26, 35, 255]);
    let frame = stage.small(Plane::Lower, &[((1, 1, 3), 15), ((1, 1, 1), 37)], &[]);
    assert_eq!(frame.pixel(35, 40), [50, 12, 17, 255]);
    let frame = stage.small(Plane::Lower, &[((1, 1, 3), 15), ((1, 1, 2), 37)], &[]);
    assert_eq!(frame.pixel(35, 40), stage.texel(B, 163, 136));
}

/// Scenario "The edge is not darkened, measured on the original".
#[test]
fn the_edge_is_drawn_after_the_shadow() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 0, 1), 2844), ((1, 1, 0), 1545), ((1, 1, 3), 15)];
    let frame = stage.small(Plane::Lower, &cells, &TABLE);
    assert_eq!(frame.pixel(35, 32), stage.texel(A2, 35, 88));
    assert_eq!(frame.pixel(35, 39), stage.texel(A2, 35, 95));
    assert_eq!(frame.pixel(35, 40), [40, 11, 80, 255]);
}

/// Scenario "No shadow from walls, measured on the original".
#[test]
fn walls_give_no_shadow() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((0, 0, 0), 4367), ((0, 1, 0), 4367), ((1, 1, 0), 2912)];
    let frame = stage.small(Plane::Lower, &cells, &[]);
    assert_eq!(frame.pixel(35, 40), stage.texel(A2, 163, 72));
    assert_eq!(frame.pixel(52, 40), stage.texel(A2, 148, 72));
}

// Requirement: Star tiles of RGSS3.

/// Scenario "A star tile in a lower layer, measured on the original".
#[test]
fn a_star_tile_of_a_lower_layer_is_drawn_by_the_lower_plane() {
    let mut stage = Stage::new(Set::Code);
    for layer in [0, 1] {
        let cells = [((1, 1, layer), 300)];
        let lower = stage.small(Plane::Lower, &cells, &[(300, 0x10)]);
        assert_eq!(lower.pixel(35, 40), stage.texel(C, 131, 168), "{layer}");
        let upper = stage.small(Plane::Upper, &cells, &[(300, 0x10)]);
        assert!(is_ground(&upper, BLACK), "layer {layer}");
    }
}

/// Scenario "A star tile in layer 2, measured on the original".
#[test]
fn a_star_tile_of_the_third_layer_is_drawn_by_the_upper_plane() {
    let mut stage = Stage::new(Set::Code);
    let cells = [((1, 1, 2), 300)];
    let upper = stage.small(Plane::Upper, &cells, &[(300, 0x10)]);
    assert_eq!(upper.pixel(35, 40), stage.texel(C, 131, 168));
    let lower = stage.small(Plane::Lower, &cells, &[(300, 0x10)]);
    assert!(is_ground(&lower, BLACK));
}

/// Scenario "A star table on the upper plane, measured on the original".
#[test]
fn a_star_table_of_the_third_layer_is_a_table_on_the_upper_plane() {
    let mut stage = Stage::new(Set::Code);
    let mut frame = |which| {
        let op = TilemapPlane {
            corner: (0, -3),
            ..plane(which)
        };
        let tables = vec![map(&[((1, 0, 2), 2844)]), flags_table(&[(2844, 0x90)])];
        stage.render(BLACK, tables, vec![op])
    };
    let (upper, lower) = (frame(Plane::Upper), frame(Plane::Lower));
    assert_eq!(upper.pixel(35, 4), stage.texel(A2, 35, 71));
    assert_eq!(upper.pixel(35, 29), stage.texel(A2, 35, 88));
    assert_eq!(upper.pixel(35, 32), BLACK);
    assert!(is_ground(&lower, BLACK));
}

/// Scenario "The flag of number 0, measured on the original".
#[test]
fn the_flag_of_number_0_does_nothing() {
    let mut stage = Stage::new(Set::Code);
    for which in [Plane::Lower, Plane::Upper] {
        let frame = stage.small(which, &[], &[(0, 0x10)]);
        assert!(is_ground(&frame, BLACK), "{which:?}");
    }
}

/// Scenario "Other bits, measured on the original".
#[test]
fn no_other_bit_of_a_flag_changes_the_frame() {
    let mut stage = Stage::new(Set::Code);
    let scenes: [(Plane, Cell); 3] = [
        (Plane::Lower, ((1, 0, 0), 2844)),
        (Plane::Upper, ((1, 1, 2), 300)),
        (Plane::Lower, ((1, 1, 2), 300)),
    ];
    for (which, cell) in scenes {
        let plain = stage.small(which, &[cell], &[]);
        for flag in [0x6f, 0x100, 0x7f6f] {
            let frame = stage.small(which, &[cell], &[(cell.1, flag)]);
            assert_eq!(frame, plain, "{which:?} {flag:#x}");
        }
    }
}

// Requirement: Undefined maps and numbers under the rule set of RGSS3.

/// Scenario "Undefined numbers".
#[test]
fn undefined_numbers_draw_nothing() {
    let mut stage = Stage::new(Set::Code);
    let cells = [
        ((0, 0, 0), 1024),
        ((1, 0, 0), 1535),
        ((2, 0, 0), -256),
        ((3, 0, 0), -1),
        ((0, 1, 0), 1300),
    ];
    for which in [Plane::Lower, Plane::Upper] {
        let frame = stage.small(which, &cells, &[]);
        assert!(is_ground(&frame, BLACK), "{which:?}");
    }
}

/// Scenario "Fewer layers".
#[test]
fn a_layer_the_map_lacks_is_empty() {
    let mut stage = Stage::new(Set::Code);
    let light = [200, 200, 200, 255];
    // Three layers, one layer, and a table of two dimensions, which has one layer.
    for layers in [3, 1, 1] {
        let tables = vec![map_of(layers, &[((1, 1, 0), 1545)]), flags_table(&[])];
        let frame = stage.render(light, tables, vec![plane(Plane::Lower)]);
        assert_eq!(frame.pixel(35, 40), stage.texel(A5, 35, 40), "{layers}");
        for y in 0..96 {
            for x in 0..128 {
                let cell = (32..64).contains(&x) && (32..64).contains(&y);
                let expected = if cell { stage.texel(A5, x, y) } else { light };
                assert_eq!(frame.pixel(x, y), expected, "({x}, {y}) of {layers}");
            }
        }
    }
}

/// Scenario "A fifth layer".
#[test]
fn a_fifth_layer_is_not_read() {
    let mut stage = Stage::new(Set::Code);
    let light = [200, 200, 200, 255];
    let cells = [((1, 1, 0), 1545), ((1, 1, 4), 37), ((2, 1, 4), 15)];
    let tables = vec![map_of(5, &cells), flags_table(&[])];
    let frame = stage.render(light, tables, vec![plane(Plane::Lower)]);
    assert_eq!(frame.pixel(35, 40), stage.texel(A5, 35, 40));
    for y in 32..64 {
        for x in 64..96 {
            assert_eq!(frame.pixel(x, y), light, "({x}, {y})");
        }
    }
}
