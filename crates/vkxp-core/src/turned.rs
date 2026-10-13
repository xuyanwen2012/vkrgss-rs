//! Sprites drawn from a picture of their own: a sprite with an angle other than 0, and a
//! stretched sprite whose source rectangle leaves its bitmap, are rendered on the CPU, by
//! the rules measured on the original runtime, into a bitmap of the scene's own, and drawn
//! from it as an unscaled quad.
//!
//! For a turned sprite the original runtime does the same: it walks the bounding box of the
//! turned source rectangle row by row in double precision ([`Geometry::turned_box`],
//! [`Geometry::turned_row`]), puts the texels into a temporary bitmap and blends that like a
//! sprite that is not turned. A stretch of a cut source rectangle shows columns and rows
//! that no stretch of a rectangle inside the bitmap shows, which is why it is rendered here
//! too ([`Span`]). The bitmap lives in the bitmap arena and is rendered again only when what
//! it was rendered from changes; a sprite that turns every frame is rendered every frame.
//!
//! Only the part of the box inside the sprite's level is rendered, so the bitmap is never
//! larger than the screen. The pixels depend on the position of the sprite only through
//! where the box and that part lie against it, so a sprite that moves keeps its bitmap.

use vkxp_proto::{BitmapId, STRETCH_FRAC_BITS, stretch_step};

use crate::extract::rgss2::{Placed, Shape};
use crate::extract::{Assembly, Bush, Level, Look, Rows, intersect, rows};
use crate::geometry::{self, Geometry, TurnedBox};
use crate::{Bitmaps, FontDefaults, MAX_BITMAP_PIXELS, Rect, Rules, Sprite};

/// What the bitmap of a sprite was rendered from.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Key {
    Turned(TurnedKey),
    Cut(CutKey),
    Upright(UprightKey),
}

/// What the bitmap of an unrotated sprite of RGSS2 was rendered from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct UprightKey {
    source: BitmapId,
    revision: u64,
    shape: Shape,
    /// Left and top of the rendered part, from the corner of the sprite, and its size.
    part: (u32, u32, u32, u32),
}

/// What the bitmap of a stretched sprite with a cut source rectangle was rendered from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CutKey {
    source: BitmapId,
    revision: u64,
    src: Rect,
    /// The size of the whole sprite on the screen.
    size: (u32, u32),
    mirror: bool,
    /// The number of faint rows, which are stretched on their own.
    bush: u32,
    /// Left and top of the rendered part, from the corner of the sprite, and its size.
    part: (u32, u32, u32, u32),
}

/// What the bitmap of a turned sprite was rendered from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TurnedKey {
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

/// The bitmap a turned sprite, or a stretched one with a cut source rectangle, is drawn from.
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

/// Whether `src` lies entirely inside `bitmap`, the rectangle of a bitmap.
fn inside(src: Rect, bitmap: Rect) -> bool {
    src.x >= 0
        && src.y >= 0
        && i64::from(src.x) + i64::from(src.width) <= i64::from(bitmap.width)
        && i64::from(src.y) + i64::from(src.height) <= i64::from(bitmap.height)
}

/// One stretch along one axis of a sprite that is not turned: `length` texels from `start`
/// on, drawn into `pixels` pixels from pixel `first` of the sprite on. Neither is 0.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Span {
    pub(crate) start: i64,
    pub(crate) length: u32,
    pub(crate) first: u32,
    pub(crate) pixels: u32,
    /// Counts the texels from the far end.
    pub(crate) mirror: bool,
    /// Whether the texels outside the bitmap cut the stretch as they cut a `stretch_blt`,
    /// or are only left out of it.
    pub(crate) cut: bool,
}

impl Span {
    fn of(rows: Rows, cut: bool) -> Span {
        Span {
            start: rows.start,
            length: rows.length,
            first: rows.first,
            pixels: rows.pixels,
            mirror: false,
            cut,
        }
    }

    /// The texel that pixel `i` of the sprite shows along this axis, in a bitmap of `size`
    /// texels along it, or `None` when it shows none.
    ///
    /// With `k` the index of a texel inside the stretched texels in the order they are
    /// drawn, `[a, b)` the indices that lie inside the bitmap, and `w` texels drawn into
    /// `dw` pixels, the original runtime draws
    ///
    /// ```text
    /// step  = (w << 20) / dw
    /// first = a * dw / w
    /// end   = (b * dw - 1) / w  if b < w  else dw
    /// pixel i of first .. end-1 shows texel k = a + (((i - first) * step) >> 20) when k < b
    /// ```
    ///
    /// so a stretch cut at its near end starts anew at the first texel of the bitmap, and
    /// one cut at its far end loses its last pixel. A mirrored stretch is mirrored before
    /// it is cut. Where the texels are only left out, `k = (i * step) >> 20` is shown when
    /// it lies in `[a, b)`.
    ///
    /// Provenance: oracle measurement 2026-10-10, RGSS103J, probes `srcrect` and `srcrect2`
    /// (112 sprites that are not turned, without a differing pixel).
    pub(crate) fn texel(&self, i: u32, size: usize) -> Option<usize> {
        let i = i128::from(i.checked_sub(self.first).filter(|i| *i < self.pixels)?);
        let (start, w, dw) = (
            i128::from(self.start),
            i128::from(self.length),
            i128::from(self.pixels),
        );
        let step = i128::from(stretch_step(self.length, self.pixels));
        let (mut a, mut b) = ((-start).max(0), w.min(size as i128 - start));
        if self.mirror {
            (a, b) = (w - b, w - a);
        }
        if a >= b {
            return None;
        }
        let k = if self.cut {
            let first = a * dw / w;
            let end = if b < w { (b * dw - 1) / w } else { dw };
            if i < first || i >= end {
                return None;
            }
            a + (((i - first) * step) >> STRETCH_FRAC_BITS)
        } else {
            (i * step) >> STRETCH_FRAC_BITS
        };
        (a <= k && k < b).then(|| {
            let texel = if self.mirror {
                start + w - 1 - k
            } else {
                start + k
            };
            // Inside the bitmap, as `k` is one of `[a, b)`.
            texel as usize
        })
    }
}

/// Where a stretched sprite with a cut source rectangle is drawn inside a level.
struct Cut {
    /// The pixels the whole sprite covers, on the screen.
    dest: vkxp_proto::Rect,
    across: Span,
    /// The rows above the faint ones and the faint ones, each stretched on its own, or all
    /// rows as the first when none is faint.
    down: [Option<Span>; 2],
    /// The number of faint rows.
    bush: u32,
    /// The pixels of `dest` that lie inside the level. Never empty.
    part: vkxp_proto::Rect,
}

/// What a sprite is rendered from this frame.
enum Picture {
    Turned(Placement),
    Cut(Cut),
    Upright(Placed),
}

impl Picture {
    /// The pixels on the screen that the rendered bitmap covers.
    fn part(&self) -> vkxp_proto::Rect {
        match self {
            Picture::Turned(placement) => placement.part,
            Picture::Cut(cut) => cut.part,
            Picture::Upright(placed) => placed.part,
        }
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
        if !inside(src, bitmap) {
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

    /// Whether the sprite is drawn from a picture although it is not turned: it is
    /// stretched on either axis and its source rectangle leaves `bitmap`, the rectangle of
    /// its bitmap.
    pub(crate) fn is_cut(&self, bitmap: Rect) -> bool {
        let src = self.src_rect;
        let (Some(width), Some(height)) = (
            geometry::stretch_length(src.width, self.zoom_x),
            geometry::stretch_length(src.height, self.zoom_y),
        ) else {
            return false;
        };
        let stretched =
            i64::from(width) != i64::from(src.width) || i64::from(height) != i64::from(src.height);
        self.angle == 0.0 && stretched && !inside(src, bitmap)
    }

    /// Where the sprite, cut as [`Sprite::is_cut`] says, is drawn inside `level`, or `None`
    /// when nothing of it is.
    fn cut_placement(&self, level: &Level, bitmap: Rect) -> Option<Cut> {
        if !self.is_cut(bitmap) {
            return None;
        }
        let dest = self.upright_dest(level)?;
        let part = intersect(dest, level.clip);
        if part.is_empty() {
            return None;
        }
        let src = self.src_rect;
        let bush = Bush::new(src, dest.height, self.bush_depth);
        // Without faint rows the rows are cut as the columns are.
        let whole = bush.faint == 0;
        Some(Cut {
            dest,
            across: Span {
                start: i64::from(src.x),
                // Positive, as the sprite has a length on the screen.
                length: src.width.unsigned_abs(),
                first: 0,
                pixels: dest.width,
                mirror: self.mirror,
                cut: true,
            },
            down: [bush.upper, bush.lower].map(|rows| rows.map(|rows| Span::of(rows, whole))),
            bush: bush.faint,
            part,
        })
    }

    /// What the sprite is rendered from inside `level`, with what tells that apart from
    /// every other picture of it, or `None` when it draws from no picture there.
    fn picture(&self, bitmaps: &Bitmaps, level: &Level) -> Option<(Key, Picture)> {
        let source = self.bitmap?;
        let revision = bitmaps.revision(source).ok()?;
        let bitmap = bitmaps.rect(source).ok()?;
        if self.angle == 0.0 && self.rules == Rules::Rgss2 {
            let placed = self.placed_rgss2(level, bitmap)?;
            let key = UprightKey {
                source,
                revision,
                shape: placed.shape,
                part: placed.offsets(),
            };
            return Some((Key::Upright(key), Picture::Upright(placed)));
        }
        if self.angle == 0.0 {
            let cut = self.cut_placement(level, bitmap)?;
            let (dest, part) = (cut.dest, cut.part);
            let key = CutKey {
                source,
                revision,
                src: self.src_rect,
                size: (dest.width, dest.height),
                mirror: self.mirror,
                bush: cut.bush,
                // The part lies inside the sprite.
                part: (
                    part.x.abs_diff(dest.x),
                    part.y.abs_diff(dest.y),
                    part.width,
                    part.height,
                ),
            };
            return Some((Key::Cut(key), Picture::Cut(cut)));
        }
        let placement = self.turned_placement(level, bitmap)?;
        let (bounds, part) = (placement.bounds, placement.part);
        // Where the box is computed for, from the corner of the viewport and on the screen.
        let (x, y) = (i64::from(self.x) - level.ox, i64::from(self.y) - level.oy);
        let (screen_x, screen_y) = (x + placement.corner.0, y + placement.corner.1);
        let key = TurnedKey {
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
        Some((Key::Turned(key), Picture::Turned(placement)))
    }

    /// Brings the bitmap of a sprite that draws from a picture up to date, if this frame
    /// draws from one. A sprite that needs none, is invisible or has nothing inside its
    /// level gives it back; one whose viewport is hidden, so that it has no `level`, keeps
    /// what it has.
    pub(crate) fn refresh_turned(&mut self, bitmaps: &mut Bitmaps, level: Option<&Level>) {
        let source = self.bitmap.and_then(|id| bitmaps.rect(id).ok());
        let pictured = self.angle != 0.0
            || source.is_some_and(|bitmap| match self.rules {
                Rules::Rgss1 => self.is_cut(bitmap),
                Rules::Rgss2 => self.is_pictured_rgss2(bitmap),
            });
        if !self.visible || !pictured {
            self.turned.dispose(bitmaps);
            return;
        }
        let Some(level) = level else {
            return;
        };
        let Some((key, picture)) = self.picture(bitmaps, level) else {
            self.turned.dispose(bitmaps);
            return;
        };
        let lost = self.turned.bitmap.is_some_and(|id| bitmaps.is_disposed(id));
        if self.turned.key == Some(key) && !lost {
            return;
        }
        let rgba = match &picture {
            Picture::Turned(placement) => self.render_turned(bitmaps, placement),
            Picture::Cut(cut) => self.render_cut(bitmaps, cut),
            Picture::Upright(placed) => self.render_rgss2(bitmaps, placed),
        };
        let part = picture.part();
        let size = Rect::new(0, 0, part.width as i32, part.height as i32);
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
                let defaults = FontDefaults::new(Vec::new());
                rgba.and_then(|rgba| {
                    bitmaps
                        .create_from_rgba(part.width, part.height, rgba, &defaults)
                        .ok()
                })
            }
        };
        self.turned.key = Some(key);
        self.turned.bitmap = target;
    }

    /// The pixels of the part of the sprite that `cut` names: the texel every pixel shows,
    /// and transparent black where it shows none. `None` when the source is disposed.
    fn render_cut(&self, bitmaps: &Bitmaps, cut: &Cut) -> Option<Vec<u8>> {
        let source = bitmaps.get(self.bitmap?).ok()?;
        let (part, dest) = (cut.part, cut.dest);
        let (width, height) = (part.width as usize, part.height as usize);
        let mut rgba = vec![0; width * height * 4];
        // The part lies inside the sprite.
        let (left, top) = (part.x.abs_diff(dest.x), part.y.abs_diff(dest.y));
        let columns: Vec<Option<usize>> = (left..left + part.width)
            .map(|i| cut.across.texel(i, source.width))
            .collect();
        for (j, pixels) in (top..).zip(rgba.chunks_exact_mut(width * 4)) {
            let spans = cut.down.iter().flatten();
            let Some(v) = spans.filter_map(|span| span.texel(j, source.height)).next() else {
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
        let faint = Look {
            opacity: look.opacity >> 1,
            ..look
        };
        self.picture(bitmap, placement.part, split, level.clip, look, faint);
    }

    /// A stretched sprite whose source rectangle leaves its bitmap: the bitmap it was
    /// rendered into, unscaled, where the rendered part of it lies, the faint rows with
    /// half the opacity.
    pub(crate) fn cut_sprite(&mut self, sprite: &Sprite, level: &Level, look: Look) {
        let Some(bitmap) = sprite.turned.bitmap else {
            return;
        };
        let Some(source) = sprite.bitmap.and_then(|id| self.bitmaps.rect(id).ok()) else {
            return;
        };
        let Some(cut) = sprite.cut_placement(level, source) else {
            return;
        };
        let bottom = i64::from(cut.dest.y) + i64::from(cut.dest.height);
        let split = bottom - i64::from(cut.bush);
        let faint = Look {
            opacity: look.opacity >> 1,
            ..look
        };
        self.picture(bitmap, cut.part, split, level.clip, look, faint);
    }

    /// The whole of `bitmap`, unscaled, at `part`: the rows above `split` as `look` says and
    /// those from it on, the faint rows, as `faint` says.
    pub(crate) fn picture(
        &mut self,
        bitmap: BitmapId,
        part: vkxp_proto::Rect,
        split: i64,
        clip: vkxp_proto::Rect,
        look: Look,
        faint: Look,
    ) {
        let src = Rect::new(0, 0, part.width as i32, part.height as i32);
        let above = rows(clip, i64::MIN, split);
        let below = rows(clip, split, i64::MAX);
        self.stretched(bitmap, src, part, false, above, look);
        self.stretched(bitmap, src, part, false, below, faint);
    }
}
