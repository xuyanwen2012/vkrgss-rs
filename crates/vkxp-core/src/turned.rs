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

use crate::extract::{Assembly, Level, Look};
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
    bush: i32,
    /// Left, top, right and bottom of the box, from the position of the sprite.
    bounds: [i64; 4],
    /// Left and top of the rendered part, from the position of the sprite, and its size.
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
    geometry: Geometry,
    bounds: TurnedBox,
    /// The pixels of the box that are drawn and lie inside the level. Never empty.
    part: vkxp_proto::Rect,
}

impl Sprite {
    /// Where the sprite, turned, is drawn inside `level`, or `None` when nothing of it is.
    ///
    /// PLACEHOLDER: a turned sprite inside a viewport is placed as if its position were moved
    /// by the viewport's; what the original runtime draws there is not measured.
    fn turned_placement(&self, level: &Level) -> Option<Placement> {
        let geometry = Geometry {
            // Sums of two `i32` and an `i64` offset built from two more.
            x: (i64::from(self.x) + level.dx) as f64,
            y: (i64::from(self.y) + level.dy) as f64,
            ox: f64::from(self.ox),
            oy: f64::from(self.oy),
            zoom_x: self.zoom_x,
            zoom_y: self.zoom_y,
            angle: self.angle,
            mirror: self.mirror,
        };
        let bounds = geometry.turned_box(self.src_rect.width, self.src_rect.height)?;
        // A box too large for a bitmap is not drawn.
        let cells = (bounds.right - bounds.left).checked_mul(bounds.bottom - bounds.top)?;
        if cells.unsigned_abs() > MAX_BITMAP_PIXELS {
            return None;
        }
        let clip = level.clip;
        let left = bounds.left.max(i64::from(clip.x));
        let top = bounds.top.max(i64::from(clip.y));
        let right = bounds.right.min(i64::from(clip.x) + i64::from(clip.width));
        let bottom = bounds
            .bottom
            .min(i64::from(clip.y) + i64::from(clip.height));
        // What is left lies inside the clip.
        (left < right && top < bottom).then(|| Placement {
            geometry,
            bounds,
            part: vkxp_proto::Rect::new(
                left as i32,
                top as i32,
                (right - left) as u32,
                (bottom - top) as u32,
            ),
        })
    }

    /// The number of rows at the bottom of the source rectangle that are faint.
    fn turned_bush(&self) -> i32 {
        self.bush_depth.clamp(0, self.src_rect.height.max(0))
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
            let placement = self.turned_placement(level)?;
            let (x, y) = (placement.geometry.x as i64, placement.geometry.y as i64);
            let (bounds, part) = (placement.bounds, placement.part);
            let key = Key {
                source,
                revision,
                src: self.src_rect,
                origin: (self.ox, self.oy),
                turn: [self.zoom_x, self.zoom_y, self.angle].map(f64::to_bits),
                mirror: self.mirror,
                bush: self.turned_bush(),
                bounds: [
                    bounds.left - x,
                    bounds.top - y,
                    bounds.right - x,
                    bounds.bottom - y,
                ],
                part: (
                    i64::from(part.x) - x,
                    i64::from(part.y) - y,
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
    ///
    /// PLACEHOLDER: the lowest rows of the source rectangle, as many as the bush depth, get
    /// half their alpha here. What the original runtime does with the bush depth of a turned
    /// sprite is not measured.
    fn render_turned(&self, bitmaps: &Bitmaps, placement: &Placement) -> Option<Vec<u8>> {
        let source = bitmaps.get(self.bitmap?).ok()?;
        let (part, bounds) = (placement.part, &placement.bounds);
        let (width, height) = (part.width as usize, part.height as usize);
        let mut rgba = vec![0; width * height * 4];
        let src = self.src_rect;
        let faint_from = i64::from(src.y) + i64::from(src.height) - i64::from(self.turned_bush());
        let first = i64::from(part.x) - bounds.left;
        let columns = first..first + i64::from(part.width);
        for (row, pixels) in rgba.chunks_exact_mut(width * 4).enumerate() {
            let j = i64::from(part.y) + row as i64;
            placement
                .geometry
                .turned_row(bounds, src, j, columns.clone(), |i, u, v| {
                    // The part of the source rectangle outside the bitmap shows nothing.
                    let (Ok(u), Ok(v_at)) = (usize::try_from(u), usize::try_from(v)) else {
                        return;
                    };
                    if u >= source.width || v_at >= source.height {
                        return;
                    }
                    let mut pixel = source.pixel(u, v_at);
                    if i64::from(v) >= faint_from {
                        pixel[3] >>= 1;
                    }
                    let at = (i - first) as usize * 4;
                    pixels[at..at + 4].copy_from_slice(&pixel);
                });
        }
        Some(rgba)
    }
}

impl Assembly<'_> {
    /// A turned sprite: the bitmap it was rendered into, unscaled, where the rendered part
    /// of its box lies.
    pub(crate) fn turned_sprite(&mut self, sprite: &Sprite, level: &Level, look: Look) {
        let Some(bitmap) = sprite.turned.bitmap else {
            return;
        };
        let Some(placement) = sprite.turned_placement(level) else {
            return;
        };
        let part = placement.part;
        let src = Rect::new(0, 0, part.width as i32, part.height as i32);
        self.stretched(bitmap, src, part, false, level.clip, look);
    }
}
