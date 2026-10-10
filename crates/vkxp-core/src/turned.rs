//! Turned sprites: a sprite with an angle other than 0 is rendered on the CPU, by the rule
//! measured on the original runtime, into a bitmap of the scene's own, and drawn from it as
//! an unscaled quad.
//!
//! The original runtime does the same: it walks the bounding box of the turned source
//! rectangle row by row in double precision ([`Geometry::turned_box`],
//! [`Geometry::turned_row`]), puts the texels into a temporary bitmap and blends that like a
//! sprite that is not turned. The bitmap here lives in the bitmap arena and is rendered again
//! only when what it was rendered from changes; a sprite that turns every frame is rendered
//! every frame.
//!
//! Only the part of the box inside the sprite's level is rendered, so the bitmap is never
//! larger than the screen. The pixels depend on the position of the sprite only through
//! where the box and that part lie against it, so a sprite that moves keeps its bitmap.

use vkxp_proto::BitmapId;

use crate::extract::{Assembly, Level, Look, rows};
use crate::geometry::{Geometry, TurnedBox};
use crate::{Bitmaps, FontDefaults, MAX_BITMAP_PIXELS, Rect, Sprite};

/// What the bitmap of a turned sprite was rendered from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Key {
    source: BitmapId,
    revision: u64,
    src: Rect,
    origin: (i32, i32),
    /// The bits of zoom x, zoom y and the angle.
    turn: [u64; 3],
    mirror: bool,
    /// The origin of the viewport, by which the picture is displaced inside the box.
    shift: (i64, i64),
    /// Left, top, right and bottom of the box, from where the box is computed for.
    bounds: [i64; 4],
    /// Left and top of the rendered part, from the same place, and its size.
    part: (i64, i64, u32, u32),
}

/// The bitmap a turned sprite is drawn from.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Turned {
    /// What `bitmap` was rendered from, or would have been had it been possible to create.
    key: Option<Key>,
    pub(crate) bitmap: Option<BitmapId>,
}

impl Turned {
    pub(crate) fn dispose(&mut self, bitmaps: &mut Bitmaps) {
        if let Some(id) = self.bitmap.take() {
            bitmaps.dispose(id);
        }
        self.key = None;
    }
}

/// Where a turned sprite is drawn inside a level.
struct Placement {
    /// The sprite at its own position, which the rows are walked from.
    geometry: Geometry,
    /// The box, counted from the corner of the viewport.
    bounds: TurnedBox,
    /// The corner of the viewport on the screen.
    corner: (i64, i64),
    /// The pixels of the box, on the screen, that are drawn and lie inside the level. Never
    /// empty.
    part: vkxp_proto::Rect,
}

impl Sprite {
    /// Where the sprite, turned, is drawn inside `level`, or `None` when nothing of it is.
    /// `bitmap` is the rectangle of its bitmap: a turned sprite whose source rectangle does
    /// not lie inside it is not drawn at all.
    ///
    /// Inside a viewport the original runtime computes the box from the position less the
    /// viewport's origin and puts it at the viewport's corner, but walks the rows from the
    /// position itself. The picture is therefore displaced inside its box by the origin,
    /// cut where it leaves the box, and gone when the origin is large.
    ///
    /// Provenance: oracle measurement 2026-10-10, RGSS103J, probes `turnedvp`, `geom2` and
    /// `srcrect` (48 turned sprites in viewports and 32 with source rectangles in and
    /// outside the bitmap, without a differing pixel).
    fn turned_placement(&self, level: &Level, bitmap: Rect) -> Option<Placement> {
        let src = self.src_rect;
        let inside = src.x >= 0
            && src.y >= 0
            && i64::from(src.x) + i64::from(src.width) <= i64::from(bitmap.width)
            && i64::from(src.y) + i64::from(src.height) <= i64::from(bitmap.height);
        if !inside {
            return None;
        }
        let geometry = Geometry {
            x: f64::from(self.x),
            y: f64::from(self.y),
            ox: f64::from(self.ox),
            oy: f64::from(self.oy),
            zoom_x: self.zoom_x,
            zoom_y: self.zoom_y,
            angle: self.angle,
            mirror: self.mirror,
        };
        let shifted = Geometry {
            // Differences of an `i32` and an `i64` built from one.
            x: (i64::from(self.x) - level.ox) as f64,
            y: (i64::from(self.y) - level.oy) as f64,
            ..geometry
        };
        let bounds = shifted.turned_box(src.width, src.height)?;
        // A box too large for a bitmap is not drawn.
        let cells = (bounds.right - bounds.left).checked_mul(bounds.bottom - bounds.top)?;
        if cells.unsigned_abs() > MAX_BITMAP_PIXELS {
            return None;
        }
        let corner = (level.dx + level.ox, level.dy + level.oy);
        let clip = level.clip;
        let left = (bounds.left + corner.0).max(i64::from(clip.x));
        let top = (bounds.top + corner.1).max(i64::from(clip.y));
        let right = (bounds.right + corner.0).min(i64::from(clip.x) + i64::from(clip.width));
        let bottom = (bounds.bottom + corner.1).min(i64::from(clip.y) + i64::from(clip.height));
        // What is left lies inside the clip.
        (left < right && top < bottom).then(|| Placement {
            geometry,
            bounds,
            corner,
            part: vkxp_proto::Rect::new(
                left as i32,
                top as i32,
                (right - left) as u32,
                (bottom - top) as u32,
            ),
        })
    }

    /// Brings the bitmap of a turned sprite up to date, if this frame draws from one. A
    /// sprite that is not turned, is invisible or has nothing inside its level gives it
    /// back; one whose viewport is hidden, so that it has no `level`, keeps what it has.
    pub(crate) fn refresh_turned(&mut self, bitmaps: &mut Bitmaps, level: Option<&Level>) {
        if !self.visible || self.angle == 0.0 {
            self.turned.dispose(bitmaps);
            return;
        }
        let Some(level) = level else {
            return;
        };
        let wanted = self.bitmap.and_then(|source| {
            let revision = bitmaps.revision(source).ok()?;
            let placement = self.turned_placement(level, bitmaps.rect(source).ok()?)?;
            let (bounds, part) = (placement.bounds, placement.part);
            // Where the box is computed for, from the corner of the viewport and on the
            // screen.
            let (x, y) = (i64::from(self.x) - level.ox, i64::from(self.y) - level.oy);
            let (screen_x, screen_y) = (x + placement.corner.0, y + placement.corner.1);
            let key = Key {
                source,
                revision,
                src: self.src_rect,
                origin: (self.ox, self.oy),
                turn: [self.zoom_x, self.zoom_y, self.angle].map(f64::to_bits),
                mirror: self.mirror,
                shift: (level.ox, level.oy),
                bounds: [
                    bounds.left - x,
                    bounds.top - y,
                    bounds.right - x,
                    bounds.bottom - y,
                ],
                part: (
                    i64::from(part.x) - screen_x,
                    i64::from(part.y) - screen_y,
                    part.width,
                    part.height,
                ),
            };
            Some((key, placement))
        });
        let Some((key, placement)) = wanted else {
            self.turned.dispose(bitmaps);
            return;
        };
        let lost = self.turned.bitmap.is_some_and(|id| bitmaps.is_disposed(id));
        if self.turned.key == Some(key) && !lost {
            return;
        }
        let rgba = self.render_turned(bitmaps, &placement);
        let size = Rect::new(
            0,
            0,
            placement.part.width as i32,
            placement.part.height as i32,
        );
        let reusable = self
            .turned
            .bitmap
            .filter(|&id| bitmaps.rect(id) == Ok(size));
        let target = match (reusable, rgba) {
            (Some(id), Some(rgba)) => bitmaps.get_mut(id).ok().map(|bitmap| {
                bitmap.rgba = rgba;
                bitmap.mark(bitmap.bounds());
                id
            }),
            (_, rgba) => {
                self.turned.dispose(bitmaps);
                let (width, height) = (placement.part.width, placement.part.height);
                let defaults = FontDefaults::new(Vec::new());
                rgba.and_then(|rgba| {
                    bitmaps
                        .create_from_rgba(width, height, rgba, &defaults)
                        .ok()
                })
            }
        };
        self.turned.key = Some(key);
        self.turned.bitmap = target;
    }

    /// The pixels of the part of the box that `placement` names: the texel every pixel
    /// shows, and transparent black where it shows none. `None` when the source is disposed.
    fn render_turned(&self, bitmaps: &Bitmaps, placement: &Placement) -> Option<Vec<u8>> {
        let source = bitmaps.get(self.bitmap?).ok()?;
        let (part, bounds) = (placement.part, &placement.bounds);
        let (width, height) = (part.width as usize, part.height as usize);
        let mut rgba = vec![0; width * height * 4];
        let src = self.src_rect;
        let first = i64::from(part.x) - placement.corner.0 - bounds.left;
        let columns = first..first + i64::from(part.width);
        for (row, pixels) in rgba.chunks_exact_mut(width * 4).enumerate() {
            let j = i64::from(part.y) - placement.corner.1 + row as i64;
            placement
                .geometry
                .turned_row(bounds, src, j, columns.clone(), |i, u, v| {
                    // Inside the source rectangle, which lies inside the bitmap.
                    let (Ok(u), Ok(v)) = (usize::try_from(u), usize::try_from(v)) else {
                        return;
                    };
                    let at = (i - first) as usize * 4;
                    pixels[at..at + 4].copy_from_slice(&source.pixel(u, v));
                });
        }
        Some(rgba)
    }
}

impl Assembly<'_> {
    /// A turned sprite: the bitmap it was rendered into, unscaled, where the rendered part
    /// of its box lies. The lowest rows of the whole box, as many as the bush depth
    /// whatever texels they show, are drawn with half the opacity.
    ///
    /// Provenance: oracle measurement 2026-10-10, RGSS103J, probes `bush` and `geom2` (51
    /// turned sprites with a bush depth, without a differing pixel; opaque texels at
    /// opacity 255 only).
    pub(crate) fn turned_sprite(&mut self, sprite: &Sprite, level: &Level, look: Look) {
        let Some(bitmap) = sprite.turned.bitmap else {
            return;
        };
        let Some(source) = sprite.bitmap.and_then(|id| self.bitmaps.rect(id).ok()) else {
            return;
        };
        let Some(placement) = sprite.turned_placement(level, source) else {
            return;
        };
        let bottom = placement.bounds.bottom + placement.corner.1;
        let split = bottom - i64::from(sprite.bush_depth.max(0));
        self.picture(bitmap, placement.part, split, level.clip, look);
    }

    /// The whole of `bitmap`, unscaled, at `part`: the rows above `split` as `look` says and
    /// those from it on with half the opacity.
    fn picture(
        &mut self,
        bitmap: BitmapId,
        part: vkxp_proto::Rect,
        split: i64,
        clip: vkxp_proto::Rect,
        look: Look,
    ) {
        let src = Rect::new(0, 0, part.width as i32, part.height as i32);
        let faint = Look {
            opacity: look.opacity >> 1,
            ..look
        };
        let above = rows(clip, i64::MIN, split);
        let below = rows(clip, split, i64::MAX);
        self.stretched(bitmap, src, part, false, above, look);
        self.stretched(bitmap, src, part, false, below, faint);
    }
}
