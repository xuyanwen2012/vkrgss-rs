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
//! The bush rows are drawn at the opacity `opacity * bush_opacity / 255`, without the
//! sprite's tone, colour and flash colour and, on a sprite that is not turned, without its
//! mirror ([`Sprite::bush_look_rgss2`]); a bush depth beyond the height stretches rows from
//! above the source rectangle ([`Sprite::rows_rgss2`]).
//!
//! A sprite with a positive wave amplitude that is not turned is drawn row by row, every
//! row moved horizontally ([`wave_offset`], [`Assembly::waving_sprite_rgss2`]); a negative
//! amplitude squeezes it instead ([`Sprite::upright_rgss2`]). At a wave length of 0 the
//! amplitude counts as 0 ([`Sprite::drawn_wave_amp`]).
//!
//! Not measured, and drawn as said where each is handled: a wave on a turned sprite (drawn
//! without the wave), a wave on a sprite stretched in x whose source rectangle leaves the
//! bitmap, a negative wave length, and a wave length of 0.
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

use super::{Assembly, Level, Look, intersect};
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
    /// The columns of the rows that are not faint. The faint rows show the same columns
    /// without the mirror.
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
    /// The columns of the faint rows.
    fn faint_across(&self) -> Run {
        Run {
            mirror: false,
            ..self.across
        }
    }

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
    /// The look of the bush rows of a sprite of RGSS2 that is drawn with `look`, or `None`
    /// when they are not drawn:
    ///
    /// ```text
    /// op = opacity * bush_opacity / 255     truncated; bush_opacity as it was written
    /// op < 0 or op > 255: the rows are not drawn
    /// ```
    ///
    /// with the sprite's blend type and without its tone, its colour and a flash colour.
    /// An opacity of 255 at a bush opacity of 128 gives 128, 161 at 300 gives 189, and 255
    /// at 256 draws nothing.
    ///
    /// Provenance: oracle measurement 2026-10-12, RGSS202E, topics `bushop` and `edge`
    /// (227 combinations of opacity and bush opacity, blend types, tone, colour and flash,
    /// on sprites turned and not, without a differing pixel).
    pub(crate) fn bush_look_rgss2(&self, look: Look) -> Option<Look> {
        let opacity = i64::from(self.opacity) * i64::from(self.bush_opacity) / 255;
        let opacity = u8::try_from(opacity).ok()?;
        Some(Look {
            blend: look.blend,
            ..Look::plain(opacity)
        })
    }

    /// The rows of the sprite, drawn `height` pixels high, as its bush depth splits them,
    /// and the number of faint rows. With `h` the rows of the source rectangle and `dh`
    /// the height:
    ///
    /// ```text
    /// bd = min(depth, dh)      the faint rows; none for a depth of 0 or less
    /// sb = depth * h / dh      the source rows they are taken from, not limited
    /// rows [0, dh - bd): the source rows [0, h - sb) stretched into dh - bd rows
    /// rows [dh - bd, dh): the source rows [h - sb, h) stretched into depth rows that end
    ///                     at the sprite's bottom; those above its top are not drawn
    /// ```
    ///
    /// each cut by the bitmap as `outside` says. Up to a depth of `dh` this is the split
    /// of RGSS1 ([`Bush::new`](super::Bush::new)); beyond it the faint rows begin above the
    /// source rectangle: 16 rows at a zoom of 2.25 with a depth of 41 show the rows
    /// `0 0 1 1 2 2 3 3 3 4 ...`. With `sb` of 0 the faint rows are not drawn.
    ///
    /// Provenance: oracle measurement 2026-10-12, RGSS202E, topic `bushbig` (108 depths
    /// near and beyond the height, with and without a source rectangle, without a
    /// differing pixel), and the held-out scenes.
    fn rows_rgss2(&self, height: u32, outside: Outside) -> ([Option<Run>; 2], u32) {
        let src = self.src_rect;
        let (top, h, dh) = (i64::from(src.y), i64::from(src.height), i64::from(height));
        let depth = i64::from(self.bush_depth);
        let run = |start: i64, length: i64, first: i64, pixels: i64| {
            let (length, pixels) = (u32::try_from(length).ok()?, u32::try_from(pixels).ok()?);
            (length > 0 && pixels > 0).then_some(Run {
                start,
                length,
                first,
                pixels,
                mirror: false,
                outside,
            })
        };
        let faint = depth.clamp(0, dh);
        if faint == 0 {
            return ([run(top, h, 0, dh), None], 0);
        }
        let shown = depth * h / dh;
        let upper = run(top, h - shown, 0, dh - faint);
        let lower = run(top + h - shown, shown, dh - depth, depth);
        // At most the height.
        ([upper, lower], faint as u32)
    }

    /// The wave amplitude the sprite is drawn with: its `wave_amp`, and 0 at a wave length
    /// of 0, with which the sprite is drawn as one without a wave, neither waved nor
    /// squeezed. The original runtime does not return from the update of such a sprite, so
    /// nothing was measured there.
    pub(crate) fn drawn_wave_amp(&self) -> i32 {
        if self.wave_length == 0 {
            0
        } else {
            self.wave_amp
        }
    }

    /// The sprite, not turned, by the rules of RGSS2, or `None` when it draws nothing: a
    /// zoom or a side of the source rectangle of 0 or less, or no pixel.
    ///
    /// A sprite that is stretched on either axis is cut on both as a `stretch_blt` is,
    /// mirrored first. One that is not stretched only loses the texels outside its bitmap,
    /// and when it is mirrored the texels that are left are mirrored where they stand:
    /// with `[a, b)` the columns inside the bitmap, column `i` of them shows texel
    /// `src.x + (a + b - 1 - i)`.
    ///
    /// A negative wave amplitude makes no wave: the sprite is squeezed into
    /// `width + 2 * wave_amp` columns that begin `-wave_amp` pixels further right, nothing
    /// when that is no column, and its columns only lose the texels outside the bitmap,
    /// mirrored first (topic `wave6` of the measurement).
    ///
    /// A positive amplitude is not drawn from this ([`Assembly::waving_sprite_rgss2`]).
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
        let amp = self.drawn_wave_amp();
        let squeezed = amp < 0;
        let (x, width) = if squeezed {
            let amp = i64::from(amp);
            let width = u32::try_from(i64::from(width) + 2 * amp).ok()?;
            (i32::try_from(i64::from(x) - amp).ok()?, width)
        } else {
            (x, width)
        };
        if width == 0 {
            return None;
        }
        let across = Run {
            start: i64::from(src.x),
            length: w,
            first: 0,
            pixels: width,
            mirror: self.mirror,
            outside: if squeezed { Outside::LeftOut } else { outside },
        };
        let (down, bush) = self.rows_rgss2(height, outside);
        Some(Upright {
            shape: Shape {
                size: (width, height),
                across,
                down,
                bush,
                in_place: self.mirror && !stretched && !squeezed,
            },
            x,
            y,
        })
    }

    /// Whether the sprite of RGSS2 is drawn from a picture although it is not turned:
    /// stretch quads do not draw what it shows of `bitmap`, the rectangle of its bitmap.
    pub(crate) fn is_pictured_rgss2(&self, bitmap: Rect) -> bool {
        self.angle == 0.0
            && self.drawn_wave_amp() <= 0
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
        let columns = |across: Run| -> Vec<Option<usize>> {
            (i64::from(left)..)
                .take(width)
                .map(|i| across.texel(i, source.width))
                .collect()
        };
        let columns = [columns(shape.across), columns(shape.faint_across())];
        let faint_from = i64::from(shape.size.1 - shape.bush);
        for (j, pixels) in (i64::from(top)..).zip(rgba.chunks_exact_mut(width * 4)) {
            let faint = usize::from(shape.bush > 0 && j >= faint_from);
            let Some(v) = shape.down[faint].and_then(|run| run.texel(j, source.height)) else {
                continue;
            };
            let columns = &columns[faint];
            for (pixel, u) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(columns) {
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
        if sprite.drawn_wave_amp() > 0 {
            self.waving_sprite_rgss2(sprite, bitmap, level, look);
            return;
        }
        let faint = sprite.bush_look_rgss2(look);
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
        let mirror = shape.across.mirror;
        for (run, look, mirror) in [
            (shape.down[0], Some(look), mirror),
            (shape.down[1], faint, false),
        ] {
            let (Some(run), Some(look)) = (run, look) else {
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
            self.stretched(bitmap, part, into, mirror, clip, look);
        }
    }
}

/// The sine of a whole number of degrees as the wave of a sprite takes it: in single
/// precision.
fn wave_sine(degrees: u32) -> f32 {
    libm::sin(f64::from(degrees).to_radians()) as f32
}

/// The number of pixels a row of a waving sprite of RGSS2 is moved to the right by. `row`
/// counts the rows of the sprite from its top row, `screen_row` is where that row stands
/// on the screen and `clip_top` the first screen row its viewport, or the screen, lets it
/// draw.
///
/// ```text
/// band: the screen rows are grouped by screen_row div 8
/// r0 = the first row of the band that is drawn, counted like `row`: the first of the
///      band, of the sprite or of the clip, whichever is lowest on the screen
/// t  = phase + r0 * (360.0 / length), then (row - r0) times t = t + 1.0 / length
/// dx = round(amp * SIN[floor(t) mod 360]), an exact half toward zero
/// ```
///
/// in double precision, with the sines of whole degrees in single precision. The offset
/// is therefore the same over a band of 8 screen rows, except where the small term carries
/// the angle over a whole degree. The order of the operations decides the rows that sit
/// exactly on a degree. A length of 0 gives no offset; no sprite is drawn with one
/// ([`Sprite::drawn_wave_amp`]).
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E, topics `wave1` to `wave6` (78400
/// rows of 196 sprites with amplitudes of 1 to 30 and 250 and 20 lengths, and amplitudes up
/// to 3000 near 0 degrees, without a differing pixel); the rows of
/// `tests/data/sprite-rgss2-wave-samples.txt` are those at which the offset changes. For a
/// negative length the angle is taken modulo 360 here: RGSS2 was not measured with one, and
/// the original runtime of RGSS3 reads outside its sine table there, so that its two
/// versions show different pictures (`ace-v1-sprite-REPORT.md`, 2026-10-13).
pub(crate) fn wave_offset(
    amp: i32,
    length: i32,
    phase: f64,
    row: i64,
    screen_row: i64,
    clip_top: i64,
) -> i64 {
    if length == 0 {
        return 0;
    }
    let band = screen_row - screen_row.rem_euclid(8);
    let first = (row - (screen_row - band.max(clip_top))).max(0);
    let length = f64::from(length);
    let mut t = phase + first as f64 * (360.0 / length);
    let step = 1.0 / length;
    for _ in first..row {
        t += step;
    }
    // Less than 360.
    let degrees = (t.floor() as i64).rem_euclid(360) as u32;
    let value = f64::from(amp) * f64::from(wave_sine(degrees));
    let whole = value.abs().floor();
    let rounded = if value.abs() - whole > 0.5 {
        whole + 1.0
    } else {
        whole
    };
    // At most the amplitude.
    (if value < 0.0 { -rounded } else { rounded }) as i64
}

/// Rows of a waving sprite that one stretch quad draws: `rows` screen rows from row `first`
/// of the sprite on, all moved by `dx`, faint or not, which show the texel row `texel` and
/// then either the same row again or the next row each (`step`, open while there is one
/// row).
#[derive(Debug, Clone, Copy)]
struct Strip {
    first: i64,
    rows: u32,
    texel: i64,
    step: Option<i64>,
    dx: i64,
    faint: bool,
}

impl Strip {
    /// Takes the next row of the sprite, if it goes on as the strip does.
    fn take(&mut self, texel: i64, dx: i64, faint: bool) -> bool {
        let last = self.texel + self.step.unwrap_or(0) * i64::from(self.rows - 1);
        let step = texel - last;
        let fits = match self.step {
            None => step == 0 || step == 1,
            Some(known) => step == known,
        };
        if !fits || dx != self.dx || faint != self.faint {
            return false;
        }
        self.step = Some(step);
        self.rows += 1;
        true
    }
}

impl Assembly<'_> {
    /// A sprite of RGSS2 with a positive wave amplitude that is not turned: every screen
    /// row is moved by [`wave_offset`] inside the clip of its level and shows
    ///
    /// ```text
    /// rows:    src.y + (j * h) div dh for row j, instead of the stretch walk
    /// columns: the stretch walk, mirrored; texels outside the bitmap are only left out
    /// bush:    the lowest min(bush_depth, dh) rows are bush rows; the rows are not split
    /// ```
    ///
    /// It becomes one stretch quad for each run of rows that share the offset and show
    /// either one row of the bitmap or one row each.
    ///
    /// Not measured: a sprite stretched in x whose source rectangle leaves the bitmap. The
    /// model of the measurement leaves the texels outside out of the walk and is known to
    /// be wrong by a column there; here the walk starts anew at the bitmap's edge, as a
    /// stretch quad draws it.
    pub(crate) fn waving_sprite_rgss2(
        &mut self,
        sprite: &Sprite,
        bitmap: BitmapId,
        level: &Level,
        look: Look,
    ) {
        let src = sprite.src_rect;
        let across = geometry::stretch_span_rgss2(sprite.x, sprite.ox, src.width, sprite.zoom_x);
        let down = geometry::stretch_span_rgss2(sprite.y, sprite.oy, src.height, sprite.zoom_y);
        let (Some((x, width)), Some((y, height))) = (across, down) else {
            return;
        };
        let (h, dh) = (i64::from(src.height), i64::from(height));
        let faint_from = dh - i64::from(sprite.bush_depth).clamp(0, dh);
        let faint = sprite.bush_look_rgss2(look);
        let (left, top) = (i64::from(x) + level.dx, i64::from(y) + level.dy);
        let clip = level.clip;
        let clip_top = i64::from(clip.y);
        let mut draw = |strip: Strip| {
            let look = if strip.faint { faint } else { Some(look) };
            let texels = if strip.step == Some(1) { strip.rows } else { 1 };
            // Rows that begin beyond the coordinates of a rectangle are not drawn.
            let (Some(look), Ok(texel), Ok(texels), Ok(x), Ok(y)) = (
                look,
                i32::try_from(strip.texel),
                i32::try_from(texels),
                i32::try_from(left + strip.dx),
                i32::try_from(top + strip.first),
            ) else {
                return;
            };
            let part = Rect::new(src.x, texel, src.width, texels);
            let into = vkxp_proto::Rect::new(x, y, width, strip.rows);
            let mirror = sprite.mirror && !strip.faint;
            self.stretched(bitmap, part, into, mirror, clip, look);
        };
        let mut strip: Option<Strip> = None;
        let rows = (clip_top - top).max(0)..(clip_top + i64::from(clip.height) - top).min(dh);
        for j in rows {
            let (amp, length, phase) = (sprite.wave_amp, sprite.wave_length, sprite.wave_phase);
            let dx = wave_offset(amp, length, phase, j, top + j, clip_top);
            let (texel, faint) = (i64::from(src.y) + j * h / dh, j >= faint_from);
            if strip
                .as_mut()
                .is_some_and(|strip| strip.take(texel, dx, faint))
            {
                continue;
            }
            let next = Strip {
                first: j,
                rows: 1,
                texel,
                step: None,
                dx,
                faint,
            };
            if let Some(done) = strip.replace(next) {
                draw(done);
            }
        }
        if let Some(done) = strip {
            draw(done);
        }
    }
}

#[cfg(test)]
mod tests;
