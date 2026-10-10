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
mod tests;
