//! Planes: a bitmap repeated without end over an area, drawn as one quad per repeat.
//!
//! Every repeat is a quad over the whole bitmap. The repeats share the linear part of the
//! transform and differ in a translation that is a whole multiple of the bitmap's width or
//! height, so each screen pixel lies in the source rectangle of exactly one of them: repeats
//! meet without gaps or overlaps at any zoom.
//!
//! A bitmap that is small against the area would need many quads. The plane then draws from a
//! bitmap of the scene's own that repeats the source a few times, kept in the bitmap arena and
//! drawn again only when the source changes.

use vkxp_proto::{BitmapId, Fixed, Op, Quad, Transform};

use crate::extract::{Assembly, Level, Look, blend_mode};
use crate::geometry::Geometry;
use crate::{BitmapError, Bitmaps, Plane, Rect};

/// Most quads one plane is drawn with.
const MAX_REPEATS: i64 = 64;
/// Most repeats along one axis once the plane draws from a pre-tiled bitmap.
const AXIS_REPEATS: i64 = 8;

/// Most pixels a pre-tiled bitmap may have. A plane that would need more, because it is
/// zoomed out very far, is not drawn.
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

/// An upper bound of the number of repeats of a tile `size` texels long that `pixels` screen
/// pixels in a row can show, when one screen pixel advances by `step` (in units of 2^-16
/// texels).
fn repeats(pixels: u32, step: i64, size: i64) -> i64 {
    span(pixels, step) / size + 2
}

/// An upper bound of the distance in texels between the first and the last of `pixels` screen
/// pixels. The rounding of the transform and of the texel add at most one each.
fn span(pixels: u32, step: i64) -> i64 {
    ((step * (i64::from(pixels) - 1)) >> Fixed::FRAC_BITS) + 2
}

/// How many times the source is repeated along one axis of the pre-tiled bitmap so that at
/// most [`AXIS_REPEATS`] of the result are shown.
fn copies(pixels: u32, step: i64, size: i64) -> i64 {
    if repeats(pixels, step, size) <= AXIS_REPEATS {
        1
    } else {
        span(pixels, step) / ((AXIS_REPEATS - 1) * size) + 1
    }
}

impl Plane {
    /// The placement of the repeat whose top-left corner is the one bitmap pixel (0, 0) of the
    /// plane has, for a tile of `width` by `height` pixels.
    ///
    /// PLACEHOLDER: how the original runtime samples a zoomed plane, what a zoom of 0 or less
    /// draws (here: nothing, and the flipped image), and whether the origin of the viewport
    /// scrolls a plane (here: it does) are not yet checked.
    fn geometry(&self, level: &Level, width: i32, height: i32) -> Geometry {
        Geometry {
            // Sums of two `i32`.
            x: level.dx as f64,
            y: level.dy as f64,
            // The image repeats, so the origin counts modulo the tile; this keeps the
            // translation small whatever the origin is.
            ox: f64::from(self.ox.rem_euclid(width)),
            oy: f64::from(self.oy.rem_euclid(height)),
            zoom_x: self.zoom_x,
            zoom_y: self.zoom_y,
            angle: 0.0,
            mirror: false,
        }
    }

    /// The size of the pre-tiled bitmap a plane showing a bitmap of `size` inside `clip`
    /// draws from, or `None` when it draws from the bitmap itself or draws nothing.
    fn pretiled_size(&self, level: &Level, size: Rect) -> Option<(i32, i32)> {
        let transform = self
            .geometry(level, size.width, size.height)
            .transform(0.0, 0.0, 0.0)?;
        let step_x = i64::from(transform.xx.to_bits()).abs();
        let step_y = i64::from(transform.yy.to_bits()).abs();
        let (width, height) = (i64::from(size.width), i64::from(size.height));
        let clip = level.clip;
        if repeats(clip.width, step_x, width) * repeats(clip.height, step_y, height) <= MAX_REPEATS
        {
            return None;
        }
        let pretiled_width = copies(clip.width, step_x, width) * width;
        let pretiled_height = copies(clip.height, step_y, height) * height;
        if pretiled_width * pretiled_height > MAX_PRETILED_PIXELS {
            // A size no bitmap can have: the plane goes without and is not drawn.
            return Some((i32::MAX, i32::MAX));
        }
        // Both are at most their product.
        Some((pretiled_width as i32, pretiled_height as i32))
    }

    /// Brings the pre-tiled bitmap up to date, if this frame draws from one. An invisible
    /// plane gives it back.
    pub(crate) fn refresh_cache(&mut self, bitmaps: &mut Bitmaps, level: &Level) {
        if !self.visible {
            self.pretiled.dispose(bitmaps);
            return;
        }
        if self.opacity == 0 {
            return;
        }
        let wanted = self.bitmap.and_then(|source| {
            let size = bitmaps.rect(source).ok()?;
            let (width, height) = self.pretiled_size(level, size)?;
            Some((source, bitmaps.revision(source).ok()?, width, height, false))
        });
        self.pretiled.refresh(bitmaps, wanted, draw_pretiled);
    }
}

impl Assembly<'_> {
    /// A plane: one quad for every repeat of its bitmap that shows inside the clip.
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
        let bitmap = match plane.pretiled_size(level, size) {
            None => source,
            // A pre-tiled bitmap that could not be made leaves the plane undrawn.
            Some(_) => match plane.pretiled.bitmap {
                Some(bitmap) => bitmap,
                None => return,
            },
        };
        let Ok(tile) = self.bitmaps.rect(bitmap) else {
            return;
        };
        let geometry = plane.geometry(level, tile.width, tile.height);
        let Some(base) = geometry.transform(f64::from(tile.width), 0.0, 0.0) else {
            return;
        };

        // The transform is monotonic along each axis, so the repeats that show are those
        // between the ones under the first and the last pixel of the clip, which lies on the
        // screen and so has coordinates that fit `u16`.
        let clip = level.clip;
        let (width, height) = (i64::from(tile.width), i64::from(tile.height));
        let first = base.source_position(clip.x as u16, clip.y as u16);
        let last = base.source_position(
            (clip.x as u32 + clip.width - 1) as u16,
            (clip.y as u32 + clip.height - 1) as u16,
        );
        let repeat = |position: i64, size: i64| (position >> Fixed::FRAC_BITS).div_euclid(size);
        let columns = (repeat(first.0, width), repeat(last.0, width));
        let rows = (repeat(first.1, height), repeat(last.1, height));
        let look = Look {
            tone: plane.tone.to_proto(),
            color: plane.color.to_proto(),
            opacity: plane.opacity,
            blend: blend_mode(plane.blend_type),
        };
        // A repeat too far from the base for the fixed-point range is left out.
        let shifted = |translation: Fixed, repeat: i64, size: i64| {
            let bits = i64::from(translation.to_bits()) - ((repeat * size) << Fixed::FRAC_BITS);
            i32::try_from(bits).ok().map(Fixed::from_bits)
        };
        for row in rows.0.min(rows.1)..=rows.0.max(rows.1) {
            let Some(ty) = shifted(base.ty, row, height) else {
                continue;
            };
            for column in columns.0.min(columns.1)..=columns.0.max(columns.1) {
                let Some(tx) = shifted(base.tx, column, width) else {
                    continue;
                };
                self.referenced.push(bitmap);
                self.ops.push(Op::Quad(Quad {
                    clip,
                    bitmap,
                    src: tile.to_proto(),
                    transform: Transform { tx, ty, ..base },
                    tone: look.tone,
                    color: look.color,
                    opacity: look.opacity,
                    blend: look.blend,
                }));
            }
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
        let quad = |op: &&Op| matches!(op, Op::Quad(_));
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

        // So do origin and zoom, which need no new copy.
        let plane = stage.scene.plane_mut(id).unwrap();
        (plane.ox, plane.oy) = (-3, 9);
        (plane.zoom_x, plane.zoom_y) = (2.0, 1.0);
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert!(quads(&packet) <= 64);
        let frame = stage.render(&packet).unwrap();
        assert!(shows(&frame, |x, y| expected((x / 2 + 1) % 4, (y + 9) % 4)));
    }

    #[test]
    fn a_plane_never_needs_more_than_64_quads() {
        let zooms = [1.0, 0.5, 0.11, 3.0, 1.0 / 3.0, -1.0, 40.0];
        for (width, height) in [(1, 1), (4, 4), (3, 200), (64, 64), (100, 7), (640, 480)] {
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
                let drawn = frame.rgba.as_chunks::<4>().0;
                assert!(
                    drawn.iter().all(|pixel| *pixel != BLACK),
                    "{width} by {height} at {zoom} covers the screen"
                );
            }
        }
    }

    #[test]
    fn a_plane_zoomed_out_too_far_for_a_pretiled_copy_is_not_drawn() {
        let mut stage = Stage::new();
        let bitmap = pattern(&mut stage, 4, 4);
        let id = plane(&mut stage, bitmap);
        let plane = stage.scene.plane_mut(id).unwrap();
        (plane.zoom_x, plane.zoom_y) = (0.01, 0.01);
        assert!(all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), 1);
        // Zoomed out less far, each screen pixel takes every fifth texel.
        let plane = stage.scene.plane_mut(id).unwrap();
        (plane.zoom_x, plane.zoom_y) = (0.2, 0.2);
        let frame = stage.frame();
        assert!(shows(&frame, |x, y| texel(
            (x * 5 + 2) % 4,
            (y * 5 + 2) % 4
        )));
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
