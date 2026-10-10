use vkxp_compose_ref::Frame;
use vkxp_pixel::Rgba;
use vkxp_proto::FramePacket;

use super::*;
use crate::bitmap::testing::color;
use crate::extract::testing::*;
use crate::{SpriteId, TilemapId};
use vkxp_proto::tiles::AUTOTILE_QUARTERS;

/// Number of tiles of the tileset of the tests: 8 columns, 40 rows.
const TILES: i32 = 320;

/// The pixel at (`x`, `y`) of the tileset: it names its tile and its place in the tile.
fn tileset_pixel(x: u32, y: u32) -> Rgba {
    let tile = x / 32 + y / 32 * 8;
    [
        (tile % 251) as u8 + 1,
        (x % 32) as u8 * 4 + (tile / 251) as u8,
        (y % 32) as u8,
        255,
    ]
}

/// Pixel (`x`, `y`) of tile `id` of the tileset.
fn tile_pixel(id: i32, x: u32, y: u32) -> Rgba {
    let index = (id - 384) as u32;
    tileset_pixel(index % 8 * 32 + x, index / 8 * 32 + y)
}

fn painted(stage: &mut Stage, width: u32, height: u32, pixel: fn(u32, u32) -> Rgba) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&pixel(x, y));
        }
    }
    stage.bitmap(width, height, rgba)
}

fn tileset(stage: &mut Stage) -> BitmapId {
    painted(stage, 256, TILES as u32 / 8 * 32, tileset_pixel)
}

/// The pixel at (`x`, `y`) of an autotile sheet: it names its frame, its quarter and its
/// place in the quarter.
fn autotile_pixel(x: u32, y: u32) -> Rgba {
    let quarter = x % 96 / 16 + y / 16 * 6;
    [
        quarter as u8 + 1,
        (x % 16) as u8 * 16 + (y % 16) as u8,
        (x / 96) as u8 + 100,
        255,
    ]
}

/// A map of `width` by `height` cells, empty.
fn map(width: i32, height: i32) -> SharedTable {
    SharedTable::new(Table::new_3d(width, height, 3).unwrap())
}

/// The tile the lowest layer of a [`full_map`] holds in cell (`x`, `y`).
fn cell_tile(x: i32, y: i32) -> i32 {
    384 + (x + y * 20) % TILES
}

/// A map of 20 by 15 cells in which every cell of the lowest layer holds another tile.
fn full_map() -> SharedTable {
    let map = map(20, 15);
    for y in 0..15 {
        for x in 0..20 {
            map.set(x, y, 0, i64::from(cell_tile(x, y)));
        }
    }
    map
}

/// A tilemap on the screen with a tileset and `map`.
fn tilemap(stage: &mut Stage, map: &SharedTable) -> TilemapId {
    let tileset = tileset(stage);
    let id = stage.scene.create_tilemap(None).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.tileset = Some(tileset);
    tilemap.map_data = Some(map.clone());
    id
}

/// Whether the frame shows a [`full_map`] scrolled to (`ox`, `oy`), inside the rectangle
/// at (`left`, `top`) of `width` by `height` pixels, and black elsewhere.
fn shows_full_map(frame: &Frame, (ox, oy): (i32, i32), area: Rect) -> bool {
    (0..frame.size.height as i32).all(|y| {
        (0..frame.size.width as i32).all(|x| {
            let inside = (area.x..area.x + area.width).contains(&x)
                && (area.y..area.y + area.height).contains(&y);
            let expected = if inside {
                let (map_x, map_y) = (x - area.x + ox, y - area.y + oy);
                let cell = (
                    map_x.div_euclid(32).rem_euclid(20),
                    map_y.div_euclid(32).rem_euclid(15),
                );
                let within = (map_x.rem_euclid(32) as u32, map_y.rem_euclid(32) as u32);
                tile_pixel(cell_tile(cell.0, cell.1), within.0, within.1)
            } else {
                BLACK
            };
            frame.pixel(x as u32, y as u32) == expected
        })
    })
}

const SCREEN: Rect = Rect::new(0, 0, 640, 480);

/// Whether the 32 by 32 pixels at (`x`, `y`) of the frame are `expected` of their place.
fn tile_shows(frame: &Frame, x: u32, y: u32, expected: impl Fn(u32, u32) -> Rgba) -> bool {
    (0..32).all(|py| (0..32).all(|px| frame.pixel(x + px, y + py) == expected(px, py)))
}

#[test]
fn a_tileset_tile_shows_its_32_by_32_pixels_of_the_tileset() {
    let mut stage = Stage::sized(96, 64);
    let map = map(3, 2);
    map.set(1, 1, 0, 384 + 8 + 3);
    tilemap(&mut stage, &map);
    let frame = stage.frame();
    assert!(tile_shows(&frame, 32, 32, |x, y| tileset_pixel(
        96 + x,
        32 + y
    )));
    // The cells that hold 0 show nothing.
    assert!(tile_shows(&frame, 0, 0, |_, _| BLACK));
    assert!(tile_shows(&frame, 64, 32, |_, _| BLACK));
    assert!(tile_shows(&frame, 32, 0, |_, _| BLACK));
    stage.assert_pinned("a_tileset_tile_shows_its_32_by_32_pixels_of_the_tileset");
}

#[test]
fn identifiers_below_48_and_tiles_outside_the_tileset_draw_nothing() {
    let mut stage = Stage::sized(96, 64);
    let map = map(3, 2);
    map.set(0, 0, 0, 1);
    map.set(1, 0, 1, 47);
    map.set(2, 0, 2, -5);
    map.set(0, 1, 0, i64::from(384 + TILES));
    map.set(1, 1, 0, i64::from(i16::MAX));
    tilemap(&mut stage, &map);
    let before = stage.bitmaps.len();
    assert!(all(&stage.frame(), BLACK));
    assert_eq!(stage.bitmaps.len(), before, "nothing to cache");
    stage.assert_pinned("identifiers_below_48_and_tiles_outside_the_tileset_draw_nothing");
}

#[test]
fn a_tilemap_without_a_live_sheet_draws_nothing_for_its_tiles() {
    let mut stage = Stage::sized(96, 64);
    let map = map(3, 2);
    map.set(0, 0, 0, 384);
    map.set(1, 0, 0, 48);
    let id = stage.scene.create_tilemap(None).unwrap();
    stage.scene.tilemap_mut(id).unwrap().map_data = Some(map.clone());
    assert!(all(&stage.frame(), BLACK));

    // With the sheets the tiles appear, and without them they go again.
    let tileset = tileset(&mut stage);
    let autotile = painted(&mut stage, 96, 128, autotile_pixel);
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.tileset = Some(tileset);
    tilemap.autotiles[0] = Some(autotile);
    let frame = stage.frame();
    assert!(tile_shows(&frame, 0, 0, |x, y| tile_pixel(384, x, y)));
    assert_ne!(frame.pixel(32, 0), BLACK);
    stage.bitmaps.dispose(tileset);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), BLACK);
    assert_ne!(frame.pixel(32, 0), BLACK);
    stage.bitmaps.dispose(autotile);
    assert!(all(&stage.frame(), BLACK));
    stage.assert_pinned("a_tilemap_without_a_live_sheet_draws_nothing_for_its_tiles");
}

#[test]
fn a_tilemap_without_map_data_draws_nothing() {
    let mut stage = Stage::sized(96, 64);
    let id = tilemap(&mut stage, &map(0, 0));
    assert!(all(&stage.frame(), BLACK));
    stage.scene.tilemap_mut(id).unwrap().map_data = None;
    assert!(all(&stage.frame(), BLACK));
    stage.assert_pinned("a_tilemap_without_map_data_draws_nothing");
}

/// A stage of 8 by 6 tiles showing the 48 patterns of autotile `index`, pattern `p` in
/// cell (`p mod 8`, `p div 8`), from `sheet`.
fn patterns(sheet_width: u32, sheet_height: u32, index: usize) -> (Stage, TilemapId) {
    let mut stage = Stage::sized(256, 192);
    let map = map(8, 6);
    for pattern in 0..48 {
        let id = 48 * (index as i32 + 1) + pattern;
        map.set(pattern % 8, pattern / 8, 0, i64::from(id));
    }
    let sheet = painted(&mut stage, sheet_width, sheet_height, autotile_pixel);
    let id = stage.scene.create_tilemap(None).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.autotiles[index] = Some(sheet);
    tilemap.map_data = Some(map);
    (stage, id)
}

/// Whether the frame of a [`patterns`] stage shows every pattern assembled from the
/// quarters the table names, taken from animation frame `shown` of the sheet.
fn shows_patterns(frame: &Frame, shown: u32) -> bool {
    (0..48u32).all(|pattern| {
        let (left, top) = (pattern % 8 * 32, pattern / 8 * 32);
        (0..4u32).all(|corner| {
            let quarter = u32::from(AUTOTILE_QUARTERS[pattern as usize][corner as usize]) - 1;
            let (sheet_x, sheet_y) = (shown * 96 + quarter % 6 * 16, quarter / 6 * 16);
            let (x, y) = (left + corner % 2 * 16, top + corner / 2 * 16);
            (0..16).all(|py| {
                (0..16).all(|px| {
                    frame.pixel(x + px, y + py) == autotile_pixel(sheet_x + px, sheet_y + py)
                })
            })
        })
    })
}

#[test]
fn every_autotile_pattern_is_assembled_from_the_quarters_the_table_names() {
    let (mut stage, _) = patterns(96, 128, 0);
    let frame = stage.frame();
    assert!(shows_patterns(&frame, 0));

    // The enclosed pattern, 0, and the isolated one, 47, spelled out.
    for (pattern, quarters) in [
        (0, [(32, 64), (48, 64), (32, 80), (48, 80)]),
        (47, [(0, 0), (16, 0), (0, 16), (16, 16)]),
    ] {
        let (left, top) = (pattern % 8 * 32, pattern / 8 * 32);
        for (corner, (sheet_x, sheet_y)) in quarters.into_iter().enumerate() {
            let (x, y) = (left + corner as u32 % 2 * 16, top + corner as u32 / 2 * 16);
            assert_eq!(frame.pixel(x, y), autotile_pixel(sheet_x, sheet_y));
            assert_eq!(
                frame.pixel(x + 15, y + 15),
                autotile_pixel(sheet_x + 15, sheet_y + 15)
            );
        }
    }
    stage.assert_pinned("every_autotile_pattern_is_assembled_from_the_quarters_the_table_names");
}

#[test]
fn each_of_the_seven_autotiles_has_its_own_sheet() {
    for index in [1, 6] {
        let (mut stage, _) = patterns(96, 128, index);
        assert!(shows_patterns(&stage.frame(), 0), "autotile {index}");
        stage.assert_pinned(&format!(
            "each_of_the_seven_autotiles_has_its_own_sheet_{index}"
        ));
    }
}

#[test]
fn a_sheet_32_pixels_high_shows_the_whole_sheet_for_every_pattern() {
    let (mut stage, _) = patterns(32, 32, 0);
    let frame = stage.frame();
    for pattern in 0..48 {
        assert!(
            tile_shows(&frame, pattern % 8 * 32, pattern / 8 * 32, autotile_pixel),
            "pattern {pattern}"
        );
    }
    stage.assert_pinned("a_sheet_32_pixels_high_shows_the_whole_sheet_for_every_pattern");
}

#[test]
fn an_autotile_shows_its_next_frame_every_16_updates() {
    let (mut stage, id) = patterns(384, 128, 0);
    assert!(shows_patterns(&stage.frame(), 0));
    let mut updates = 0;
    for (after, shown) in [
        (15, 0),
        (16, 1),
        (31, 1),
        (32, 2),
        (48, 3),
        (63, 3),
        (64, 0),
    ] {
        while updates < after {
            stage.scene.tilemap_mut(id).unwrap().update();
            updates += 1;
        }
        assert!(
            shows_patterns(&stage.frame(), shown),
            "after {after} updates"
        );
    }
    stage.assert_pinned("an_autotile_shows_its_next_frame_every_16_updates");
}

#[test]
fn a_single_tile_sheet_is_animated_in_frames_of_32_pixels() {
    let (mut stage, id) = patterns(96, 32, 0);
    for shown in [0, 1, 2, 0] {
        let frame = stage.frame();
        assert!(tile_shows(&frame, 64, 32, |x, y| autotile_pixel(
            shown * 32 + x,
            y
        )));
        for _ in 0..16 {
            stage.scene.tilemap_mut(id).unwrap().update();
        }
    }
    stage.assert_pinned("a_single_tile_sheet_is_animated_in_frames_of_32_pixels");
}

#[test]
fn without_updates_the_animation_stands_still() {
    let (mut stage, id) = patterns(384, 128, 0);
    for _ in 0..16 {
        stage.scene.tilemap_mut(id).unwrap().update();
    }
    let first = stage.scene.extract(&mut stage.bitmaps);
    assert!(shows_patterns(&stage.render(&first).unwrap(), 1));
    for _ in 0..40 {
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(packet.bitmaps, [], "nothing is composed again");
        assert_eq!(packet.ops, first.ops);
        assert!(shows_patterns(&stage.render(&packet).unwrap(), 1));
    }
}

#[test]
fn cell_x_y_is_drawn_at_32_times_its_coordinates() {
    let mut stage = Stage::new();
    tilemap(&mut stage, &full_map());
    let frame = stage.frame();
    assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
        cell_tile(3, 2),
        x,
        y
    )));
    assert!(shows_full_map(&frame, (0, 0), SCREEN));
    stage.assert_pinned("cell_x_y_is_drawn_at_32_times_its_coordinates");
}

#[test]
fn the_origin_scrolls_the_map_by_pixels() {
    let mut stage = Stage::new();
    let id = tilemap(&mut stage, &full_map());
    stage.scene.tilemap_mut(id).unwrap().ox = 40;
    let frame = stage.frame();
    // Cell (1, 0) starts 8 pixels left of the screen, cell (2, 0) at x 24.
    assert_eq!(frame.pixel(0, 0), tile_pixel(cell_tile(1, 0), 8, 0));
    assert_eq!(frame.pixel(23, 0), tile_pixel(cell_tile(1, 0), 31, 0));
    assert!(tile_shows(&frame, 24, 0, |x, y| tile_pixel(
        cell_tile(2, 0),
        x,
        y
    )));
    assert!(shows_full_map(&frame, (40, 0), SCREEN));

    for origin in [
        (0, 0),
        (31, 31),
        (32, 32),
        (-1, -1),
        (-33, 7),
        (1000, -1000),
    ] {
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        (tilemap.ox, tilemap.oy) = origin;
        assert!(shows_full_map(&stage.frame(), origin, SCREEN), "{origin:?}");
    }
    stage.assert_pinned("the_origin_scrolls_the_map_by_pixels");
}

#[test]
fn an_origin_at_the_limits_of_its_type_is_drawn() {
    let mut stage = Stage::sized(64, 64);
    let id = tilemap(&mut stage, &full_map());
    for origin in [(i32::MAX, i32::MIN), (i32::MIN, i32::MAX)] {
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        (tilemap.ox, tilemap.oy) = origin;
        let frame = stage.frame();
        let area = Rect::new(0, 0, 64, 64);
        let wrapped = (origin.0.rem_euclid(640), origin.1.rem_euclid(480));
        assert!(shows_full_map(&frame, wrapped, area), "{origin:?}");
    }
    stage.assert_pinned("an_origin_at_the_limits_of_its_type_is_drawn");
}

#[test]
fn scrolling_uploads_nothing() {
    let mut stage = Stage::new();
    let id = tilemap(&mut stage, &full_map());
    let autotile = painted(&mut stage, 384, 128, autotile_pixel);
    let priorities = SharedTable::new(Table::new(1000).unwrap());
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.autotiles[0] = Some(autotile);
    tilemap.priorities = Some(priorities);
    // The packet of the frame after the origin is set to `ox`.
    let mut packet = |ox: i32| {
        stage.scene.tilemap_mut(id).unwrap().ox = ox;
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let frame = stage.render(&packet).unwrap();
        assert!(shows_full_map(&frame, (ox, 0), SCREEN), "{ox}");
        packet
    };
    let first = packet(0);
    assert_eq!(first.tables.len(), 2, "tile numbers and priorities");
    assert!(!first.bitmaps.is_empty(), "the sheets");
    // 100 pixels in ten frames, across three tile boundaries.
    for frame in 1..=10 {
        let packet = packet(frame * 10);
        assert_eq!(packet.tables, [], "frame {frame}");
        assert_eq!(packet.bitmaps, [], "frame {frame}");
    }
}

#[test]
fn the_upper_layer_covers_the_lower_one() {
    let mut stage = Stage::sized(96, 64);
    let map = map(3, 2);
    map.set(1, 0, 0, 384 + 1);
    map.set(1, 0, 2, 384 + 2);
    map.set(2, 1, 0, 384 + 3);
    map.set(2, 1, 1, 384 + 4);
    map.set(2, 1, 2, 384 + 5);
    map.set(0, 1, 1, 384 + 6);
    tilemap(&mut stage, &map);
    let frame = stage.frame();
    assert!(tile_shows(&frame, 32, 0, |x, y| tile_pixel(384 + 2, x, y)));
    assert!(tile_shows(&frame, 64, 32, |x, y| tile_pixel(384 + 5, x, y)));
    assert!(tile_shows(&frame, 0, 32, |x, y| tile_pixel(384 + 6, x, y)));
    stage.assert_pinned("the_upper_layer_covers_the_lower_one");
}

#[test]
fn an_upper_tile_lets_the_lower_one_show_through_its_transparent_pixels() {
    let mut stage = Stage::sized(32, 32);
    let map = map(1, 1);
    map.set(0, 0, 0, 384);
    map.set(0, 0, 1, 385);
    let id = tilemap(&mut stage, &map);
    let tileset = stage.scene.tilemap(id).unwrap().tileset.unwrap();
    let hole = Rect::new(32 + 8, 8, 16, 16);
    stage
        .bitmaps
        .fill_rect(tileset, hole, color(CLEAR))
        .unwrap();
    let frame = stage.frame();
    assert!(tile_shows(&frame, 0, 0, |x, y| {
        let upper = !((8..24).contains(&x) && (8..24).contains(&y));
        tile_pixel(if upper { 385 } else { 384 }, x, y)
    }));
    stage.assert_pinned("an_upper_tile_lets_the_lower_one_show_through_its_transparent_pixels");
}

#[test]
fn the_map_repeats_beyond_its_edges() {
    let mut stage = Stage::new();
    let id = tilemap(&mut stage, &full_map());
    stage.scene.tilemap_mut(id).unwrap().ox = 320;
    let frame = stage.frame();
    // The right half of the screen shows columns 0 to 9.
    for column in 0..10 {
        assert!(
            tile_shows(&frame, 320 + column * 32, 64, |x, y| {
                tile_pixel(cell_tile(column as i32, 2), x, y)
            }),
            "column {column}"
        );
    }
    assert!(shows_full_map(&frame, (320, 0), SCREEN));
    stage.assert_pinned("the_map_repeats_beyond_its_edges");
}

#[test]
fn a_tilemap_in_a_viewport_is_drawn_inside_its_rectangle() {
    let mut stage = Stage::new();
    let area = Rect::new(100, 50, 200, 100);
    let viewport = stage.scene.create_viewport(area).unwrap();
    let tileset = tileset(&mut stage);
    let id = stage.scene.create_tilemap(Some(viewport)).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.tileset = Some(tileset);
    tilemap.map_data = Some(full_map());
    assert!(shows_full_map(&stage.frame(), (0, 0), area));

    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    (tilemap.ox, tilemap.oy) = (45, -7);
    assert!(shows_full_map(&stage.frame(), (45, -7), area));

    // The origin of the viewport scrolls the map like every element.
    stage.scene.viewport_mut(viewport).unwrap().ox = 20;
    assert!(shows_full_map(&stage.frame(), (65, -7), area));

    // A viewport that reaches off the screen shows the part that is on it.
    let viewport = stage.scene.viewport_mut(viewport).unwrap();
    viewport.ox = 0;
    viewport.rect = Rect::new(-50, -20, 200, 100);
    let frame = stage.frame();
    assert!(shows_full_map(&frame, (95, 13), Rect::new(0, 0, 150, 80)));
    stage.assert_pinned("a_tilemap_in_a_viewport_is_drawn_inside_its_rectangle");
}

/// The stage of the priority scenarios: a tile of `priority` in cell (2, 3), whose top
/// edge is at y 96, a tile of priority 0 next to it in cell (3, 3), and a blue sprite
/// over both, created before the tilemap.
fn with_priority(priority: i64) -> (Stage, SpriteId, TilemapId) {
    let mut stage = Stage::sized(160, 160);
    let blue = stage.solid(64, 32, BLUE);
    let sprite = stage.scene.create_sprite(None).unwrap();
    let sprite_mut = stage.scene.sprite_mut(sprite).unwrap();
    sprite_mut.set_bitmap(Some(blue), &stage.bitmaps);
    (sprite_mut.x, sprite_mut.y) = (64, 96);

    let map = map(5, 5);
    map.set(2, 3, 0, 400);
    map.set(3, 3, 0, 384);
    let priorities = SharedTable::new(Table::new(500).unwrap());
    priorities.set(400, 0, 0, priority);
    let id = tilemap(&mut stage, &map);
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities);
    (stage, sprite, id)
}

/// Whether the tile of priority, and the ground tile, are in front of the sprite.
fn in_front(stage: &mut Stage) -> (bool, bool) {
    let frame = stage.frame();
    let tile = tile_shows(&frame, 64, 96, |x, y| tile_pixel(400, x, y));
    let ground = tile_shows(&frame, 96, 96, |x, y| tile_pixel(384, x, y));
    assert!(tile || tile_shows(&frame, 64, 96, |_, _| BLUE));
    assert!(ground || tile_shows(&frame, 96, 96, |_, _| BLUE));
    (tile, ground)
}

#[test]
fn tiles_of_priority_0_are_drawn_at_z_0() {
    let (mut stage, sprite, _) = with_priority(0);
    assert_eq!(in_front(&mut stage), (true, true), "the tilemap is newer");
    stage.scene.set_sprite_z(sprite, 1).unwrap();
    assert_eq!(in_front(&mut stage), (false, false));
    stage.scene.set_sprite_z(sprite, -1).unwrap();
    assert_eq!(in_front(&mut stage), (true, true));
    stage.assert_pinned("tiles_of_priority_0_are_drawn_at_z_0");
}

#[test]
fn a_tile_of_priority_1_ranks_with_the_sprites_one_row_below_it() {
    let (mut stage, sprite, _) = with_priority(1);
    stage.scene.set_sprite_z(sprite, 159).unwrap();
    assert_eq!(in_front(&mut stage), (true, false));
    stage.scene.set_sprite_z(sprite, 192).unwrap();
    assert_eq!(in_front(&mut stage), (false, false));
    // At the z of the tile the newer of the two is in front: here the tilemap, which was
    // created after the sprite.
    stage.scene.set_sprite_z(sprite, 160).unwrap();
    assert_eq!(in_front(&mut stage), (true, false));
    stage.scene.set_sprite_z(sprite, 161).unwrap();
    assert_eq!(in_front(&mut stage), (false, false));
    stage.assert_pinned("a_tile_of_priority_1_ranks_with_the_sprites_one_row_below_it");
}

#[test]
fn a_tile_of_a_higher_priority_ranks_32_higher_for_each_step() {
    let (mut stage, sprite, _) = with_priority(2);
    stage.scene.set_sprite_z(sprite, 191).unwrap();
    assert_eq!(in_front(&mut stage), (true, false));
    stage.scene.set_sprite_z(sprite, 193).unwrap();
    assert_eq!(in_front(&mut stage), (false, false));

    for priority in 1..=5 {
        let (mut stage, sprite, _) = with_priority(priority);
        let z = 96 + (priority as i32 + 1) * 32;
        stage.scene.set_sprite_z(sprite, z - 1).unwrap();
        assert_eq!(in_front(&mut stage), (true, false), "priority {priority}");
        stage.scene.set_sprite_z(sprite, z + 1).unwrap();
        assert_eq!(in_front(&mut stage), (false, false), "priority {priority}");
        stage.assert_pinned(&format!(
            "a_tile_of_a_higher_priority_ranks_32_higher_for_each_step_{priority}"
        ));
    }
    stage.assert_pinned("a_tile_of_a_higher_priority_ranks_32_higher_for_each_step");
}

#[test]
fn the_z_of_a_tile_follows_its_row_on_the_screen() {
    let (mut stage, sprite, id) = with_priority(1);
    // Scrolled up by a row and a half, the tile's top edge is at y 48.
    stage.scene.tilemap_mut(id).unwrap().oy = 48;
    stage.scene.sprite_mut(sprite).unwrap().y = 48;
    stage.scene.set_sprite_z(sprite, 48 + 64 - 1).unwrap();
    assert_eq!(in_front_at(&mut stage, 48), (true, false));
    stage.scene.set_sprite_z(sprite, 48 + 64 + 1).unwrap();
    assert_eq!(in_front_at(&mut stage, 48), (false, false));
    stage.assert_pinned("the_z_of_a_tile_follows_its_row_on_the_screen");
}

/// [`in_front`] for a tilemap scrolled so that the row of the tiles is at `y`.
fn in_front_at(stage: &mut Stage, y: u32) -> (bool, bool) {
    let frame = stage.frame();
    (
        tile_shows(&frame, 64, y, |px, py| tile_pixel(400, px, py)),
        tile_shows(&frame, 96, y, |px, py| tile_pixel(384, px, py)),
    )
}

#[test]
fn the_origin_of_the_viewport_does_not_enter_the_z_of_a_tile() {
    let mut stage = Stage::sized(160, 160);
    let viewport = stage.scene.create_viewport(Rect::new(0, 0, 160, 160));
    let viewport = viewport.unwrap();
    stage.scene.viewport_mut(viewport).unwrap().oy = 32;
    let blue = stage.solid(32, 32, BLUE);
    let sprite = stage.scene.create_sprite(Some(viewport)).unwrap();
    let sprite_mut = stage.scene.sprite_mut(sprite).unwrap();
    sprite_mut.set_bitmap(Some(blue), &stage.bitmaps);
    (sprite_mut.x, sprite_mut.y) = (64, 96);

    let map = map(5, 5);
    map.set(2, 3, 0, 400);
    let priorities = SharedTable::new(Table::new(500).unwrap());
    priorities.set(400, 0, 0, 1);
    let tileset = tileset(&mut stage);
    let id = stage.scene.create_tilemap(Some(viewport)).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.tileset = Some(tileset);
    tilemap.map_data = Some(map);
    tilemap.priorities = Some(priorities);

    // The viewport's origin moves the tile's top edge from y 96 to y 64 on the screen, and
    // its z stays 96 + 64. Tile and sprite are both drawn at (64, 64).
    let mut tile_in_front = |z: i32| {
        stage.scene.set_sprite_z(sprite, z).unwrap();
        let frame = stage.frame();
        let tile = tile_shows(&frame, 64, 64, |x, y| tile_pixel(400, x, y));
        assert!(tile || tile_shows(&frame, 64, 64, |_, _| BLUE));
        tile
    };
    assert!(!tile_in_front(161));
    assert!(tile_in_front(159));
    assert!(tile_in_front(129));

    // The tilemap's origin counts.
    stage.scene.tilemap_mut(id).unwrap().oy = -16;
    stage.scene.sprite_mut(sprite).unwrap().y = 112;
    let mut tile_in_front = |z: i32| {
        stage.scene.set_sprite_z(sprite, z).unwrap();
        let frame = stage.frame();
        tile_shows(&frame, 64, 80, |x, y| tile_pixel(400, x, y))
    };
    assert!(tile_in_front(112 + 64 - 1));
    assert!(!tile_in_front(112 + 64 + 1));
    stage.assert_pinned("the_origin_of_the_viewport_does_not_enter_the_z_of_a_tile");
}

/// The z of the tiles of `priority` in map row `row`, counted without wrapping, of a map of
/// 20 by `height` cells that is scrolled to `oy` and drawn in a viewport, if one is given,
/// at `rect_y` with the origin `viewport_oy`.
fn row_z(height: i32, row: i64, priority: i64, oy: i32, viewport: Option<(i32, i32)>) -> i64 {
    let mut stage = Stage::new();
    let viewport = viewport.map(|(rect_y, viewport_oy)| {
        let rect = Rect::new(0, rect_y, 640, 480);
        let id = stage.scene.create_viewport(rect).unwrap();
        stage.scene.viewport_mut(id).unwrap().oy = viewport_oy;
        id
    });
    let map = map(20, height);
    for y in 0..height {
        map.set(0, y, 0, 400);
    }
    let priorities = SharedTable::new(Table::new(500).unwrap());
    priorities.set(400, 0, 0, priority);
    let tileset = tileset(&mut stage);
    let id = stage.scene.create_tilemap(viewport).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.tileset = Some(tileset);
    tilemap.map_data = Some(map);
    tilemap.priorities = Some(priorities);
    tilemap.oy = oy;
    stage.frame();

    let tilemap = stage.scene.tilemap(id).unwrap();
    let first = tilemap.cache.view.as_ref().unwrap().first.1;
    let scroll = viewport.map_or(0, |id| stage.scene.viewport(id).unwrap().oy);
    let strips = tilemap.strips(i64::from(scroll)).into_iter();
    let mut found = strips.filter_map(|(z, strip)| match strip {
        Strip::Ground => (priority == 0).then_some(z),
        Strip::Row { row: shown, .. } => (first + i64::from(shown) == row).then_some(z),
        Strip::Flash => None,
    });
    found.next().expect("the row shows")
}

#[test]
fn the_z_of_a_row_is_the_measured_one() {
    // Row, priority and origin of the tilemap, then the z, as measured on the original
    // runtime.
    for (row, priority, oy, z) in [
        (0, 0, 0, 0),
        (0, 0, 45, 0),
        (1, 1, 0, 96),
        (2, 2, 0, 160),
        (3, 3, 0, 224),
        (4, 4, 0, 288),
        (5, 5, 0, 352),
        (7, 1, 0, 288),
        (14, 2, 0, 544),
        (1, 1, 7, 89),
        (1, 1, 31, 65),
        (2, 2, 32, 128),
        (3, 3, 109, 115),
        (11, 5, 300, 244),
    ] {
        assert_eq!(row_z(15, row, priority, oy, None), z, "row {row}, oy {oy}");
    }
    // The row above row 0 of a map of 4 rows.
    assert_eq!(row_z(4, -1, 5, -20, None), 180);
    // Neither the origin nor the rectangle of a viewport enters.
    assert_eq!(row_z(15, 1, 1, 0, Some((0, 0))), 96);
    assert_eq!(row_z(15, 1, 1, 0, Some((0, 10))), 96);
    assert_eq!(row_z(15, 1, 1, 7, Some((16, 10))), 89);
}

#[test]
fn a_tile_whose_priority_is_not_0_to_5_is_not_drawn() {
    for priority in [6, 7, 8, 9, 10, 100, 255, 256, 32767, -1, -100, -32768] {
        let (mut stage, sprite, _) = with_priority(priority);
        // Neither in a row nor as ground: behind, at and in front of every z the cell is
        // without the tile, and the ground tile next to it is drawn.
        stage.scene.sprite_mut(sprite).unwrap().visible = false;
        let frame = stage.frame();
        assert!(tile_shows(&frame, 64, 96, |_, _| BLACK), "{priority}");
        assert!(
            tile_shows(&frame, 96, 96, |x, y| tile_pixel(384, x, y)),
            "{priority}"
        );
        stage.scene.sprite_mut(sprite).unwrap().visible = true;
        for z in [-1, 0, 1, 96 + 7 * 32, i32::MAX] {
            stage.scene.set_sprite_z(sprite, z).unwrap();
            let frame = stage.frame();
            assert!(tile_shows(&frame, 64, 96, |_, _| BLUE), "{priority}, {z}");
        }
    }
    // A priority that is in the table for a tile the map does not hold is harmless, and a
    // map of such tiles alone sends no op.
    let mut stage = Stage::sized(96, 64);
    let map = map(3, 2);
    map.set(1, 1, 0, 400);
    let id = tilemap(&mut stage, &map);
    let priorities = SharedTable::new(Table::new(500).unwrap());
    priorities.set(400, 0, 0, 6);
    priorities.set(401, 0, 0, -1);
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities.clone());
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(layers(&packet), []);
    priorities.set(400, 0, 0, 5);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(layers(&packet).len(), 1);
}

#[test]
fn an_invisible_tilemap_gives_its_bitmaps_back_also_in_a_hidden_viewport() {
    let mut stage = Stage::sized(160, 160);
    let viewport = stage.scene.create_viewport(Rect::new(0, 0, 160, 160));
    let viewport = viewport.unwrap();
    let map = map(5, 5);
    map.set(1, 1, 0, 384);
    let tileset = tileset(&mut stage);
    let id = stage.scene.create_tilemap(Some(viewport)).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.tileset = Some(tileset);
    tilemap.map_data = Some(map);
    // The one bitmap a tilemap has of its own is the overlay of its flashing cells.
    let flash = SharedTable::new(Table::new_2d(5, 5).unwrap());
    flash.set(1, 1, 0, 0xf00);
    tilemap.flash_data = Some(flash);
    let before = stage.bitmaps.len();
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 1);

    // A visible tilemap in a hidden viewport keeps its bitmaps.
    stage.scene.viewport_mut(viewport).unwrap().visible = false;
    assert!(all(&stage.frame(), BLACK));
    assert_eq!(stage.bitmaps.len(), before + 1);

    // An invisible one gives them back, whatever its viewport: hidden or off the screen.
    stage.scene.tilemap_mut(id).unwrap().visible = false;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before);

    stage.scene.viewport_mut(viewport).unwrap().visible = true;
    stage.scene.tilemap_mut(id).unwrap().visible = true;
    assert!(!all(&stage.frame(), BLACK));
    assert_eq!(stage.bitmaps.len(), before + 1);
    stage.scene.viewport_mut(viewport).unwrap().rect = Rect::new(200, 0, 64, 64);
    stage.scene.tilemap_mut(id).unwrap().visible = false;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before);

    stage.scene.viewport_mut(viewport).unwrap().rect = Rect::new(0, 0, 160, 160);
    stage.scene.tilemap_mut(id).unwrap().visible = true;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 1);
    // A disposed viewport takes the tilemap along.
    stage.scene.dispose_viewport(viewport, &mut stage.bitmaps);
    assert!(stage.scene.is_tilemap_disposed(id));
    assert_eq!(stage.bitmaps.len(), before);
    assert!(all(&stage.frame(), BLACK));
    assert_eq!(stage.bitmaps.len(), before);
    stage.assert_pinned("an_invisible_tilemap_gives_its_bitmaps_back_also_in_a_hidden_viewport");
}

#[test]
fn a_tile_without_an_entry_in_the_priorities_has_priority_0() {
    let (mut stage, sprite, id) = with_priority(3);
    stage.scene.set_sprite_z(sprite, 1).unwrap();
    assert_eq!(in_front(&mut stage), (true, false));
    let short = SharedTable::new(Table::new(390).unwrap());
    short.set(384, 0, 0, 5);
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(short);
    assert_eq!(in_front(&mut stage), (false, true));
    stage.scene.tilemap_mut(id).unwrap().priorities = None;
    assert_eq!(in_front(&mut stage), (false, false));
    stage.assert_pinned("a_tile_without_an_entry_in_the_priorities_has_priority_0");
}

#[test]
fn a_change_to_a_shared_table_shows_in_the_next_frame() {
    let mut stage = Stage::new();
    let map = full_map();
    let id = tilemap(&mut stage, &map);
    assert!(shows_full_map(&stage.frame(), (0, 0), SCREEN));

    // The cell is changed through the handle the script kept, not through the tilemap.
    map.set(3, 2, 0, 384 + 300);
    let frame = stage.frame();
    assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
        384 + 300,
        x,
        y
    )));
    assert!(tile_shows(&frame, 128, 64, |x, y| tile_pixel(
        cell_tile(4, 2),
        x,
        y
    )));

    // So is a priority.
    let blue = stage.solid(32, 32, BLUE);
    let sprite = stage.scene.create_sprite(None).unwrap();
    stage.scene.set_sprite_z(sprite, 100).unwrap();
    let sprite = stage.scene.sprite_mut(sprite).unwrap();
    sprite.set_bitmap(Some(blue), &stage.bitmaps);
    (sprite.x, sprite.y) = (96, 64);
    let priorities = SharedTable::new(Table::new(1000).unwrap());
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities.clone());
    assert_eq!(stage.frame().pixel(96, 64), BLUE);
    priorities.set(384 + 300, 0, 0, 1);
    let frame = stage.frame();
    assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
        384 + 300,
        x,
        y
    )));

    // Another table with the same contents is another table.
    let other = SharedTable::new(map.read().clone());
    stage.scene.tilemap_mut(id).unwrap().map_data = Some(other);
    map.set(3, 2, 0, 384);
    let frame = stage.frame();
    assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
        384 + 300,
        x,
        y
    )));
    stage.assert_pinned("a_change_to_a_shared_table_shows_in_the_next_frame");
}

#[test]
fn drawing_into_a_sheet_shows_in_the_next_frame() {
    let mut stage = Stage::sized(96, 64);
    let map = map(3, 2);
    map.set(0, 0, 0, 384);
    map.set(1, 0, 0, 48 + 47);
    let id = tilemap(&mut stage, &map);
    let autotile = painted(&mut stage, 96, 128, autotile_pixel);
    stage.scene.tilemap_mut(id).unwrap().autotiles[0] = Some(autotile);
    let tileset = stage.scene.tilemap(id).unwrap().tileset.unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(5, 6), tile_pixel(384, 5, 6));
    assert_eq!(frame.pixel(32 + 5, 6), autotile_pixel(5, 6));

    stage
        .bitmaps
        .set_pixel(tileset, 5, 6, color(WHITE))
        .unwrap();
    let frame = stage.frame();
    assert_eq!(frame.pixel(5, 6), WHITE);
    assert_eq!(frame.pixel(6, 6), tile_pixel(384, 6, 6));

    stage.bitmaps.set_pixel(autotile, 5, 6, color(RED)).unwrap();
    assert_eq!(stage.frame().pixel(32 + 5, 6), RED);

    // Another sheet with other pixels replaces the old one.
    let other = stage.solid(256, 64, GREEN);
    stage.scene.tilemap_mut(id).unwrap().tileset = Some(other);
    assert_eq!(stage.frame().pixel(5, 6), GREEN);
    stage.assert_pinned("drawing_into_a_sheet_shows_in_the_next_frame");
}

/// Measured on the original runtime (tilemap blend probe): an opaque tile of priority 0
/// on the lowest layer and the same translucent tile of priority 2 on both layers above.
/// Each of the two is blended onto the frame by itself; composed with each other first
/// they would give (111, 106, 120).
#[test]
fn stacked_translucent_tiles_of_one_priority_are_blended_one_by_one() {
    let mut stage = Stage::sized(64, 64);
    let mut rgba = [6, 14, 2, 255].repeat(32 * 32);
    rgba.extend([138, 130, 150, 142].repeat(32 * 32));
    let tileset = stage.bitmap(32, 64, rgba);
    let map = map(3, 3);
    map.set(0, 0, 0, 384);
    map.set(0, 0, 1, 392);
    map.set(0, 0, 2, 392);
    // One translucent tile alone, and one opaque tile alone.
    map.set(1, 0, 2, 392);
    map.set(1, 1, 0, 384);
    let priorities = SharedTable::new(Table::new(500).unwrap());
    priorities.set(392, 0, 0, 2);
    let id = stage.scene.create_tilemap(None).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.tileset = Some(tileset);
    tilemap.map_data = Some(map.clone());
    tilemap.priorities = Some(priorities);

    let packet = stage.scene.extract(&mut stage.bitmaps);
    // The ground of the lowest layer, and a row of priority 2 of each layer above: the
    // lowest layer holds no tile of that priority and draws nothing for it.
    assert_eq!(packet.ops.len(), 3);
    let frame = stage.render(&packet).unwrap();
    assert_eq!(frame.pixel(5, 6), [111, 107, 120, 255]);
    let once = vkxp_pixel::blend_normal(BLACK, [138, 130, 150, 142], 255);
    assert_eq!(frame.pixel(40, 6), once);
    assert_eq!(frame.pixel(40, 40), [6, 14, 2, 255]);
    // The same picture once more, as a frame of the stage.
    assert_eq!(stage.frame(), frame);
    stage.assert_pinned("stacked_translucent_tiles_of_one_priority_are_blended_one_by_one");
}

/// The layers of the ground are blended one by one as well.
#[test]
fn stacked_translucent_tiles_of_the_ground_are_blended_one_by_one() {
    let mut stage = Stage::sized(64, 64);
    let mut rgba = [6, 14, 2, 255].repeat(32 * 32);
    rgba.extend([138, 130, 150, 142].repeat(32 * 32));
    let tileset = stage.bitmap(32, 64, rgba);
    let map = map(3, 3);
    map.set(0, 0, 0, 384);
    map.set(0, 0, 1, 392);
    map.set(0, 0, 2, 392);
    let id = stage.scene.create_tilemap(None).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.tileset = Some(tileset);
    tilemap.map_data = Some(map.clone());
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(packet.ops.len(), 3);
    let frame = stage.render(&packet).unwrap();
    assert_eq!(frame.pixel(5, 6), [111, 107, 120, 255]);
    assert_eq!(stage.frame(), frame);
    stage.assert_pinned("stacked_translucent_tiles_of_the_ground_are_blended_one_by_one");
}

#[test]
fn tiles_are_drawn_without_bitmaps_of_the_scene() {
    let mut stage = Stage::sized(160, 160);
    let map = map(5, 5);
    map.set(0, 0, 0, 384);
    map.set(1, 1, 0, 385);
    map.set(2, 2, 0, 386);
    let priorities = SharedTable::new(Table::new(500).unwrap());
    priorities.set(385, 0, 0, 1);
    priorities.set(386, 0, 0, 5);
    let id = tilemap(&mut stage, &map);
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities.clone());
    let before = stage.bitmaps.len();
    assert_eq!(before, 1, "the tileset");
    for ox in [0, 32, 45] {
        stage.scene.tilemap_mut(id).unwrap().ox = ox;
        assert!(!all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), before);
    }
    stage.scene.tilemap_mut(id).unwrap().visible = false;
    assert!(all(&stage.frame(), BLACK));
    stage.scene.tilemap_mut(id).unwrap().visible = true;
    assert!(!all(&stage.frame(), BLACK));
    assert_eq!(stage.bitmaps.len(), before);
    stage.assert_pinned("tiles_are_drawn_without_bitmaps_of_the_scene");
}

/// The tilemap layer ops of a packet.
fn layers(packet: &FramePacket) -> Vec<TilemapLayer> {
    let ops = packet.ops.iter().filter_map(|op| match op {
        Op::TilemapLayer(layer) => Some(*layer),
        _ => None,
    });
    ops.collect()
}

#[test]
fn a_tilemap_is_one_op_per_layer_of_the_ground_and_per_row_and_layer_above() {
    let mut stage = Stage::sized(96, 96);
    let map = map(3, 3);
    // Ground on layers 0 and 2; priority 1 in row 1 on layers 0 and 1 and in row 2 on
    // layer 1; priority 3 in row 0 on layer 2.
    map.set(0, 0, 0, 384);
    map.set(2, 2, 2, 385);
    map.set(1, 1, 0, 386);
    map.set(2, 1, 1, 386);
    map.set(0, 2, 1, 386);
    map.set(1, 0, 2, 387);
    let priorities = SharedTable::new(Table::new(400).unwrap());
    priorities.set(386, 0, 0, 1);
    priorities.set(387, 0, 0, 3);
    let id = tilemap(&mut stage, &map);
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.priorities = Some(priorities);
    (tilemap.ox, tilemap.oy) = (5, -7);
    let tileset = tilemap.tileset;
    stage.frame();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let found = layers(&packet);
    assert_eq!(found.len(), packet.ops.len());
    let screen = vkxp_proto::Rect::new(0, 0, 96, 96);
    let row = |row: i32| vkxp_proto::Rect::new(0, row * 32 + 7, 96, 32);
    // In draw order: the ground at z 0, then the rows by their z, which is 64 more than
    // their top edge for priority 1 and 128 more for priority 3. The map repeats: its
    // last row shows once more in the seven rows of pixels above its first.
    let expected = [
        (screen, 0, 0),
        (screen, 2, 0),
        (vkxp_proto::Rect::new(0, 0, 96, 7), 1, 1),
        (row(1), 0, 1),
        (row(1), 1, 1),
        (row(2), 1, 1),
        (row(0), 2, 3),
    ];
    let pieces: Vec<_> = found
        .iter()
        .map(|layer| (layer.clip, layer.layer, layer.priority))
        .collect();
    // The last row reaches below the screen and is clipped to it.
    let mut expected = expected.to_vec();
    expected[5].0 = vkxp_proto::Rect::new(0, 71, 96, 25);
    assert_eq!(expected[5].0.y, row(2).y);
    assert_eq!(pieces, expected);
    for layer in &found {
        assert_eq!(layer.origin, (5, -7));
        assert_eq!(layer.tileset, tileset);
        assert_eq!(
            (layer.map, layer.priorities),
            (found[0].map, found[0].priorities)
        );
        assert_ne!(layer.map, layer.priorities);
    }
    stage.assert_pinned("a_tilemap_is_one_op_per_layer_of_the_ground_and_per_row_and_layer_above");
}

#[test]
fn a_changed_table_is_sent_again_and_a_table_no_tilemap_has_is_destroyed() {
    let mut stage = Stage::new();
    let map = full_map();
    let priorities = SharedTable::new(Table::new(1000).unwrap());
    let id = tilemap(&mut stage, &map);
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities.clone());
    let first = stage.scene.extract(&mut stage.bitmaps);
    stage.render(&first).unwrap();
    let op = layers(&first)[0];
    assert_eq!(first.tables.len(), 2);

    // One cell is changed through the handle the script kept: that table is replaced.
    map.set(3, 2, 0, 384 + 300);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let expected = TableDelta::Set {
        id: op.map,
        xsize: 20,
        ysize: 15,
        zsize: 3,
        data: map.read().data().to_vec(),
    };
    assert_eq!(packet.tables, [expected]);
    let frame = stage.render(&packet).unwrap();
    assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
        384 + 300,
        x,
        y
    )));

    // A priority outside 0 to 5 is sent too.
    priorities.set(384, 0, 0, 9);
    priorities.set(385, 0, 0, -3);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let [
        TableDelta::Set {
            id: sent,
            xsize,
            data,
            ..
        },
    ] = &packet.tables[..]
    else {
        panic!("{:?}", packet.tables);
    };
    assert_eq!((*sent, *xsize), (op.priorities, 1000));
    assert_eq!(data[384..386], [9, -3]);
    stage.render(&packet).unwrap();

    // Invisible, the tilemap keeps its tables with the compositor.
    stage.scene.tilemap_mut(id).unwrap().visible = false;
    assert_eq!(stage.scene.extract(&mut stage.bitmaps).tables, []);
    stage.scene.tilemap_mut(id).unwrap().visible = true;
    assert_eq!(stage.scene.extract(&mut stage.bitmaps).tables, []);

    // Without its priorities it names a table without entries, and the old one goes.
    stage.scene.tilemap_mut(id).unwrap().priorities = None;
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let destroyed = TableDelta::Destroy { id: op.priorities };
    assert_eq!(packet.tables.len(), 2);
    assert!(packet.tables.contains(&destroyed));
    assert!(all_layers_name(&packet, op.map));
    stage.render(&packet).unwrap();

    stage.scene.dispose_tilemap(id, &mut stage.bitmaps);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(packet.tables, [TableDelta::Destroy { id: op.map }]);
    // The compositor is told, and accepts the frame that follows.
    assert!(all(&stage.render(&packet).unwrap(), BLACK));
    assert!(all(&stage.frame(), BLACK));
}

fn all_layers_name(packet: &FramePacket, map: TableId) -> bool {
    let layers = layers(packet);
    !layers.is_empty() && layers.iter().all(|layer| layer.map == map)
}

#[test]
fn two_tilemaps_share_the_tables_they_share() {
    let mut stage = Stage::sized(96, 64);
    let map = map(3, 2);
    map.set(0, 0, 0, 384);
    let first = tilemap(&mut stage, &map);
    let second = tilemap(&mut stage, &map);
    stage.scene.tilemap_mut(second).unwrap().ox = 32;
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(packet.tables.len(), 2, "the map and the empty priorities");
    let layers = layers(&packet);
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].map, layers[1].map);
    stage.render(&packet).unwrap();
    // One of them disposed, the other still has the table.
    stage.scene.dispose_tilemap(first, &mut stage.bitmaps);
    assert_eq!(stage.scene.extract(&mut stage.bitmaps).tables, []);
}

#[test]
fn a_sprite_between_two_rows_is_in_front_of_the_upper_and_behind_the_lower() {
    let mut stage = Stage::sized(96, 96);
    let map = map(3, 3);
    map.set(0, 0, 0, 384);
    map.set(0, 1, 0, 384);
    let priorities = SharedTable::new(Table::new(400).unwrap());
    priorities.set(384, 0, 0, 1);
    let id = tilemap(&mut stage, &map);
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities);
    // The rows rank at 64 and 96; the sprite covers both cells from in between.
    let blue = stage.solid(32, 64, BLUE);
    let sprite = stage.scene.create_sprite(None).unwrap();
    stage.scene.set_sprite_z(sprite, 80).unwrap();
    let sprite = stage.scene.sprite_mut(sprite).unwrap();
    sprite.set_bitmap(Some(blue), &stage.bitmaps);
    let frame = stage.frame();
    assert!(tile_shows(&frame, 0, 0, |_, _| BLUE));
    assert!(tile_shows(&frame, 0, 32, |x, y| tile_pixel(384, x, y)));
    stage.assert_pinned("a_sprite_between_two_rows_is_in_front_of_the_upper_and_behind_the_lower");
}

/// Not a check: prints how long extracting a frame takes for a screen of 640 by 480,
/// which shows 21 by 16 cells, with three full layers. Run it in the release profile:
/// `cargo test --release -p vkxp-core measure_an_extraction -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, run on request"]
fn measure_an_extraction() {
    let mut stage = Stage::new();
    let map = map(40, 30);
    for y in 0..30 {
        for x in 0..40 {
            // Autotiles below, tileset tiles above, some of them with a priority.
            map.set(x, y, 0, i64::from(48 + (x * 7 + y * 13) % 336));
            map.set(x, y, 1, i64::from(384 + (x + y * 3) % TILES));
            map.set(x, y, 2, i64::from(384 + (x * 5 + y) % TILES));
        }
    }
    let priorities = SharedTable::new(Table::new(384 + TILES).unwrap());
    for id in 384..384 + TILES {
        priorities.set(id, 0, 0, i64::from(id % 6));
    }
    let id = tilemap(&mut stage, &map);
    for index in 0..7 {
        let sheet = painted(&mut stage, 384, 128, autotile_pixel);
        stage.scene.tilemap_mut(id).unwrap().autotiles[index] = Some(sheet);
    }
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities);
    stage.frame();

    let mut extract = Vec::new();
    for step in 1..=20 {
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        (tilemap.ox, tilemap.oy) = (step * 32, step * 32);
        let start = std::time::Instant::now();
        let packet = stage.scene.extract(&mut stage.bitmaps);
        extract.push(start.elapsed());
        assert!(packet.bitmaps.is_empty() && packet.tables.is_empty());
    }
    extract.sort();
    println!(
        "extracting a frame of 21 by 16 cells in three layers: median {:?}",
        extract[10]
    );
}

/// A stage of 3 by 2 cells whose every cell holds a tileset tile, with flash data that
/// is all zero, at the strongest point of the pulse.
fn flashing() -> (Stage, TilemapId, SharedTable) {
    let mut stage = Stage::sized(96, 64);
    let map = map(3, 2);
    for cell in 0..6 {
        map.set(cell % 3, cell / 3, 0, i64::from(384 + cell));
    }
    let id = tilemap(&mut stage, &map);
    let flash = SharedTable::new(Table::new_2d(3, 2).unwrap());
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.flash_data = Some(flash.clone());
    for _ in 0..16 {
        tilemap.update();
    }
    (stage, id, flash)
}

/// Whether `frame` is `plain` with `color` added to the 32 by 32 pixels
/// at (`left`, `top`), as far as they are on the screen, and nothing else.
fn flashes(frame: &Frame, plain: &Frame, (left, top): (i32, i32), color: Rgba) -> bool {
    (0..frame.size.height).all(|y| {
        (0..frame.size.width).all(|x| {
            let inside =
                (left..left + 32).contains(&(x as i32)) && (top..top + 32).contains(&(y as i32));
            let expected = if inside {
                vkxp_pixel::blend_add(plain.pixel(x, y), color, 255)
            } else {
                plain.pixel(x, y)
            };
            frame.pixel(x, y) == expected
        })
    })
}

#[test]
fn a_cell_with_a_flash_entry_is_tinted_and_its_neighbours_are_not() {
    let (mut stage, id, flash) = flashing();
    let plain = stage.frame();
    flash.set(1, 1, 0, 0xf00);
    let frame = stage.frame();
    assert!(flashes(&frame, &plain, (32, 32), [120, 0, 0, 255]));
    let (tinted, before) = (frame.pixel(40, 40), plain.pixel(40, 40));
    assert!(tinted[0] > before[0], "redder");
    assert_eq!(tinted[1..], before[1..]);

    // Four bits each for red, green and blue, times 8 at the strongest point; bits 12 to
    // 14 are not part of the colour.
    flash.set(1, 1, 0, 0x48c);
    assert!(flashes(&stage.frame(), &plain, (32, 32), [32, 64, 96, 255]));
    flash.set(1, 1, 0, 0x7000 + 0x00f);
    assert!(flashes(&stage.frame(), &plain, (32, 32), [0, 0, 120, 255]));
    // A negative entry, and one without a colour, draw nothing.
    for entry in [-1, -2, -0x0edd, -0x8000, 0x1000, 0x7000] {
        flash.set(1, 1, 0, entry);
        assert_eq!(stage.frame().rgba, plain.rgba, "{entry}");
    }

    // Back at zero, and without flash data, the tilemap is drawn as before.
    flash.set(1, 1, 0, 0);
    assert_eq!(stage.frame().rgba, plain.rgba);
    flash.set(1, 1, 0, 0xf00);
    stage.scene.tilemap_mut(id).unwrap().flash_data = None;
    assert_eq!(stage.frame().rgba, plain.rgba);
    let stored = stage.scene.tilemap(id).unwrap().flash_data.clone();
    assert_eq!(stored, None);
    stage.assert_pinned("a_cell_with_a_flash_entry_is_tinted_and_its_neighbours_are_not");
}

/// The lines of part `part` of the samples measured on the original runtime.
fn flash_samples(part: u32) -> impl Iterator<Item = &'static str> {
    const SAMPLES: &str = include_str!("../../tests/data/tilemap-flash-samples.txt");
    let from = SAMPLES.find(&format!("# Part {part}:")).unwrap();
    let lines = SAMPLES[from..].lines().skip(1);
    lines
        .take_while(|line| !line.starts_with("# Part"))
        .filter(|line| !line.starts_with('#'))
}

/// The numbers in `text`, in order.
fn numbers(text: &str) -> Vec<i32> {
    let words = text.split(|c: char| !c.is_ascii_digit() && c != '-');
    words.filter_map(|word| word.parse().ok()).collect()
}

/// A stage of one cell without a tile, over a sprite of the colour `below` with z -1, whose
/// flash data holds `entry`, after `updates` updates.
fn flashing_cell(updates: u64, entry: i64, below: Rgba) -> (Stage, TilemapId) {
    let mut stage = Stage::sized(32, 32);
    let bitmap = stage.solid(32, 32, below);
    let sprite = stage.scene.create_sprite(None).unwrap();
    let sprite_mut = stage.scene.sprite_mut(sprite).unwrap();
    sprite_mut.set_bitmap(Some(bitmap), &stage.bitmaps);
    stage.scene.set_sprite_z(sprite, -1).unwrap();
    let id = stage.scene.create_tilemap(None).unwrap();
    let flash = SharedTable::new(Table::new_2d(1, 1).unwrap());
    flash.set(0, 0, 0, entry);
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.map_data = Some(map(1, 1));
    tilemap.flash_data = Some(flash);
    for _ in 0..updates {
        tilemap.update();
    }
    (stage, id)
}

/// What a [`flashing_cell`] shows.
fn flashed(updates: u64, entry: i64, below: Rgba) -> Rgba {
    let (mut stage, _) = flashing_cell(updates, entry, below);
    let frame = stage.frame();
    assert!(all(&frame, frame.pixel(0, 0)));
    frame.pixel(0, 0)
}

#[test]
fn the_flash_pulses_with_the_updates_of_the_tilemap() {
    for (updates, added) in [(0, 60), (4, 75), (16, 120), (28, 75), (32, 60)] {
        let expected = [added, added, added, 255];
        assert_eq!(flashed(updates, 0xfff, BLACK), expected, "{updates}");
    }

    // Every value stays for 4 updates, and without an update it stays.
    let (mut stage, id) = flashing_cell(0, 0x888, BLACK);
    let mut reds = Vec::new();
    for _ in 0..80 {
        let red = stage.frame().pixel(8, 8)[0];
        assert_eq!(stage.frame().pixel(8, 8)[0], red);
        reds.push(red);
        // Neither new flash data nor a second tilemap starts the pulse again.
        let flash = SharedTable::new(Table::new_2d(1, 1).unwrap());
        flash.set(0, 0, 0, 0x888);
        stage.scene.tilemap_mut(id).unwrap().flash_data = Some(flash);
        stage.scene.create_tilemap(None).unwrap();
        stage.scene.tilemap_mut(id).unwrap().update();
    }
    let steps = [32, 40, 48, 56, 64, 56, 48, 40];
    assert!((0..80).all(|update| reds[update] == steps[update / 4 % 8]));
}

#[test]
fn the_pulse_is_the_measured_one() {
    let mut lines = 0;
    for line in flash_samples(1) {
        let sample = numbers(line);
        let (updates, added) = (sample[1] as u64, sample[2]);
        assert_eq!(15 * i32::from(flash_multiplier(updates)), added, "{line}");
        lines += 1;
    }
    assert_eq!(lines, 160);
}

#[test]
fn flashing_cells_show_the_measured_colours() {
    // Spelled out: bits 12 to 14 are ignored, the sum stops at 255, a negative entry and
    // one without a colour change nothing.
    let below = [50, 100, 150, 255];
    assert_eq!(flashed(4, 0x7abc, below), [100, 155, 210, 255]);
    assert_eq!(flashed(4, 0xfff, below), [125, 175, 225, 255]);
    assert_eq!(flashed(44, 0xf00, [204, 51, 110, 255]), [255, 51, 110, 255]);
    assert_eq!(flashed(1, 0xfff, GREEN), [60, 255, 60, 255]);
    assert_eq!(flashed(4, -1, below), below);
    assert_eq!(flashed(1, 0x1000, below), below);

    let channel = |value: i32| u8::try_from(value).unwrap();
    let mut lines = 0;
    for line in flash_samples(2) {
        let sample = numbers(line);
        let below = [sample[3], sample[4], sample[5], 255].map(channel);
        let shown = [sample[6], sample[7], sample[8], 255].map(channel);
        let found = flashed(sample[1] as u64, i64::from(sample[2]), below);
        assert_eq!(found, shown, "{line}");
        lines += 1;
    }
    assert_eq!(lines, 400);
    // After 4 updates over one colour.
    for line in flash_samples(3) {
        let (entry, shown) = line.split_once("->").unwrap();
        let (entry, shown) = (numbers(entry)[0], numbers(shown));
        let shown = [shown[0], shown[1], shown[2], 255].map(channel);
        let found = flashed(4, i64::from(entry), below);
        assert_eq!(found, shown, "{line}");
        lines += 1;
    }
    assert_eq!(lines, 408);
}

/// A stage with a map of 7 by 5 cells without tiles, scrolled to `origin`, whose flash data
/// of `size` names its cell in each entry: column and row, plus one, in red and green.
fn flash_of_another_size(size: (i32, i32), origin: (i32, i32)) -> Frame {
    let mut stage = Stage::new();
    let id = stage.scene.create_tilemap(None).unwrap();
    let flash = SharedTable::new(Table::new_2d(size.0, size.1).unwrap());
    for y in 0..size.1 {
        for x in 0..size.0 {
            flash.set(x, y, 0, i64::from((x + 1) << 8 | (y + 1) << 4 | 8));
        }
    }
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.map_data = Some(map(7, 5));
    tilemap.flash_data = Some(flash);
    (tilemap.ox, tilemap.oy) = origin;
    stage.frame()
}

/// The cell of the flash data of a [`flash_of_another_size`] that the pixel shows.
fn flash_cell(frame: &Frame, x: i32, y: i32) -> (i32, i32) {
    let pixel = frame.pixel(x as u32, y as u32);
    assert_eq!((pixel[0] % 4, pixel[1] % 4, pixel[2]), (0, 0, 32));
    (i32::from(pixel[0]) / 4 - 1, i32::from(pixel[1]) / 4 - 1)
}

/// The columns of the flash data that the first `count` columns of the screen show, from
/// the one that holds pixel `ox` on.
fn flash_columns(size: (i32, i32), ox: i32, count: i32) -> Vec<i32> {
    let frame = flash_of_another_size(size, (ox, 0));
    let columns = (0..count).map(|column| column * 32 + 31 - ox.rem_euclid(32));
    columns.map(|x| flash_cell(&frame, x, 0).0).collect()
}

/// [`flash_columns`] for the rows.
fn flash_rows(size: (i32, i32), oy: i32, count: i32) -> Vec<i32> {
    let frame = flash_of_another_size(size, (0, oy));
    let rows = (0..count).map(|row| row * 32 + 31 - oy.rem_euclid(32));
    rows.map(|y| flash_cell(&frame, 0, y).1).collect()
}

#[test]
fn flash_data_of_another_size_is_counted_on_from_the_first_cell_of_the_map() {
    for (ox, first) in [(0, 0), (40, 1), (264, 1), (-50, 0), (-300, 4)] {
        let counted: Vec<i32> = (0..20).map(|column| (first + column) % 5).collect();
        assert_eq!(flash_columns((5, 4), ox, 20), counted, "ox {ox}");
    }
    assert_eq!(flash_columns((9, 6), -300, 7), [4, 5, 6, 7, 8, 0, 1]);
    assert_eq!(flash_rows((9, 6), -200, 5), [3, 4, 5, 0, 1]);
    // Of the same size as the map, the flash data gives the map's cell.
    assert_eq!(flash_columns((7, 5), 264, 8), [1, 2, 3, 4, 5, 6, 0, 1]);
    assert_eq!(flash_rows((7, 5), -200, 6), [3, 4, 0, 1, 2, 3]);
}

#[test]
fn flash_data_of_another_size_shows_the_measured_cells() {
    let mut lines = 0;
    for line in flash_samples(4) {
        // Sizes of the flash data, origin, where the first whole cell is on the screen,
        // then the columns of the 12 whole cells of the top row and the rows of those of
        // the left column.
        let sample = numbers(line);
        let (size, origin) = ((sample[0], sample[1]), (sample[2], sample[3]));
        let (left, top) = (sample[4], sample[5]);
        let frame = flash_of_another_size(size, origin);
        for (cell, (&column, &row)) in sample[6..18].iter().zip(&sample[18..30]).enumerate() {
            let step = cell as i32 * 32;
            assert_eq!(flash_cell(&frame, left + step, top).0, column, "{line}");
            assert_eq!(flash_cell(&frame, left, top + step).1, row, "{line}");
        }
        lines += 1;
    }
    assert_eq!(lines, 40);
}

#[test]
fn the_flash_overlay_scrolls_with_its_cell() {
    let (mut stage, id, flash) = flashing();
    let mut plain = Vec::new();
    let origins = [(0, 0), (16, 0), (16, -8), (40, 0), (-96, 64), (70, 70)];
    for (ox, oy) in origins {
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        (tilemap.ox, tilemap.oy) = (ox, oy);
        plain.push(stage.frame());
    }
    flash.set(1, 1, 0, 0x0f0);
    for ((ox, oy), plain) in origins.into_iter().zip(&plain) {
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        (tilemap.ox, tilemap.oy) = (ox, oy);
        let frame = stage.frame();
        // The map repeats every 96 by 64 pixels, and the flash data with it.
        let at = ((32 - ox).rem_euclid(96), (32 - oy).rem_euclid(64));
        let tinted = [
            at,
            (at.0 - 96, at.1),
            (at.0, at.1 - 64),
            (at.0 - 96, at.1 - 64),
        ]
        .into_iter()
        .fold(plain.clone(), |shown, corner| {
            let mut next = shown.clone();
            for y in 0..64 {
                for x in 0..96 {
                    if flashes_at(corner, x, y) {
                        let pixel = vkxp_pixel::blend_add(shown.pixel(x, y), [0, 120, 0, 255], 255);
                        let offset = ((y * 96 + x) * 4) as usize;
                        next.rgba[offset..offset + 4].copy_from_slice(&pixel);
                    }
                }
            }
            next
        });
        assert_eq!(frame.rgba, tinted.rgba, "origin ({ox}, {oy})");
        assert_ne!(frame.rgba, plain.rgba, "origin ({ox}, {oy})");
    }
    stage.assert_pinned("the_flash_overlay_scrolls_with_its_cell");
}

/// Whether (`x`, `y`) is among the 32 by 32 pixels at `corner`.
fn flashes_at(corner: (i32, i32), x: u32, y: u32) -> bool {
    (corner.0..corner.0 + 32).contains(&(x as i32))
        && (corner.1..corner.1 + 32).contains(&(y as i32))
}

#[test]
fn the_flash_overlay_is_above_the_ground_and_below_tiles_with_a_priority() {
    let (mut stage, id, flash) = flashing();
    let priorities = SharedTable::new(Table::new(384 + TILES).unwrap());
    priorities.set(384 + 100, 0, 0, 1);
    stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities);
    let map = stage.scene.tilemap(id).unwrap().map_data.clone().unwrap();
    map.set(2, 0, 1, 384 + 100);
    let plain = stage.frame();
    flash.set(1, 0, 0, 0xfff);
    flash.set(2, 0, 0, 0xfff);
    let frame = stage.frame();
    assert!(flashes(&frame, &plain, (32, 0), [120, 120, 120, 255]));
    assert!(tile_shows(&frame, 64, 0, |x, y| tile_pixel(484, x, y)));
    stage.assert_pinned("the_flash_overlay_is_above_the_ground_and_below_tiles_with_a_priority");
}

#[test]
fn the_flash_overlay_is_given_back_with_the_last_flashing_cell() {
    let (mut stage, id, flash) = flashing();
    stage.frame();
    let without = stage.bitmaps.len();
    flash.set(2, 1, 0, 0x00f);
    stage.frame();
    assert_eq!(stage.bitmaps.len(), without + 1);
    // Unchanged, the overlay is not uploaded again.
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert!(packet.bitmaps.is_empty());
    flash.set(2, 1, 0, 0);
    stage.frame();
    assert_eq!(stage.bitmaps.len(), without);
    flash.set(2, 1, 0, 0x00f);
    stage.frame();
    stage.scene.tilemap_mut(id).unwrap().visible = false;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), without, "the overlay");
    stage.assert_pinned("the_flash_overlay_is_given_back_with_the_last_flashing_cell");
}
