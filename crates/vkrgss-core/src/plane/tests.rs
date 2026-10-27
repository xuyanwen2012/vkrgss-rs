use vkrgss_compose_ref::Frame;
use vkrgss_pixel::Rgba;

use super::*;
use crate::bitmap::testing::color;
use crate::extract::testing::*;
use crate::{Color, PlaneId, Rules, Tone};

/// The pixel at (`x`, `y`) of a [`pattern`] bitmap: different from its neighbours.
fn texel(x: u32, y: u32) -> Rgba {
    [(x % 251) as u8 + 1, (y % 241) as u8 + 1, 9, 255]
}

fn pattern(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&texel(x, y));
        }
    }
    stage.bitmap(width, height, rgba)
}

/// A plane on the screen showing `bitmap`.
fn plane(stage: &mut Stage, bitmap: BitmapId) -> PlaneId {
    let id = stage.scene.create_plane(None).unwrap();
    stage.scene.plane_mut(id).unwrap().bitmap = Some(bitmap);
    id
}

/// Whether every pixel of the frame is `expected` of its position.
fn shows(frame: &Frame, expected: impl Fn(u32, u32) -> Rgba) -> bool {
    (0..frame.size.height)
        .all(|y| (0..frame.size.width).all(|x| frame.pixel(x, y) == expected(x, y)))
}

fn quads(packet: &vkrgss_proto::FramePacket) -> usize {
    let quad = |op: &&vkrgss_proto::Op| matches!(op, vkrgss_proto::Op::Quad(_));
    packet.ops.iter().filter(quad).count()
}

#[test]
fn a_plane_repeats_its_bitmap_over_the_screen() {
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 64, 64);
    plane(&mut stage, bitmap);
    assert!(shows(&stage.frame(), |x, y| texel(x % 64, y % 64)));
}

#[test]
fn a_large_bitmap_is_drawn_without_a_pretiled_copy() {
    let mut stage = Stage::sized(64, 48);
    let bitmap = pattern(&mut stage, 40, 30);
    let id = plane(&mut stage, bitmap);
    stage.scene.plane_mut(id).unwrap().ox = 7;
    assert!(shows(&stage.frame(), |x, y| texel((x + 7) % 40, y % 30)));
    assert_eq!(stage.bitmaps.len(), 1);
}

#[test]
fn the_origin_scrolls_a_plane() {
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 64, 64);
    let id = plane(&mut stage, bitmap);
    let plane = stage.scene.plane_mut(id).unwrap();
    (plane.ox, plane.oy) = (10, 20);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), texel(10, 20));
    assert!(shows(&frame, |x, y| texel((x + 10) % 64, (y + 20) % 64)));

    let plane = stage.scene.plane_mut(id).unwrap();
    (plane.ox, plane.oy) = (-1, -1);
    let frame = stage.frame();
    assert_eq!(frame.pixel(0, 0), texel(63, 63));
    assert_eq!(frame.pixel(1, 1), texel(0, 0));
    assert!(shows(&frame, |x, y| texel((x + 63) % 64, (y + 63) % 64)));

    // An origin far away is the same origin modulo the bitmap.
    let plane = stage.scene.plane_mut(id).unwrap();
    (plane.ox, plane.oy) = (i32::MAX, i32::MIN);
    let frame = stage.frame();
    let (ox, oy) = (
        i32::MAX.rem_euclid(64) as u32,
        i32::MIN.rem_euclid(64) as u32,
    );
    assert!(shows(&frame, |x, y| texel((x + ox) % 64, (y + oy) % 64)));
}

/// The lines of `plane-samples.txt`: the source column and row that the original runtime
/// showed at the first 60 columns and rows of a viewport of 320 by 240.
#[test]
fn planes_show_the_measured_texels() {
    let numbers = |text: &str| -> Vec<f64> {
        let words = text.split_whitespace();
        words.map(|word| word.parse().unwrap()).collect()
    };
    let samples = include_str!("../../tests/data/plane-samples.txt");
    let mut checked = 0;
    for line in samples.lines().filter(|line| !line.starts_with('#')) {
        let parts: Vec<Vec<f64>> = line.split('|').map(numbers).collect();
        // The viewport reaches beyond this screen; its columns count from its own corner.
        let mut stage = Stage::sized(80, 72);
        let bitmap = pattern(&mut stage, parts[0][0] as u32, parts[0][1] as u32);
        let area = Rect::new(8, 4, 320, 240);
        let viewport = stage.scene.create_viewport(area).unwrap();
        let view = stage.scene.viewport_mut(viewport).unwrap();
        (view.ox, view.oy) = (parts[3][0] as i32, parts[3][1] as i32);
        let id = stage.scene.create_plane(Some(viewport)).unwrap();
        let plane = stage.scene.plane_mut(id).unwrap();
        plane.bitmap = Some(bitmap);
        (plane.zoom_x, plane.zoom_y) = (parts[1][0], parts[1][1]);
        (plane.ox, plane.oy) = (parts[2][0] as i32, parts[2][1] as i32);
        let frame = stage.frame();
        let (columns, rows) = (&parts[4], &parts[5]);
        assert_eq!((columns.len(), rows.len()), (60, 60), "{line}");
        for (j, row) in rows.iter().enumerate() {
            for (i, column) in columns.iter().enumerate() {
                assert_eq!(
                    frame.pixel(8 + i as u32, 4 + j as u32),
                    texel(*column as u32, *row as u32),
                    "column {i}, row {j} of {line}"
                );
            }
        }
        checked += 1;
    }
    assert!(checked >= 20);
}

/// Without a viewport the grid begins at the corner of the screen, and the origin counts
/// in screen pixels whatever the zoom.
#[test]
fn the_origin_of_a_zoomed_plane_counts_screen_pixels() {
    let mut stage = Stage::sized(64, 48);
    let bitmap = pattern(&mut stage, 13, 7);
    let id = plane(&mut stage, bitmap);
    let plane = stage.scene.plane_mut(id).unwrap();
    (plane.zoom_x, plane.zoom_y) = (2.5, 1.7);
    (plane.ox, plane.oy) = (-5, 100);
    // Tiles of 32 by 11 pixels.
    let (step_x, step_y) = ((13u64 << 20) / 32, (7u64 << 20) / 11);
    assert!(shows(&stage.frame(), |x, y| {
        let column = (i64::from(x) - 5).rem_euclid(32) as u64;
        let row = (i64::from(y) + 100).rem_euclid(11) as u64;
        texel(
            ((column * step_x) >> 20) as u32,
            ((row * step_y) >> 20) as u32,
        )
    }));
}

/// Scenario "Measured on the original" of "Plane tile size of RGSS2".
#[test]
fn the_tile_is_sized_in_double_precision_in_rgss2() {
    for (rules, rows) in [(Rules::Rgss2, 62), (Rules::Rgss1, 63)] {
        let mut stage = Stage::with_rules(rules, 4, 200);
        let bitmap = pattern(&mut stage, 4, 90);
        let id = plane(&mut stage, bitmap);
        stage.scene.plane_mut(id).unwrap().zoom_y = 0.7;
        let step = (90u64 << 20) / rows;
        assert!(
            shows(&stage.frame(), |x, y| {
                texel(x, ((u64::from(y) % rows * step) >> 20) as u32)
            }),
            "{rules:?}"
        );
    }
}

#[test]
fn zoom_scales_a_plane_about_the_corner_of_its_area() {
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 64, 64);
    let id = plane(&mut stage, bitmap);
    let plane = stage.scene.plane_mut(id).unwrap();
    (plane.zoom_x, plane.zoom_y) = (2.0, 2.0);
    let frame = stage.frame();
    for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(frame.pixel(x, y), texel(0, 0));
    }
    assert_eq!(frame.pixel(2, 2), texel(1, 1));
    assert!(shows(&frame, |x, y| texel(x / 2 % 64, y / 2 % 64)));
    assert_eq!(frame.pixel(128, 128), frame.pixel(0, 0));
    assert_eq!(frame.pixel(131, 259), frame.pixel(3, 3));
}

#[test]
fn repeats_meet_without_seams_at_a_fractional_zoom() {
    for (width, height) in [(7, 5), (64, 64), (200, 150)] {
        let mut stage = Stage::new();
        let back = stage.solid(640, 480, [40, 80, 120, 255]);
        let sprite = stage.scene.create_sprite(None).unwrap();
        let sprite = stage.scene.sprite_mut(sprite).unwrap();
        sprite.set_bitmap(Some(back), &stage.bitmaps);

        let veil = stage.solid(width, height, [255, 255, 255, 128]);
        let id = plane(&mut stage, veil);
        stage.scene.set_plane_z(id, 1).unwrap();
        let plane = stage.scene.plane_mut(id).unwrap();
        (plane.zoom_x, plane.zoom_y) = (1.5, 1.5);
        (plane.ox, plane.oy) = (3, 2);
        let frame = stage.frame();
        let pixel = frame.pixel(0, 0);
        assert_ne!(pixel, [40, 80, 120, 255]);
        assert!(all(&frame, pixel), "{width} by {height}");
    }
}

#[test]
fn a_plane_in_a_viewport_fills_the_rectangle_of_the_viewport() {
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 64, 64);
    let area = Rect::new(100, 50, 200, 100);
    let viewport = stage.scene.create_viewport(area).unwrap();
    let id = stage.scene.create_plane(Some(viewport)).unwrap();
    let plane = stage.scene.plane_mut(id).unwrap();
    plane.bitmap = Some(bitmap);
    (plane.ox, plane.oy) = (5, 6);
    let frame = stage.frame();
    assert_eq!(frame.pixel(100, 50), texel(5, 6));
    assert!(shows(&frame, |x, y| {
        if (100..300).contains(&x) && (50..150).contains(&y) {
            texel((x - 100 + 5) % 64, (y - 50 + 6) % 64)
        } else {
            BLACK
        }
    }));
}

#[test]
fn opacity_blend_type_colour_and_tone_act_as_those_of_a_sprite() {
    let looks: [(i32, i32, Color, Tone); 4] = [
        (128, 0, Color::TRANSPARENT, Tone::NEUTRAL),
        (255, 1, Color::TRANSPARENT, Tone::NEUTRAL),
        (200, 2, Color::TRANSPARENT, Tone::NEUTRAL),
        (
            255,
            0,
            Color::new(255.0, 0.0, 0.0, 100.0),
            Tone::new(-40.0, 30.0, 0.0, 128.0),
        ),
    ];
    for (opacity, blend_type, colour, tone) in looks {
        let frame = |as_plane: bool| {
            let mut stage = Stage::sized(16, 12);
            let back = stage.solid(16, 12, [90, 60, 30, 255]);
            let sprite = stage.scene.create_sprite(None).unwrap();
            let sprite = stage.scene.sprite_mut(sprite).unwrap();
            sprite.set_bitmap(Some(back), &stage.bitmaps);
            let bitmap = pattern(&mut stage, 16, 12);
            if as_plane {
                let id = plane(&mut stage, bitmap);
                stage.scene.set_plane_z(id, 1).unwrap();
                let plane = stage.scene.plane_mut(id).unwrap();
                plane.set_opacity(opacity);
                plane.blend_type = blend_type;
                plane.color = colour;
                plane.tone = tone;
            } else {
                let id = stage.scene.create_sprite(None).unwrap();
                stage.scene.set_sprite_z(id, 1).unwrap();
                let sprite = stage.scene.sprite_mut(id).unwrap();
                sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
                sprite.set_opacity(opacity);
                sprite.blend_type = blend_type;
                sprite.color = colour;
                sprite.tone = tone;
            }
            stage.frame()
        };
        let (plane, sprite) = (frame(true), frame(false));
        assert_ne!(plane.pixel(3, 3), [90, 60, 30, 255]);
        assert_eq!(plane.rgba, sprite.rgba, "blend type {blend_type}");
    }
}

#[test]
fn a_plane_is_ordered_by_z_among_the_other_elements() {
    let mut stage = Stage::sized(16, 16);
    let red = stage.solid(4, 4, RED);
    let green = stage.solid(16, 16, GREEN);
    let sprite = stage.scene.create_sprite(None).unwrap();
    let sprite_mut = stage.scene.sprite_mut(sprite).unwrap();
    sprite_mut.set_bitmap(Some(red), &stage.bitmaps);
    let id = plane(&mut stage, green);
    assert_eq!(stage.frame().pixel(0, 0), GREEN, "created later");
    stage.scene.set_sprite_z(sprite, 1).unwrap();
    let frame = stage.frame();
    assert_eq!((frame.pixel(0, 0), frame.pixel(8, 8)), (RED, GREEN));
    stage.scene.set_plane_z(id, 2).unwrap();
    assert!(all(&stage.frame(), GREEN));
}

#[test]
fn a_plane_that_has_nothing_to_show_draws_nothing() {
    let mut stage = Stage::sized(32, 24);
    let bitmap = stage.solid(8, 8, RED);
    let id = plane(&mut stage, bitmap);
    assert!(all(&stage.frame(), RED));

    stage.scene.plane_mut(id).unwrap().visible = false;
    assert!(all(&stage.frame(), BLACK));
    let plane = stage.scene.plane_mut(id).unwrap();
    plane.visible = true;
    plane.set_opacity(0);
    assert!(all(&stage.frame(), BLACK));
    let plane = stage.scene.plane_mut(id).unwrap();
    plane.set_opacity(255);
    plane.zoom_x = 0.0;
    assert!(all(&stage.frame(), BLACK));
    let plane = stage.scene.plane_mut(id).unwrap();
    plane.zoom_x = 1.0;
    plane.zoom_y = f64::NAN;
    assert!(all(&stage.frame(), BLACK));
    let plane = stage.scene.plane_mut(id).unwrap();
    plane.zoom_y = 1.0;
    plane.bitmap = None;
    assert!(all(&stage.frame(), BLACK));
    stage.scene.plane_mut(id).unwrap().bitmap = Some(bitmap);
    assert!(all(&stage.frame(), RED));
    stage.bitmaps.dispose(bitmap);
    assert!(all(&stage.frame(), BLACK));
}

#[test]
fn a_small_bitmap_is_drawn_from_a_pretiled_copy() {
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 4, 4);
    let id = plane(&mut stage, bitmap);
    let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
    assert!((1..=64).contains(&quads(&packet)), "{}", quads(&packet));
    let frame = stage.render(&packet).unwrap();
    assert!(shows(&frame, |x, y| texel(x % 4, y % 4)));

    // The copy follows the source.
    let changed: Rgba = [200, 100, 50, 255];
    stage
        .bitmaps
        .set_pixel(bitmap, 1, 2, color(changed))
        .unwrap();
    let expected = |x: u32, y: u32| match (x % 4, y % 4) {
        (1, 2) => changed,
        (x, y) => texel(x, y),
    };
    assert!(shows(&stage.frame(), expected));

    // So do origin and zoom: a zoom of 2 has an exact step, so copies in a row stretch
    // as the source does. The origin counts screen pixels.
    let plane = stage.scene.plane_mut(id).unwrap();
    (plane.ox, plane.oy) = (-3, 9);
    (plane.zoom_x, plane.zoom_y) = (2.0, 1.0);
    let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
    assert!(quads(&packet) <= 64);
    let frame = stage.render(&packet).unwrap();
    assert!(shows(&frame, |x, y| expected((x + 5) % 8 / 2, (y + 9) % 4)));
}

#[test]
fn a_plane_with_exact_steps_never_needs_more_than_64_quads() {
    let zooms = [1.0, 0.5, 0.25, 2.0, 4.0, 32.0];
    for (width, height) in [(1, 1), (4, 4), (3, 200), (64, 64), (100, 8), (640, 480)] {
        for zoom in zooms {
            let mut stage = Stage::new();
            let bitmap = pattern(&mut stage, width, height);
            let id = plane(&mut stage, bitmap);
            let plane = stage.scene.plane_mut(id).unwrap();
            (plane.zoom_x, plane.zoom_y) = (zoom, zoom);
            (plane.ox, plane.oy) = (width as i32 - 1, 1);
            let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
            let count = quads(&packet);
            assert!(
                count <= 64,
                "{count} quads for {width} by {height} at {zoom}"
            );
            let frame = stage.render(&packet).unwrap();
            // A tile of less than one pixel draws nothing.
            let shows = f64::from(width.min(height)) * zoom >= 1.0;
            let drawn = frame.rgba.as_chunks::<4>().0;
            assert_eq!(
                drawn.iter().all(|pixel| *pixel != BLACK),
                shows,
                "{width} by {height} at {zoom} covers the screen"
            );
            assert_eq!(drawn.iter().all(|pixel| *pixel == BLACK), !shows);
        }
    }
}

/// A step that is not exact allows no pre-tiled copy: every repeat is a quad of its own.
#[test]
fn a_plane_emits_at_most_one_quad_per_pixel_it_covers() {
    // The worst cases on the largest screen of the binding: a tile of one pixel, from a 1 by
    // 1 bitmap at zoom 1 and from a 300 by 300 bitmap at zoom 0.01.
    for rules in [Rules::Rgss1, Rules::Rgss2] {
        for (side, zoom) in [(1, 1.0), (300, 0.01)] {
            let mut stage = Stage::with_rules(rules, 640, 480);
            let bitmap = pattern(&mut stage, side, side);
            let id = plane(&mut stage, bitmap);
            let zoomed = stage.scene.plane_mut(id).unwrap();
            (zoomed.zoom_x, zoomed.zoom_y) = (zoom, zoom);
            for _ in 0..2 {
                let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
                let count = quads(&packet);
                assert!(count <= 640 * 480, "{count} quads for {side} at {zoom}");
            }
        }
    }
    // Inexact steps, which draw every repeat from the source, at any origin on a small
    // screen.
    for side in [3, 4, 7, 300] {
        for zoom in [0.75, 0.43, 1.5, 0.01, 0.0034, 1.0 / 3.0] {
            for origin in [0, 1, -5, 1000] {
                let mut stage = Stage::sized(64, 48);
                let bitmap = pattern(&mut stage, side, side);
                let id = plane(&mut stage, bitmap);
                let zoomed = stage.scene.plane_mut(id).unwrap();
                (zoomed.zoom_x, zoomed.zoom_y) = (zoom, zoom);
                (zoomed.ox, zoomed.oy) = (origin, -origin);
                let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
                let count = quads(&packet);
                assert!(count <= 64 * 48, "{count} quads for {side} at {zoom}");
            }
        }
    }
}

#[test]
fn a_plane_with_an_inexact_step_draws_every_repeat_from_its_bitmap() {
    let mut stage = Stage::sized(160, 120);
    let bitmap = pattern(&mut stage, 4, 4);
    let id = plane(&mut stage, bitmap);
    let plane = stage.scene.plane_mut(id).unwrap();
    (plane.zoom_x, plane.zoom_y) = (1.5, 1.5);
    let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
    assert_eq!(stage.bitmaps.len(), 1);
    // Tiles of 6 by 6 pixels.
    assert_eq!(quads(&packet), 27 * 20);
    let step = (4u64 << 20) / 6;
    let tile = |i: u32| ((u64::from(i % 6) * step) >> 20) as u32;
    assert!(shows(&stage.render(&packet).unwrap(), |x, y| texel(
        tile(x),
        tile(y)
    )));
}

/// Tiles of 3 by 3 pixels with an inexact step: a quad for each, however many there are.
#[test]
fn a_plane_of_tiny_tiles_with_an_inexact_step_is_drawn_tile_by_tile() {
    let mut stage = Stage::sized(64, 48);
    let bitmap = pattern(&mut stage, 4, 4);
    let id = plane(&mut stage, bitmap);
    let zoomed = stage.scene.plane_mut(id).unwrap();
    (zoomed.zoom_x, zoomed.zoom_y) = (0.75, 0.75);
    (zoomed.ox, zoomed.oy) = (1, -1);
    let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
    assert_eq!(stage.bitmaps.len(), 1);
    assert_eq!(quads(&packet), 22 * 17);
    let step = (4u64 << 20) / 3;
    let tile = |i: u32| ((u64::from(i % 3) * step) >> 20) as u32;
    assert!(shows(&stage.render(&packet).unwrap(), |x, y| texel(
        tile(x + 1),
        tile(y + 2)
    )));

    // On the whole screen as well: every repeat that shows is a quad.
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 4, 4);
    let id = plane(&mut stage, bitmap);
    let zoomed = stage.scene.plane_mut(id).unwrap();
    (zoomed.zoom_x, zoomed.zoom_y) = (0.75, 0.75);
    let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
    assert_eq!(quads(&packet), 214 * 160);
}

#[test]
fn a_plane_that_would_need_too_large_a_copy_draws_every_repeat_from_its_bitmap() {
    // Tiles of 2 by 2 pixels have an exact step, but copies of 1024 texels a side for
    // them would be far too large: the plane goes without.
    let mut stage = Stage::sized(32, 32);
    let bitmap = pattern(&mut stage, 1024, 1024);
    let id = plane(&mut stage, bitmap);
    let zoomed = stage.scene.plane_mut(id).unwrap();
    (zoomed.zoom_x, zoomed.zoom_y) = (2.0 / 1024.0, 2.0 / 1024.0);
    (zoomed.ox, zoomed.oy) = (1, 0);
    let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
    assert_eq!(stage.bitmaps.len(), 1);
    assert_eq!(quads(&packet), 17 * 16);
    let frame = stage.render(&packet).unwrap();
    assert!(shows(&frame, |x, y| texel((x + 1) % 2 * 512, y % 2 * 512)));

    // A tile of one pixel on the whole screen: a quad for every pixel.
    let mut stage = Stage::new();
    let bitmap = stage.solid(32, 32, RED);
    let id = plane(&mut stage, bitmap);
    let zoomed = stage.scene.plane_mut(id).unwrap();
    (zoomed.zoom_x, zoomed.zoom_y) = (1.0 / 32.0, 1.0 / 32.0);
    let packet = stage.scene.extract(&mut stage.bitmaps).unwrap();
    assert_eq!(stage.bitmaps.len(), 1);
    assert_eq!(quads(&packet), 640 * 480);
    let covered = packet.ops.iter().all(|op| match op {
        vkrgss_proto::Op::Quad(quad) => (quad.clip.width, quad.clip.height) == (1, 1),
        _ => true,
    });
    assert!(covered);

    // Zoomed out less far, each screen pixel takes every fourth texel of a copy.
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 16, 16);
    let id = plane(&mut stage, bitmap);
    let zoomed = stage.scene.plane_mut(id).unwrap();
    (zoomed.zoom_x, zoomed.zoom_y) = (0.25, 0.25);
    let frame = stage.frame();
    assert_eq!(stage.bitmaps.len(), 2);
    assert!(shows(&frame, |x, y| texel(x % 4 * 4, y % 4 * 4)));
}

#[test]
fn an_invisible_plane_gives_its_copy_back_also_in_a_hidden_viewport() {
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 4, 4);
    let before = stage.bitmaps.len();
    let viewport = stage.scene.create_viewport(Rect::new(0, 0, 640, 480));
    let viewport = viewport.unwrap();
    let id = stage.scene.create_plane(Some(viewport)).unwrap();
    stage.scene.plane_mut(id).unwrap().bitmap = Some(bitmap);
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 1);

    // A visible plane in a hidden viewport keeps its copy.
    stage.scene.viewport_mut(viewport).unwrap().visible = false;
    assert!(all(&stage.frame(), BLACK));
    assert_eq!(stage.bitmaps.len(), before + 1);

    // An invisible one gives it back, whatever its viewport: hidden or off the screen.
    stage.scene.plane_mut(id).unwrap().visible = false;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before);

    stage.scene.viewport_mut(viewport).unwrap().visible = true;
    stage.scene.plane_mut(id).unwrap().visible = true;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 1);
    stage.scene.viewport_mut(viewport).unwrap().rect = Rect::new(700, 0, 64, 64);
    stage.scene.plane_mut(id).unwrap().visible = false;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before);

    stage.scene.viewport_mut(viewport).unwrap().rect = Rect::new(0, 0, 640, 480);
    stage.scene.plane_mut(id).unwrap().visible = true;
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 1);
    // A disposed viewport takes the plane along.
    stage.scene.dispose_viewport(viewport, &mut stage.bitmaps);
    assert!(stage.scene.is_plane_disposed(id));
    assert_eq!(stage.bitmaps.len(), before);
    assert!(all(&stage.frame(), BLACK));
    assert_eq!(stage.bitmaps.len(), before);
}

#[test]
fn the_pretiled_copy_is_disposed_with_the_plane_or_when_it_is_not_needed() {
    let mut stage = Stage::new();
    let bitmap = pattern(&mut stage, 4, 4);
    let large = pattern(&mut stage, 200, 200);
    let before = stage.bitmaps.len();
    let id = plane(&mut stage, bitmap);
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 1);

    // Redrawing the copy does not let copies pile up.
    stage.bitmaps.set_pixel(bitmap, 0, 0, color(RED)).unwrap();
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 1);

    // An invisible plane keeps no copy.
    stage.scene.plane_mut(id).unwrap().visible = false;
    stage.bitmaps.set_pixel(bitmap, 0, 0, color(GREEN)).unwrap();
    assert!(all(&stage.frame(), BLACK));
    assert_eq!(stage.bitmaps.len(), before);
    stage.scene.plane_mut(id).unwrap().visible = true;
    assert_eq!(stage.frame().pixel(0, 0), GREEN);
    assert_eq!(stage.bitmaps.len(), before + 1);

    // A bitmap that needs no copy gives it back.
    stage.scene.plane_mut(id).unwrap().bitmap = Some(large);
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before);
    stage.scene.plane_mut(id).unwrap().bitmap = Some(bitmap);
    stage.frame();
    assert_eq!(stage.bitmaps.len(), before + 1);

    stage.scene.dispose_plane(id, &mut stage.bitmaps);
    assert_eq!(stage.bitmaps.len(), before);
    assert!(!stage.bitmaps.is_disposed(bitmap));
    assert!(all(&stage.frame(), BLACK));
}

/// Scenario "Plane tile and sprite width" of "Scene of RGSS3": the plane.
#[test]
fn the_tile_is_sized_in_double_precision_in_rgss3() {
    let mut stage = Stage::with_rules(Rules::Rgss3, 4, 200);
    let bitmap = pattern(&mut stage, 4, 90);
    let id = plane(&mut stage, bitmap);
    stage.scene.plane_mut(id).unwrap().zoom_y = 0.7;
    let step = (90u64 << 20) / 62;
    assert!(shows(&stage.frame(), |x, y| {
        texel(x, ((u64::from(y) % 62 * step) >> 20) as u32)
    }));
}
