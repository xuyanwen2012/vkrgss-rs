//! The sprite of RGSS2 that is not turned, which stands beside the sprite of RGSS1
//! ([`Assembly::upright_sprite`](super::Assembly)) and shares none of its arithmetic of
//! place and size.
//!
//! Measured on the original runtime of VX, on both versions of its DLL, the sprite is
//! placed and sized in double precision ([`geometry::stretch_span_rgss2`]), and a mirrored
//! sprite that is not stretched and whose source rectangle leaves its bitmap is cut first
//! and mirrored inside what is left. The blit, the stretch walk and its cut rule are those
//! of RGSS1.
//!
//! A sprite becomes stretch quads where they draw exactly the measured texels, and is
//! otherwise rendered into a bitmap of the scene's own, like the cut sprite of RGSS1
//! ([`crate::turned`]).
//!
//! Provenance: oracle measurement 2026-10-12, RGSS202E 2.0.2.1 and 2.0.0.1 (the report
//! `vx-v1-sprite-REPORT.md`, sections 1 and 2: 325 frames and a held-out set of random
//! scenes without a differing pixel). The cases of `tests/data/sprite-rgss2-cases.txt` are
//! a selection of what was measured.

use vkxp_proto::BitmapId;

use super::{Assembly, Bush, Level, Look, Rows, intersect};
use crate::geometry;
use crate::turned::Span;
use crate::{Bitmaps, Rect, Sprite};

/// What the texels of a stretch that lie outside the bitmap do to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outside {
    /// They are left out, and the others stay where they are.
    LeftOut,
    /// They cut the stretch as they cut a `stretch_blt` ([`Span::texel`]).
    Cut,
}

/// One stretch along one axis of a sprite: `length` texels from `start` on, drawn into
/// `pixels` pixels from pixel `first` of the sprite on, which may lie before the sprite.
/// Neither length is 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Run {
    pub(crate) start: i64,
    pub(crate) length: u32,
    pub(crate) first: i64,
    pub(crate) pixels: u32,
    /// Counts the texels from the far end.
    pub(crate) mirror: bool,
    pub(crate) outside: Outside,
}

impl Run {
    /// The texel that pixel `i` of the sprite shows along this axis, in a bitmap of `size`
    /// texels along it, or `None` when it shows none.
    fn texel(&self, i: i64, size: usize) -> Option<usize> {
        let span = Span {
            start: self.start,
            length: self.length,
            first: 0,
            pixels: self.pixels,
            mirror: self.mirror,
            cut: self.outside == Outside::Cut,
        };
        span.texel(u32::try_from(i - self.first).ok()?, size)
    }

    /// Whether a stretch quad of the texels inside a bitmap of `size` texels along this
    /// axis shows what [`Run::texel`] gives: the texels all lie inside, or they are drawn
    /// one to a pixel and those outside are only left out.
    fn is_quad(&self, size: i32) -> bool {
        let inside = self.start >= 0 && self.start + i64::from(self.length) <= i64::from(size);
        inside || (self.pixels == self.length && self.outside == Outside::LeftOut)
    }
}

/// What an unrotated sprite of RGSS2 shows, apart from where it stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Shape {
    /// The pixels the sprite covers.
    pub(crate) size: (u32, u32),
    pub(crate) across: Run,
    /// The rows above the faint ones, or all rows when none is faint, and the faint rows.
    pub(crate) down: [Option<Run>; 2],
    /// The number of faint rows.
    pub(crate) bush: u32,
    /// Whether the columns are cut first and mirrored inside what is left: the sprite is
    /// mirrored and not stretched.
    pub(crate) in_place: bool,
}

impl Shape {
    /// Whether stretch quads draw the sprite, for `bitmap`, the rectangle of its bitmap.
    pub(crate) fn is_quads(&self, bitmap: Rect) -> bool {
        self.across.is_quad(bitmap.width)
            && self
                .down
                .iter()
                .flatten()
                .all(|run| run.is_quad(bitmap.height))
    }
}

/// An unrotated sprite of RGSS2 in the coordinates of its viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Upright {
    pub(crate) shape: Shape,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl Upright {
    /// The pixels the sprite covers inside `level`, before the level clips it, or `None`
    /// when it begins beyond the coordinates of the protocol.
    fn dest(&self, level: &Level) -> Option<vkxp_proto::Rect> {
        let x = i32::try_from(i64::from(self.x) + level.dx).ok()?;
        let y = i32::try_from(i64::from(self.y) + level.dy).ok()?;
        let (width, height) = self.shape.size;
        Some(vkxp_proto::Rect::new(x, y, width, height))
    }
}

/// A sprite of RGSS2 that is drawn from a picture of its own, inside a level.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Placed {
    pub(crate) shape: Shape,
    /// The pixels the whole sprite covers, on the screen.
    pub(crate) dest: vkxp_proto::Rect,
    /// The pixels of `dest` that lie inside the level. Never empty.
    pub(crate) part: vkxp_proto::Rect,
}

impl Placed {
    /// Left and top of the part, from the corner of the sprite, and its size.
    pub(crate) fn offsets(&self) -> (u32, u32, u32, u32) {
        let (part, dest) = (self.part, self.dest);
        // The part lies inside the sprite.
        (
            part.x.abs_diff(dest.x),
            part.y.abs_diff(dest.y),
            part.width,
            part.height,
        )
    }
}

impl Sprite {
    /// The rows of the sprite, drawn `height` pixels high, as its bush depth splits them,
    /// and the number of faint rows.
    fn rows_rgss2(&self, height: u32, outside: Outside) -> ([Option<Run>; 2], u32) {
        let bush = Bush::new(self.src_rect, height, self.bush_depth);
        let outside = if bush.faint == 0 {
            outside
        } else {
            Outside::LeftOut
        };
        let run = |rows: Rows| Run {
            start: rows.start,
            length: rows.length,
            first: i64::from(rows.first),
            pixels: rows.pixels,
            mirror: false,
            outside,
        };
        ([bush.upper.map(run), bush.lower.map(run)], bush.faint)
    }

    /// The sprite, not turned, by the rules of RGSS2, or `None` when it draws nothing: a
    /// zoom or a side of the source rectangle of 0 or less, or no pixel.
    ///
    /// A sprite that is stretched on either axis is cut on both as a `stretch_blt` is,
    /// mirrored first. One that is not stretched only loses the texels outside its bitmap,
    /// and when it is mirrored the texels that are left are mirrored where they stand:
    /// with `[a, b)` the columns inside the bitmap, column `i` of them shows texel
    /// `src.x + (a + b - 1 - i)`.
    pub(crate) fn upright_rgss2(&self) -> Option<Upright> {
        let src = self.src_rect;
        let (x, width) = geometry::stretch_span_rgss2(self.x, self.ox, src.width, self.zoom_x)?;
        let (y, height) = geometry::stretch_span_rgss2(self.y, self.oy, src.height, self.zoom_y)?;
        // Positive, as the sprite has a length on the screen.
        let (w, h) = (src.width.unsigned_abs(), src.height.unsigned_abs());
        let stretched = width != w || height != h;
        let outside = if stretched {
            Outside::Cut
        } else {
            Outside::LeftOut
        };
        let across = Run {
            start: i64::from(src.x),
            length: w,
            first: 0,
            pixels: width,
            mirror: self.mirror,
            outside,
        };
        let (down, bush) = self.rows_rgss2(height, outside);
        Some(Upright {
            shape: Shape {
                size: (width, height),
                across,
                down,
                bush,
                in_place: self.mirror && !stretched,
            },
            x,
            y,
        })
    }

    /// Whether the sprite of RGSS2 is drawn from a picture although it is not turned:
    /// stretch quads do not draw what it shows of `bitmap`, the rectangle of its bitmap.
    pub(crate) fn is_pictured_rgss2(&self, bitmap: Rect) -> bool {
        self.angle == 0.0
            && self
                .upright_rgss2()
                .is_some_and(|upright| !upright.shape.is_quads(bitmap))
    }

    /// Where the sprite, drawn from a picture as [`Sprite::is_pictured_rgss2`] says, is
    /// drawn inside `level`, or `None` when nothing of it is.
    pub(crate) fn placed_rgss2(&self, level: &Level, bitmap: Rect) -> Option<Placed> {
        if !self.is_pictured_rgss2(bitmap) {
            return None;
        }
        let upright = self.upright_rgss2()?;
        let dest = upright.dest(level)?;
        let part = intersect(dest, level.clip);
        (!part.is_empty()).then_some(Placed {
            shape: upright.shape,
            dest,
            part,
        })
    }

    /// The pixels of the part of the sprite that `placed` names: the texel every pixel
    /// shows, and transparent black where it shows none. `None` when the source is disposed.
    pub(crate) fn render_rgss2(&self, bitmaps: &Bitmaps, placed: &Placed) -> Option<Vec<u8>> {
        let source = bitmaps.get(self.bitmap?).ok()?;
        let shape = &placed.shape;
        let (left, top, width, height) = placed.offsets();
        let (width, height) = (width as usize, height as usize);
        let mut rgba = vec![0; width * height * 4];
        let columns: Vec<Option<usize>> = (i64::from(left)..)
            .take(width)
            .map(|i| shape.across.texel(i, source.width))
            .collect();
        for (j, pixels) in (i64::from(top)..).zip(rgba.chunks_exact_mut(width * 4)) {
            let runs = shape.down.iter().flatten();
            let Some(v) = runs.filter_map(|run| run.texel(j, source.height)).next() else {
                continue;
            };
            for (pixel, u) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(&columns) {
                if let Some(u) = *u {
                    *pixel = source.pixel(u, v);
                }
            }
        }
        Some(rgba)
    }
}

impl Assembly<'_> {
    /// A sprite of RGSS2 that is not turned: stretch quads of its rows, or the picture it
    /// was rendered into where quads do not draw it.
    pub(crate) fn upright_sprite_rgss2(
        &mut self,
        sprite: &Sprite,
        bitmap: BitmapId,
        level: &Level,
        look: Look,
    ) {
        let Ok(bounds) = self.bitmaps.rect(bitmap) else {
            return;
        };
        let faint = Look {
            opacity: look.opacity >> 1,
            ..look
        };
        if sprite.is_pictured_rgss2(bounds) {
            let (Some(picture), Some(placed)) =
                (sprite.turned.bitmap, sprite.placed_rgss2(level, bounds))
            else {
                return;
            };
            let bottom = i64::from(placed.dest.y) + i64::from(placed.dest.height);
            let split = bottom - i64::from(placed.shape.bush);
            self.picture(picture, placed.part, split, level.clip, look, faint);
            return;
        }
        let Some(upright) = sprite.upright_rgss2() else {
            return;
        };
        let Some(dest) = upright.dest(level) else {
            return;
        };
        let (shape, src) = (upright.shape, sprite.src_rect);
        // The columns that are drawn, with their texels: all, or those inside the bitmap.
        let (mut left, mut columns, mut x, mut width) = (src.x, src.width, dest.x, dest.width);
        if shape.in_place {
            let a = src.x.saturating_neg().max(0);
            let b = src.width.min(bounds.width.saturating_sub(src.x));
            let Some(across) = b.checked_sub(a).filter(|across| *across > 0) else {
                return;
            };
            // Inside the bitmap and inside the sprite.
            (left, columns, x, width) = (src.x + a, across, dest.x + a, across.unsigned_abs());
        }
        // The rows before the sprite's first are not drawn.
        let clip = intersect(level.clip, dest);
        for (run, look) in [(shape.down[0], look), (shape.down[1], faint)] {
            let Some(run) = run else {
                continue;
            };
            // Rows that begin beyond the coordinates of a rectangle are not drawn.
            let (Ok(top), Ok(y), Ok(rows)) = (
                i32::try_from(run.start),
                i32::try_from(i64::from(dest.y) + run.first),
                i32::try_from(run.length),
            ) else {
                continue;
            };
            let part = Rect::new(left, top, columns, rows);
            let into = vkxp_proto::Rect::new(x, y, width, run.pixels);
            self.stretched(bitmap, part, into, shape.across.mirror, clip, look);
        }
    }
}

#[cfg(test)]
mod tests;
