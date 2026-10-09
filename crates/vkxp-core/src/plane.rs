//! Planes: a bitmap repeated without end over an area, drawn as one quad per repeat.
//!
//! As measured on the original runtime, a plane repeats a tile: its bitmap stretched to
//! `floor(width * zoom_x)` by `floor(height * zoom_y)` whole pixels by the rule of a stretch
//! quad. The tiles lie on a grid of whole screen pixels that the origins of the plane and of
//! its viewport shift, pixel by pixel whatever the zoom. Every repeat that shows is a stretch
//! quad over the whole bitmap.
//!
//! A bitmap that is small against the area would need many quads. Where that leaves every
//! pixel as it is, the plane then draws from a bitmap of the scene's own that repeats the
//! source a few times, kept in the bitmap arena and drawn again only when the source changes.
//! It does so along an axis whose stretch step is exact: only then is a stretch of several
//! copies the same as several stretches of one.

use vkxp_proto::{BitmapId, STRETCH_FRAC_BITS};

use crate::extract::{Assembly, Level, Look, blend_mode};
use crate::geometry::stretch_length;
use crate::{BitmapError, Bitmaps, Plane, Rect};

/// Most quads a plane is drawn with when it can draw from a pre-tiled bitmap.
const MAX_REPEATS: i64 = 64;
/// Most repeats along one axis once the plane draws from a pre-tiled bitmap.
const AXIS_REPEATS: i64 = 8;

/// Most pixels a pre-tiled bitmap may have. A plane that would need more, because it is
/// zoomed out very far, draws every repeat from its bitmap instead.
const MAX_PRETILED_PIXELS: i64 = 1 << 22;

/// The pre-tiled bitmap: `src` repeated over `width` by `height`.
fn draw_pretiled(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    src: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    let tile = bitmaps.rect(src)?;
    bitmaps.blt(dst, 0, 0, src, tile, 255)?;
    // What is there is doubled until it fills the first row of repeats, and that row until
    // it fills the bitmap. A transfer onto transparent pixels copies.
    let mut done = tile.width;
    while done < width {
        bitmaps.blt(dst, done, 0, dst, Rect::new(0, 0, done, tile.height), 255)?;
        done *= 2;
    }
    let mut done = tile.height;
    while done < height {
        bitmaps.blt(dst, 0, done, dst, Rect::new(0, 0, width, done), 255)?;
        done *= 2;
    }
    Ok(())
}

/// How a plane tiles one axis of its area.
#[derive(Debug, Clone, Copy)]
struct Tiling {
    /// The screen pixels one copy of the source is stretched into.
    tile: i64,
    /// How many copies of the source one quad draws: 1 from the source itself, more from a
    /// pre-tiled bitmap.
    copies: i64,
}

impl Tiling {
    /// The tiling of `pixels` screen pixels in a row by a source `size` texels long at
    /// `zoom`, or `None` when nothing is drawn.
    fn new(size: i32, zoom: f64, pixels: u32) -> Option<Tiling> {
        let tile = stretch_length(size, zoom)?;
        // Whether the stretch step is exact, so that copies of the source in a row stretch
        // as the source itself does.
        let exact =
            (u64::from(size.unsigned_abs()) << STRETCH_FRAC_BITS).is_multiple_of(u64::from(tile));
        let (tile, pixels) = (i64::from(tile), i64::from(pixels));
        let single = Tiling { tile, copies: 1 };
        let copies = if exact && single.repeats(pixels) > AXIS_REPEATS {
            // The fewest copies that leave at most `AXIS_REPEATS` repeats.
            pixels / ((AXIS_REPEATS - 2) * tile) + 1
        } else {
            1
        };
        Some(Tiling { tile, copies })
    }

    /// The screen pixels one quad covers.
    fn span(&self) -> i64 {
        self.tile * self.copies
    }

    /// An upper bound of the quads that `pixels` screen pixels in a row can show, wherever
    /// the grid lies.
    fn repeats(&self, pixels: i64) -> i64 {
        pixels / self.span() + 2
    }
}

impl Plane {
    /// How the plane tiles a `level` with a bitmap of `size`: across and down. `None` when
    /// it draws nothing: a zoom of 0 or less, or a tile of less than one pixel.
    fn tiling(&self, level: &Level, size: Rect) -> Option<(Tiling, Tiling)> {
        Some((
            Tiling::new(size.width, self.zoom_x, level.clip.width)?,
            Tiling::new(size.height, self.zoom_y, level.clip.height)?,
        ))
    }

    /// The size of the pre-tiled bitmap a plane showing a bitmap of `size` inside `level`
    /// draws from, or `None` when it draws from the bitmap itself or draws nothing. The
    /// pre-tiled bitmap only saves quads, so one that would be too large is not used.
    fn pretiled_size(&self, level: &Level, size: Rect) -> Option<(i32, i32)> {
        let (across, down) = self.tiling(level, size)?;
        if across.copies == 1 && down.copies == 1 {
            return None;
        }
        let clip = level.clip;
        let source = Tiling {
            copies: 1,
            ..across
        }
        .repeats(i64::from(clip.width))
            * Tiling { copies: 1, ..down }.repeats(i64::from(clip.height));
        if source <= MAX_REPEATS {
            return None;
        }
        let width = across.copies * i64::from(size.width);
        let height = down.copies * i64::from(size.height);
        // Both are at most their product.
        (width * height <= MAX_PRETILED_PIXELS).then_some((width as i32, height as i32))
    }

    /// Brings the pre-tiled bitmap up to date, if this frame draws from one. An invisible
    /// plane gives it back; one that is not drawn for another reason, such as having no
    /// `level` because its viewport is hidden, keeps what it has.
    pub(crate) fn refresh_cache(&mut self, bitmaps: &mut Bitmaps, level: Option<&Level>) {
        if !self.visible {
            self.pretiled.dispose(bitmaps);
            return;
        }
        let Some(level) = level.filter(|_| self.opacity > 0) else {
            return;
        };
        let wanted = self.bitmap.and_then(|source| {
            let size = bitmaps.rect(source).ok()?;
            let (width, height) = self.pretiled_size(level, size)?;
            Some((source, bitmaps.revision(source).ok()?, width, height, false))
        });
        self.pretiled.refresh(bitmaps, wanted, draw_pretiled);
    }
}

/// The first of the positions `start`, `start + span`, ... that the grid of a plane has at
/// or before `first`, when `shift` is what the origins move the grid by.
fn grid_start(first: i64, shift: i64, span: i64) -> i64 {
    // The mathematical modulo: an origin below zero moves the grid the other way.
    first - (first - shift).rem_euclid(span)
}

impl Assembly<'_> {
    /// A plane: one stretch quad for every repeat of its tile that shows inside the clip.
    ///
    /// Column `i` of the viewport, or of the screen without one, shows column
    /// `(i + plane.ox + viewport.ox) mod tile width` of the tile, and rows likewise.
    pub(crate) fn plane(&mut self, plane: &Plane, level: &Level) {
        if !plane.visible || plane.opacity == 0 {
            return;
        }
        let Some(source) = plane.bitmap else {
            return;
        };
        let Ok(size) = self.bitmaps.rect(source) else {
            return;
        };
        let Some((mut across, mut down)) = plane.tiling(level, size) else {
            return;
        };
        // Without a pre-tiled bitmap, also when one could not be made, every repeat is drawn
        // from the source.
        let pretiled = plane.pretiled_size(level, size);
        let bitmap = match pretiled.and(plane.pretiled.bitmap) {
            Some(bitmap) => bitmap,
            None => {
                (across.copies, down.copies) = (1, 1);
                source
            }
        };
        let Ok(tile) = self.bitmaps.rect(bitmap) else {
            return;
        };
        let clip = level.clip;
        let look = Look {
            tone: plane.tone.to_proto(),
            color: plane.color.to_proto(),
            opacity: plane.opacity,
            blend: blend_mode(plane.blend_type),
        };
        // The grid has a line where the viewport begins, moved back by both origins.
        let (left, top) = (i64::from(clip.x), i64::from(clip.y));
        let (right, bottom) = (left + i64::from(clip.width), top + i64::from(clip.height));
        let first_x = grid_start(left, level.dx - i64::from(plane.ox), across.span());
        let first_y = grid_start(top, level.dy - i64::from(plane.oy), down.span());
        // A span is one tile or little more than the clip, so it fits. A tile so large that
        // it begins beyond the coordinates of the protocol is left out.
        let (width, height) = (across.span() as u32, down.span() as u32);
        let mut y = first_y;
        while y < bottom {
            let mut x = first_x;
            while x < right {
                if let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) {
                    let dest = vkxp_proto::Rect::new(x, y, width, height);
                    self.stretched(bitmap, tile, dest, false, clip, look);
                }
                x += across.span();
            }
            y += down.span();
        }
    }
}

#[cfg(test)]
mod tests {
    use vkxp_compose_ref::Frame;
    use vkxp_pixel::Rgba;

    use super::*;
    use crate::bitmap::testing::color;
    use crate::extract::testing::*;
    use crate::{Color, PlaneId, Tone};

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

    fn quads(packet: &vkxp_proto::FramePacket) -> usize {
        let quad = |op: &&vkxp_proto::Op| matches!(op, vkxp_proto::Op::Quad(_));
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
        let samples = include_str!("../tests/data/plane-samples.txt");
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
        let packet = stage.scene.extract(&mut stage.bitmaps);
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
        let packet = stage.scene.extract(&mut stage.bitmaps);
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
                let packet = stage.scene.extract(&mut stage.bitmaps);
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
    fn a_plane_with_an_inexact_step_draws_every_repeat_from_its_bitmap() {
        let mut stage = Stage::sized(160, 120);
        let bitmap = pattern(&mut stage, 4, 4);
        let id = plane(&mut stage, bitmap);
        let plane = stage.scene.plane_mut(id).unwrap();
        (plane.zoom_x, plane.zoom_y) = (1.5, 1.5);
        let packet = stage.scene.extract(&mut stage.bitmaps);
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
        let packet = stage.scene.extract(&mut stage.bitmaps);
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
        let packet = stage.scene.extract(&mut stage.bitmaps);
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
        let packet = stage.scene.extract(&mut stage.bitmaps);
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
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(stage.bitmaps.len(), 1);
        assert_eq!(quads(&packet), 640 * 480);
        let covered = packet.ops.iter().all(|op| match op {
            vkxp_proto::Op::Quad(quad) => (quad.clip.width, quad.clip.height) == (1, 1),
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
}
