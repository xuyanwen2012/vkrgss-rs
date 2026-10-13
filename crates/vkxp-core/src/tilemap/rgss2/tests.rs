//! The scenarios of the tilemaps of RGSS2 and of RGSS3 of `specs/core-scene/spec.md`.

use vkxp_proto::{FramePacket, Op, Plane, PlaneRules, TableDelta, TilemapPlane};

use crate::extract::testing::*;
use crate::{Rect, SharedTable, Table, TilemapId, ViewportId};

/// A map of `width` by `height` cells and three layers, empty.
fn map(width: i32, height: i32) -> SharedTable {
    SharedTable::new(Table::new_3d(width, height, 3).unwrap())
}

/// A tilemap of RGSS2 with a map of 5 by 4 cells and one sheet of one colour in every slot.
fn tilemap(stage: &mut Stage, viewport: Option<ViewportId>) -> (TilemapId, SharedTable) {
    let sheet = stage.solid(512, 512, GREEN);
    let map = map(5, 4);
    let id = stage.scene.create_tilemap_rgss2(viewport).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.rgss2.as_mut().unwrap().bitmaps = [Some(sheet); 9];
    tilemap.map_data = Some(map.clone());
    stage.scene.refresh_tilemap(id).unwrap();
    (id, map)
}

fn planes(packet: &FramePacket) -> Vec<TilemapPlane> {
    let planes = packet.ops.iter().filter_map(|op| match op {
        Op::TilemapPlane(plane) => Some(*plane),
        _ => None,
    });
    planes.collect()
}

/// What kind each op of a packet is: a plane of a tilemap, or anything else.
fn kinds(packet: &FramePacket) -> Vec<Option<Plane>> {
    let kinds = packet.ops.iter().map(|op| match op {
        Op::TilemapPlane(plane) => Some(plane.plane),
        _ => None,
    });
    kinds.collect()
}

/// Scenario "Two planes, measured on the original".
#[test]
fn a_tilemap_is_a_lower_plane_at_z_0_and_an_upper_plane_at_z_200() {
    let mut stage = Stage::sized(96, 64);
    let (_, map) = tilemap(&mut stage, None);
    // Tile 37 in the third layer of cell (0, 0) has the star bit; tile 38 in the first
    // layer of cell (1, 0) does not.
    map.set(0, 0, 2, 37);
    map.set(1, 0, 0, 38);
    let passages = SharedTable::new(Table::new(64).unwrap());
    passages.set(37, 0, 0, 0x10);
    let id = stage.scene.tilemaps.iter().next().unwrap().0;
    let tilemap = stage.scene.tilemaps.get_mut(id).unwrap();
    tilemap.rgss2.as_mut().unwrap().passages = Some(passages);
    for (z, pixel) in [(100, RED), (300, BLUE)] {
        let bitmap = stage.solid(96, 64, pixel);
        let sprite = stage.scene.create_sprite(None).unwrap();
        let sprite_mut = stage.scene.sprite_mut(sprite).unwrap();
        sprite_mut.set_bitmap(Some(bitmap), &stage.bitmaps);
        stage.scene.set_sprite_z(sprite, z).unwrap();
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let expected = if z == 100 {
            vec![Some(Plane::Lower), None, Some(Plane::Upper)]
        } else {
            vec![Some(Plane::Lower), None, Some(Plane::Upper), None]
        };
        assert_eq!(kinds(&packet), expected);
        let frame = stage.render(&packet).unwrap();
        // The first sprite is under the star tile and over the other tile; the second is
        // over both.
        let expected = if z == 100 { [GREEN, RED] } else { [BLUE, BLUE] };
        assert_eq!([frame.pixel(5, 5), frame.pixel(40, 5)], expected);
    }
}

/// Scenario "Invisible, measured on the original".
#[test]
fn an_invisible_tilemap_is_drawn_as_before() {
    let mut stage = Stage::sized(96, 64);
    let (id, map) = tilemap(&mut stage, None);
    map.set(0, 0, 0, 37);
    let shown = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(planes(&shown).len(), 2);
    assert_eq!(stage.render(&shown).unwrap().pixel(5, 5), GREEN);
    stage.scene.tilemap_mut(id).unwrap().visible = false;
    let hidden = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(hidden.ops, shown.ops);
    assert_eq!(stage.render(&hidden).unwrap().pixel(5, 5), GREEN);
}

/// Scenario "Counter, measured on the original".
#[test]
fn the_counter_advances_by_updates_only() {
    let mut stage = Stage::sized(96, 64);
    let (id, _) = tilemap(&mut stage, None);
    let mut seen = Vec::new();
    for _ in 0..31 {
        // Two frames between the updates: frames do not count.
        for _ in 0..2 {
            let packet = stage.scene.extract(&mut stage.bitmaps);
            let [lower, upper] = planes(&packet)[..] else {
                panic!("two planes");
            };
            assert_eq!(
                (lower.animation_step, lower.flash_level),
                (upper.animation_step, upper.flash_level)
            );
            seen.push((lower.animation_step, lower.flash_level));
        }
        stage.scene.tilemap_mut(id).unwrap().update();
        stage.scene.refresh_tilemap(id).unwrap();
    }
    let steps: Vec<u64> = seen.iter().map(|(step, _)| *step).collect();
    assert_eq!(steps[..60], [0; 60]);
    assert_eq!(steps[60..], [1, 1]);
    // The flash level by the same counter: 4 for four updates, then 5, and so on.
    let levels: Vec<u8> = seen.iter().step_by(8).map(|(_, level)| *level).collect();
    assert_eq!(levels, [4, 5, 6, 7, 8, 7, 6, 5]);
}

/// Scenario "Scrolling".
#[test]
fn the_origin_gives_the_first_cell_and_the_corner() {
    let mut stage = Stage::sized(96, 64);
    let (id, _) = tilemap(&mut stage, None);
    let mut at = |ox: i32, oy: i32| {
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        (tilemap.ox, tilemap.oy) = (ox, oy);
        stage.scene.refresh_tilemap(id).unwrap();
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let plane = planes(&packet)[0];
        assert_eq!(plane.clip, vkxp_proto::Rect::new(0, 0, 96, 64));
        (plane.first_cell, plane.corner)
    };
    assert_eq!(at(0, 0), ((0, 0), (0, 0)));
    assert_eq!(at(-40, 0), ((3, 0), (-24, 0)));
    assert_eq!(at(33, 130), ((1, 0), (-1, -2)));
    assert_eq!(at(i32::MAX, i32::MIN), ((3, 0), (-31, 0)));
}

#[test]
fn the_planes_start_at_the_corner_of_the_viewport_and_its_origin_only_adds() {
    let mut stage = Stage::sized(200, 150);
    let viewport = stage.scene.create_viewport(Rect::new(40, 30, 100, 80));
    let viewport = viewport.unwrap();
    let (id, _) = tilemap(&mut stage, Some(viewport));
    stage.scene.tilemap_mut(id).unwrap().ox = 5;
    stage.scene.refresh_tilemap(id).unwrap();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let plane = planes(&packet)[0];
    assert_eq!(plane.clip, vkxp_proto::Rect::new(40, 30, 100, 80));
    assert_eq!((plane.first_cell, plane.corner), ((0, 0), (35, 30)));

    // A change of the viewport's origin refreshes the tilemap.
    let moved = stage.scene.viewport_mut(viewport).unwrap();
    (moved.ox, moved.oy) = (30, -3);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let plane = planes(&packet)[0];
    assert_eq!((plane.first_cell, plane.corner), ((1, 3), (37, 1)));
}

/// Scenario "Other viewport, measured on the original".
#[test]
fn another_viewport_is_not_a_refresh() {
    let mut stage = Stage::sized(200, 150);
    let old = stage.scene.create_viewport(Rect::new(0, 0, 100, 80));
    let new = stage.scene.create_viewport(Rect::new(40, 30, 100, 80));
    let (old, new) = (old.unwrap(), new.unwrap());
    stage.scene.viewport_mut(new).unwrap().ox = 16;
    let (id, _) = tilemap(&mut stage, Some(old));
    stage.scene.set_tilemap_viewport(id, Some(new)).unwrap();
    let corner = |stage: &mut Stage| planes(&stage.scene.extract(&mut stage.bitmaps))[0].corner;
    // In the rectangle of the new viewport, at the origin of the old one.
    assert_eq!(corner(&mut stage), (40, 30));
    assert_eq!(corner(&mut stage), (40, 30));
    // `ox = ox`.
    stage.scene.refresh_tilemap(id).unwrap();
    assert_eq!(corner(&mut stage), (24, 30));

    // Out of any viewport: still the origin of the one it left, until it is refreshed.
    stage.scene.set_tilemap_viewport(id, None).unwrap();
    assert_eq!(corner(&mut stage), (-16, 0));
    stage.scene.refresh_tilemap(id).unwrap();
    assert_eq!(corner(&mut stage), (0, 0));
}

#[test]
fn a_change_of_the_viewport_before_the_tilemap_leaves_it_is_latched() {
    let mut stage = Stage::sized(200, 150);
    let old = stage.scene.create_viewport(Rect::new(0, 0, 100, 80));
    let old = old.unwrap();
    let (id, _) = tilemap(&mut stage, Some(old));
    stage.scene.viewport_mut(old).unwrap().ox = 7;
    stage.scene.set_tilemap_viewport(id, None).unwrap();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(planes(&packet)[0].corner, (-7, 0));
}

#[test]
fn without_map_data_nothing_is_drawn_and_nothing_is_latched() {
    let mut stage = Stage::sized(96, 64);
    let viewport = stage.scene.create_viewport(Rect::new(0, 0, 96, 64));
    let viewport = viewport.unwrap();
    stage.scene.viewport_mut(viewport).unwrap().ox = 9;
    let id = stage.scene.create_tilemap_rgss2(Some(viewport)).unwrap();
    stage.scene.refresh_tilemap(id).unwrap();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert!(packet.ops.is_empty() && packet.tables.is_empty());
    // A map without cells draws nothing either.
    stage.scene.tilemap_mut(id).unwrap().map_data = Some(map(0, 0));
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert!(packet.ops.is_empty());
    // Moved while it had no map: the origin of the viewport it left was never taken.
    stage.scene.tilemap_mut(id).unwrap().map_data = None;
    stage.scene.set_tilemap_viewport(id, None).unwrap();
    stage.scene.tilemap_mut(id).unwrap().map_data = Some(map(2, 2));
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(planes(&packet)[0].corner, (0, 0));
}

#[test]
fn the_three_tables_are_sent_as_they_are_and_again_when_written() {
    let mut stage = Stage::sized(96, 64);
    let (id, map) = tilemap(&mut stage, None);
    let passages = SharedTable::new(Table::new(8192).unwrap());
    let flash = SharedTable::new(Table::new_2d(5, 4).unwrap());
    flash.set(1, 1, 0, 0xf00);
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.rgss2.as_mut().unwrap().passages = Some(passages.clone());
    tilemap.flash_data = Some(flash);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let sizes: Vec<(u32, u32, u32)> = packet
        .tables
        .iter()
        .map(|delta| match delta {
            TableDelta::Set {
                xsize,
                ysize,
                zsize,
                ..
            } => (*xsize, *ysize, *zsize),
            other => panic!("not a set: {other:?}"),
        })
        .collect();
    assert_eq!(sizes, [(5, 4, 3), (8192, 1, 1), (5, 4, 1)]);
    let [lower, upper] = planes(&packet)[..] else {
        panic!("two planes");
    };
    assert!(lower.passages.is_some() && lower.flash.is_some());
    assert_eq!(
        (lower.map, lower.passages, lower.flash),
        (upper.map, upper.passages, upper.flash)
    );
    // Nothing written: nothing sent. An element written: that table again.
    assert!(stage.scene.extract(&mut stage.bitmaps).tables.is_empty());
    map.set(0, 0, 0, 5);
    passages.set(5, 0, 0, 0x10);
    assert_eq!(stage.scene.extract(&mut stage.bitmaps).tables.len(), 2);
    // Disposed: the compositor gives the three tables back.
    stage.scene.dispose_tilemap(id, &mut stage.bitmaps);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert!(packet.ops.is_empty());
    let destroyed = |delta: &TableDelta| matches!(delta, TableDelta::Destroy { .. });
    assert_eq!(packet.tables.iter().filter(|d| destroyed(d)).count(), 3);
}

#[test]
fn a_disposed_sheet_is_left_out() {
    let mut stage = Stage::sized(96, 64);
    let (id, map) = tilemap(&mut stage, None);
    map.set(0, 0, 0, 37);
    let other = stage.solid(512, 512, RED);
    stage
        .scene
        .tilemap_mut(id)
        .unwrap()
        .rgss2
        .as_mut()
        .unwrap()
        .bitmaps[5] = Some(other);
    assert_eq!(stage.frame().pixel(5, 5), RED);
    stage.bitmaps.dispose(other);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(planes(&packet)[0].bitmaps[5], None);
    assert!(planes(&packet)[0].bitmaps[4].is_some());
    assert_eq!(stage.render(&packet).unwrap().pixel(5, 5), BLACK);
}

#[test]
fn a_tilemap_of_rgss1_stays_as_it_is() {
    let mut stage = Stage::sized(96, 64);
    let id = stage.scene.create_tilemap(None).unwrap();
    assert_eq!(stage.scene.tilemap(id).unwrap().rgss2, None);
    stage.scene.tilemap_mut(id).unwrap().map_data = Some(map(2, 2));
    stage.scene.refresh_tilemap(id).unwrap();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    assert!(planes(&packet).is_empty());
}

/// Scenario "Rule set of RGSS2".
#[test]
fn a_tilemap_of_rgss2_names_the_rule_set_of_rgss2() {
    let mut stage = Stage::sized(96, 64);
    tilemap(&mut stage, None);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let rules: Vec<PlaneRules> = planes(&packet).iter().map(|plane| plane.rules).collect();
    assert_eq!(rules, [PlaneRules::Rgss2; 2]);
}

/// A tilemap of RGSS3 with a map of `width` by `height` cells and four layers, a flags table
/// of 8192 elements and one sheet of one colour in every slot.
fn tilemap_rgss3(
    stage: &mut Stage,
    viewport: Option<ViewportId>,
    (width, height): (i32, i32),
) -> (TilemapId, SharedTable, SharedTable) {
    let sheet = stage.solid(512, 512, GREEN);
    let map = SharedTable::new(Table::new_3d(width, height, 4).unwrap());
    let flags = SharedTable::new(Table::new(8192).unwrap());
    let id = stage.scene.create_tilemap_rgss3(viewport).unwrap();
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    let rgss2 = tilemap.rgss2.as_mut().unwrap();
    rgss2.bitmaps = [Some(sheet); 9];
    rgss2.passages = Some(flags.clone());
    tilemap.map_data = Some(map.clone());
    stage.scene.refresh_tilemap(id).unwrap();
    (id, map, flags)
}

/// A sprite of one colour over the whole screen at `z`.
fn cover(stage: &mut Stage, size: (u32, u32), z: i32, pixel: vkxp_pixel::Rgba) {
    let bitmap = stage.solid(size.0, size.1, pixel);
    let sprite = stage.scene.create_sprite(None).unwrap();
    let sprite_mut = stage.scene.sprite_mut(sprite).unwrap();
    sprite_mut.set_bitmap(Some(bitmap), &stage.bitmaps);
    stage.scene.set_sprite_z(sprite, z).unwrap();
}

/// Scenario "Two planes of the rule set of RGSS3".
#[test]
fn a_tilemap_of_rgss3_is_two_planes_of_the_rule_set_of_rgss3() {
    let mut stage = Stage::sized(128, 96);
    let (_, map, _) = tilemap_rgss3(&mut stage, None, (4, 3));
    map.set(3, 2, 3, 15);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let [lower, upper] = planes(&packet)[..] else {
        panic!("two planes");
    };
    assert_eq!((lower.plane, upper.plane), (Plane::Lower, Plane::Upper));
    assert_eq!(
        (lower.rules, upper.rules),
        (PlaneRules::Rgss3, PlaneRules::Rgss3)
    );
    assert!(lower.passages.is_some());
    assert_eq!((lower.map, lower.passages), (upper.map, upper.passages));
    let sent = |id| {
        let found = packet.tables.iter().find_map(|delta| match delta {
            TableDelta::Set {
                id: set,
                xsize,
                ysize,
                zsize,
                data,
            } if *set == id => Some(((*xsize, *ysize, *zsize), data.clone())),
            _ => None,
        });
        found.expect("the table is sent")
    };
    let (sizes, data) = sent(lower.map);
    assert_eq!((sizes, data.len()), ((4, 3, 4), 48));
    assert_eq!((data[47], data[..47].iter().all(|&e| e == 0)), (15, true));
    assert_eq!(sent(lower.passages.unwrap()).0, (8192, 1, 1));
}

/// Scenario "Star tiles among sprites, measured on the original".
#[test]
fn a_star_tile_of_rgss3_is_above_a_sprite_only_from_the_third_layer() {
    let mut stage = Stage::sized(96, 64);
    let (_, map, flags) = tilemap_rgss3(&mut stage, None, (4, 3));
    map.set(0, 0, 2, 300);
    map.set(1, 0, 0, 300);
    flags.set(300, 0, 0, 0x10);
    for (z, pixel) in [(100, RED), (300, BLUE)] {
        cover(&mut stage, (96, 64), z, pixel);
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let frame = stage.render(&packet).unwrap();
        // The first sprite is under the tile of the third layer and over the tile of the
        // first; the second is over both.
        let expected = if z == 100 { [GREEN, RED] } else { [BLUE, BLUE] };
        assert_eq!([frame.pixel(5, 5), frame.pixel(40, 5)], expected);
    }
}

/// Scenario "A shadow from layer 3, measured on the original".
#[test]
fn the_fourth_layer_of_rgss3_darkens_what_lies_under_the_tilemap() {
    let mut stage = Stage::sized(128, 96);
    cover(&mut stage, (128, 96), -10, [200, 200, 200, 255]);
    let (_, map, _) = tilemap_rgss3(&mut stage, None, (4, 3));
    map.set(1, 1, 3, 15);
    let packet = stage.scene.extract(&mut stage.bitmaps);
    let frame = stage.render(&packet).unwrap();
    for y in 0..96 {
        for x in 0..128 {
            let level = if (32..64).contains(&x) && (32..64).contains(&y) {
                99
            } else {
                200
            };
            assert_eq!(frame.pixel(x, y), [level, level, level, 255], "({x}, {y})");
        }
    }
}

/// Scenario "Counter and scrolling as RGSS2".
#[test]
fn a_tilemap_of_rgss3_scrolls_and_counts_as_one_of_rgss2() {
    let mut stage = Stage::sized(96, 64);
    let (id, _, _) = tilemap_rgss3(&mut stage, None, (5, 4));
    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    tilemap.ox = -40;
    for _ in 0..30 {
        tilemap.update();
    }
    stage.scene.refresh_tilemap(id).unwrap();
    let packet = stage.scene.extract(&mut stage.bitmaps);
    for plane in planes(&packet) {
        assert_eq!((plane.first_cell, plane.corner), ((3, 0), (-24, 0)));
        assert_eq!(plane.animation_step, 1);
    }
}

/// Scenario "Latch as RGSS2, measured on the original".
#[test]
fn another_viewport_is_not_a_refresh_of_a_tilemap_of_rgss3() {
    let mut stage = Stage::sized(200, 150);
    let old = stage.scene.create_viewport(Rect::new(0, 0, 100, 80));
    let new = stage.scene.create_viewport(Rect::new(40, 30, 100, 80));
    let (old, new) = (old.unwrap(), new.unwrap());
    stage.scene.viewport_mut(new).unwrap().ox = 16;
    let (id, _, _) = tilemap_rgss3(&mut stage, Some(old), (5, 4));
    stage.scene.set_tilemap_viewport(id, Some(new)).unwrap();
    let corner = |stage: &mut Stage| planes(&stage.scene.extract(&mut stage.bitmaps))[0].corner;
    assert_eq!(corner(&mut stage), (40, 30));
    stage.scene.refresh_tilemap(id).unwrap();
    assert_eq!(corner(&mut stage), (24, 30));
}

/// Scenarios "Hidden, measured on the original" and "Back, measured on the original".
#[test]
fn a_hidden_tilemap_of_rgss3_has_no_planes_and_counts_on() {
    let mut stage = Stage::sized(128, 96);
    cover(&mut stage, (128, 96), -10, [200, 200, 200, 255]);
    let (id, map, flags) = tilemap_rgss3(&mut stage, None, (4, 3));
    map.set(0, 0, 0, 37);
    map.set(1, 0, 2, 300);
    flags.set(300, 0, 0, 0x10);
    map.set(2, 0, 3, 15);
    let flash = SharedTable::new(Table::new_2d(4, 3).unwrap());
    flash.set(3, 0, 0, 0xf00);
    stage.scene.tilemap_mut(id).unwrap().flash_data = Some(flash);
    stage.scene.refresh_tilemap(id).unwrap();

    let shown = stage.scene.extract(&mut stage.bitmaps);
    assert_eq!(planes(&shown).len(), 2);
    let frame = stage.render(&shown).unwrap();
    let cells = [5, 40, 70, 100].map(|x| frame.pixel(x, 5));
    assert_eq!(
        cells,
        [GREEN, GREEN, [99, 99, 99, 255], [255, 200, 200, 255]]
    );

    stage.scene.tilemap_mut(id).unwrap().visible = false;
    stage.scene.refresh_tilemap(id).unwrap();
    let hidden = stage.scene.extract(&mut stage.bitmaps);
    assert!(planes(&hidden).is_empty());
    // The tables stay with the compositor.
    assert!(hidden.tables.is_empty());
    let frame = stage.render(&hidden).unwrap();
    assert_eq!(
        [5, 40, 70, 100].map(|x| frame.pixel(x, 5)),
        [[200, 200, 200, 255]; 4]
    );

    let tilemap = stage.scene.tilemap_mut(id).unwrap();
    for _ in 0..30 {
        tilemap.update();
    }
    tilemap.visible = true;
    stage.scene.refresh_tilemap(id).unwrap();
    let back = stage.scene.extract(&mut stage.bitmaps);
    let steps: Vec<u64> = planes(&back).iter().map(|p| p.animation_step).collect();
    assert_eq!(steps, [1, 1]);
    assert!(back.tables.is_empty());
}
