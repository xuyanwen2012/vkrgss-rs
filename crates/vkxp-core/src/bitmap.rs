//! `Bitmaps`: the arena that owns every bitmap, and the operations that draw into them.

use vkxp_pixel::{Rgba, blend_normal, block_span, rotate_hue, stretch_index, transfer_opacity};
use vkxp_proto::BitmapId;

use crate::{BitmapError, Color, CreateFailure, Font, FontDefaults, Rect};

mod rgss2;

/// Largest bitmap that can be created, in pixels. It equals the limit of the compositors'
/// bitmap stores, so every bitmap created here can be uploaded.
pub const MAX_BITMAP_PIXELS: u64 = 1 << 28;

/// A rectangle of pixels inside a bitmap: `left..right` by `top..bottom`, never empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Area {
    pub(crate) left: usize,
    pub(crate) top: usize,
    pub(crate) right: usize,
    pub(crate) bottom: usize,
}

impl Area {
    /// The smallest area containing both.
    pub(crate) fn union(self, other: Area) -> Area {
        Area {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

/// The part of the span `pos..pos + len` that lies inside `0..size`, or `None` when no part
/// does. A `len` of 0 or less is an empty span.
pub(crate) fn clip_span(pos: i32, len: i32, size: usize) -> Option<(usize, usize)> {
    let start = i64::from(pos).max(0);
    let end = (i64::from(pos) + i64::from(len)).min(size as i64);
    (start < end).then_some((start as usize, end as usize))
}

/// One bitmap: its pixels in the protocol's layout (red, green, blue, straight alpha, row
/// order), its font, the area changed since the last update sent to the host, and whether the
/// host knows the bitmap at all.
#[derive(Debug, Clone)]
pub(crate) struct Bitmap {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) rgba: Vec<u8>,
    pub(crate) font: Font,
    pub(crate) changed: Option<Area>,
    /// Whether a create for this bitmap was emitted.
    pub(crate) known: bool,
    /// Counts the drawing operations that may have changed pixels, so that whatever is
    /// derived from the pixels can tell when it is out of date.
    pub(crate) revision: u64,
}

impl Bitmap {
    pub(crate) fn pixel(&self, x: usize, y: usize) -> Rgba {
        let offset = (y * self.width + x) * 4;
        [
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
            self.rgba[offset + 3],
        ]
    }

    pub(crate) fn put(&mut self, x: usize, y: usize, pixel: Rgba) {
        let offset = (y * self.width + x) * 4;
        self.rgba[offset..offset + 4].copy_from_slice(&pixel);
    }

    /// The whole bitmap.
    pub(crate) fn bounds(&self) -> Area {
        Area {
            left: 0,
            top: 0,
            right: self.width,
            bottom: self.height,
        }
    }

    /// The part of `rect` that lies inside the bitmap.
    pub(crate) fn clip(&self, rect: Rect) -> Option<Area> {
        let (left, right) = clip_span(rect.x, rect.width, self.width)?;
        let (top, bottom) = clip_span(rect.y, rect.height, self.height)?;
        Some(Area {
            left,
            top,
            right,
            bottom,
        })
    }

    /// Records that pixels inside `area` may have changed.
    pub(crate) fn mark(&mut self, area: Area) {
        self.revision += 1;
        self.changed = Some(match self.changed {
            Some(changed) => changed.union(area),
            None => area,
        });
    }

    /// Sets every pixel of `area` to `pixel` and marks it.
    fn fill(&mut self, area: Area, pixel: Rgba) {
        for y in area.top..area.bottom {
            let start = (y * self.width + area.left) * 4;
            let end = (y * self.width + area.right) * 4;
            for target in self.rgba[start..end].as_chunks_mut::<4>().0 {
                *target = pixel;
            }
        }
        self.mark(area);
    }
}

/// One slot of the arena. `generation` is that of the current occupant, or of the last one
/// when the slot is empty.
#[derive(Debug, Clone)]
struct Slot {
    generation: u32,
    bitmap: Option<Bitmap>,
}

/// For each position of a destination span that a scaled transfer draws, the destination
/// coordinate and the source coordinate it takes, as measured on the original runtime. A
/// transfer of equal sizes goes through [`block_axis`] instead.
///
/// A `dst_len` of 0 or less and a `src_len` of 0 draw nothing. Otherwise the source span is
/// walked over `span = |src_len|` texels, from `src_pos` upward, or from `src_pos - 1` downward
/// when `src_len` is negative (mirroring). Position `i` of `0..dst_len` lands on `dst_pos + i`
/// and takes the texel `stretch_index(i, span, dst_len)` steps into the walk, with these rules
/// where the walk leaves `0..src_size`:
///
/// - When the walk starts `c` texels before the source, the first position drawn is
///   `c * dst_len / span`, and the steps are counted again from 0 there, at the first texel of
///   the source.
/// - When only the first `v` texels of the walk exist, with `v` below `span`, positions stop
///   before `(v * dst_len - 1) / span`.
///
/// Clipping by `0..dst_size` leaves `i` counted from the unclipped start. The destination
/// coordinates of the result are consecutive.
fn transfer_axis(
    (dst_pos, dst_len, dst_size): (i32, i32, usize),
    (src_pos, src_len, src_size): (i32, i32, usize),
) -> Vec<(usize, usize)> {
    if src_len == 0 {
        return Vec::new();
    }
    let Some((start, end)) = clip_span(dst_pos, dst_len, dst_size) else {
        return Vec::new();
    };
    let mirrored = src_len < 0;
    let (dst_len, span) = (i64::from(dst_len), i64::from(src_len).abs());
    let (src_pos, src_size) = (i64::from(src_pos), src_size as i64);
    // Texels of the walk that lie before the source, and texels from the start of the walk to
    // the far edge of the source.
    let (before, valid) = if mirrored {
        (src_pos - src_size, src_pos)
    } else {
        (-src_pos, src_size - src_pos)
    };
    let before = before.max(0);
    let first = before * dst_len / span;
    let stop = if valid >= span {
        dst_len
    } else {
        (valid * dst_len - 1).div_euclid(span)
    };
    (start..end)
        .filter_map(|dst| {
            // `dst` is inside the span, so the difference is in `0..dst_len`.
            let i = dst as i64 - i64::from(dst_pos);
            if i < first || i >= stop {
                return None;
            }
            let steps = stretch_index((i - first) as u32, span as u32, dst_len as u32);
            let offset = before + i64::from(steps);
            let src = if mirrored {
                src_pos - 1 - offset
            } else {
                src_pos + offset
            };
            (src >= 0 && src < src_size).then_some((dst, src as usize))
        })
        .collect()
}

/// One axis of a transfer of equal sizes: the pairs of [`transfer_axis`], by the clips of the
/// original ([`vkxp_pixel::block_span`]). A span with a negative `src_len` is clipped before
/// it is mirrored.
fn block_axis(
    (dst_pos, dst_size): (i32, usize),
    (src_pos, src_len, src_size): (i32, i32, usize),
) -> Vec<(usize, usize)> {
    // Sides are at most `MAX_BITMAP_PIXELS`, which fits `u32`.
    let Some(span) = block_span(dst_pos, dst_size as u32, src_pos, src_len, src_size as u32) else {
        return Vec::new();
    };
    (0..span.len)
        .map(|i| {
            let src = if span.mirrored {
                span.src - i
            } else {
                span.src + i
            };
            ((span.dst + i) as usize, src as usize)
        })
        .collect()
}

/// Blends source pixels onto `dst` at `opacity`, row by row from the top and from left to
/// right within a row. `columns` and `rows` come from [`transfer_axis`] or [`block_axis`]; `source` is called
/// with `dst` as it is at that moment and with indices into them.
fn blend_into(
    dst: &mut Bitmap,
    columns: &[(usize, usize)],
    rows: &[(usize, usize)],
    opacity: u8,
    source: impl Fn(&Bitmap, usize, usize) -> Rgba,
) {
    let (Some(first_column), Some(last_column)) = (columns.first(), columns.last()) else {
        return;
    };
    let (Some(first_row), Some(last_row)) = (rows.first(), rows.last()) else {
        return;
    };
    for (row, &(y, _)) in rows.iter().enumerate() {
        for (column, &(x, _)) in columns.iter().enumerate() {
            let blended = blend_normal(dst.pixel(x, y), source(dst, column, row), opacity);
            dst.put(x, y, blended);
        }
    }
    dst.mark(Area {
        left: first_column.0,
        top: first_row.0,
        right: last_column.0 + 1,
        bottom: last_row.0 + 1,
    });
}

/// The arena that owns every bitmap.
///
/// A bitmap is named by its protocol [`BitmapId`]: `index` is its slot and `generation`
/// increases every time a slot gets a new occupant, so no two bitmaps of one arena ever share
/// an id. An id whose slot is empty or holds another generation names a disposed bitmap, and
/// every operation on it other than [`Bitmaps::dispose`] and [`Bitmaps::is_disposed`] fails
/// with [`BitmapError::Disposed`] without changing anything. As the source of a transfer a
/// disposed bitmap is no error ([`Bitmaps::blt`]).
///
/// Coordinates and rectangles are in pixels with the origin at the top left corner.
#[derive(Debug, Clone, Default)]
pub struct Bitmaps {
    slots: Vec<Slot>,
    /// Empty slots that can take a new occupant.
    free: Vec<u32>,
    /// Disposed bitmaps the host still knows, in the order they were disposed.
    pub(crate) destroyed: Vec<BitmapId>,
}

impl Bitmaps {
    pub fn new() -> Bitmaps {
        Bitmaps::default()
    }

    /// Number of live bitmaps.
    pub fn len(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.bitmap.is_some())
            .count()
    }

    /// Whether there is no live bitmap.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Creates a bitmap whose pixels are all transparent black. Its font is created from
    /// `defaults`.
    pub fn create(
        &mut self,
        width: u32,
        height: u32,
        defaults: &FontDefaults,
    ) -> Result<BitmapId, BitmapError> {
        let bytes = Bitmaps::byte_len(width, height)?;
        Ok(self.insert(width, height, vec![0; bytes], defaults)?)
    }

    /// Creates a bitmap from pixels given as red, green, blue and straight alpha bytes in row
    /// order. Its font is created from `defaults`.
    pub fn create_from_rgba(
        &mut self,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        defaults: &FontDefaults,
    ) -> Result<BitmapId, BitmapError> {
        let bytes = Bitmaps::byte_len(width, height)?;
        if rgba.len() != bytes {
            return Err(CreateFailure::PixelLength {
                width,
                height,
                expected: bytes as u64,
                found: rgba.len(),
            }
            .into());
        }
        Ok(self.insert(width, height, rgba, defaults)?)
    }

    /// The number of bytes a bitmap of this size holds, if such a bitmap may be created.
    fn byte_len(width: u32, height: u32) -> Result<usize, CreateFailure> {
        if width == 0 || height == 0 {
            return Err(CreateFailure::ZeroSide { width, height });
        }
        let pixels = u64::from(width) * u64::from(height);
        let too_large = CreateFailure::TooLarge {
            width,
            height,
            limit: MAX_BITMAP_PIXELS,
        };
        if pixels > MAX_BITMAP_PIXELS {
            return Err(too_large);
        }
        usize::try_from(pixels * 4).map_err(|_| too_large)
    }

    fn insert(
        &mut self,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        defaults: &FontDefaults,
    ) -> Result<BitmapId, CreateFailure> {
        let bitmap = Bitmap {
            width: width as usize,
            height: height as usize,
            rgba,
            font: Font::new(defaults),
            changed: None,
            known: false,
            revision: 0,
        };
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            // Slots whose generations are used up never reach the free list.
            slot.generation += 1;
            slot.bitmap = Some(bitmap);
            return Ok(BitmapId::new(index, slot.generation));
        }
        let index = u32::try_from(self.slots.len()).map_err(|_| CreateFailure::ArenaFull)?;
        self.slots.push(Slot {
            generation: 0,
            bitmap: Some(bitmap),
        });
        Ok(BitmapId::new(index, 0))
    }

    /// Disposes a bitmap and frees its pixels. Disposing a disposed bitmap does nothing. If
    /// the host knows the bitmap, the next [`Bitmaps::deltas`] tells it to destroy it.
    pub fn dispose(&mut self, id: BitmapId) {
        let Some(slot) = self.slots.get_mut(id.index as usize) else {
            return;
        };
        if slot.generation != id.generation {
            return;
        }
        let Some(bitmap) = slot.bitmap.take() else {
            return;
        };
        if bitmap.known {
            self.destroyed.push(id);
        }
        // A slot at the last generation is retired, so that no id is ever handed out twice.
        if slot.generation < u32::MAX {
            self.free.push(id.index);
        }
    }

    /// Whether `id` names no live bitmap.
    pub fn is_disposed(&self, id: BitmapId) -> bool {
        self.get(id).is_err()
    }

    /// Every live bitmap.
    pub(crate) fn live_mut(&mut self) -> impl Iterator<Item = &mut Bitmap> {
        self.slots
            .iter_mut()
            .filter_map(|slot| slot.bitmap.as_mut())
    }

    pub(crate) fn get(&self, id: BitmapId) -> Result<&Bitmap, BitmapError> {
        self.slots
            .get(id.index as usize)
            .filter(|slot| slot.generation == id.generation)
            .and_then(|slot| slot.bitmap.as_ref())
            .ok_or(BitmapError::Disposed)
    }

    pub(crate) fn get_mut(&mut self, id: BitmapId) -> Result<&mut Bitmap, BitmapError> {
        self.slots
            .get_mut(id.index as usize)
            .filter(|slot| slot.generation == id.generation)
            .and_then(|slot| slot.bitmap.as_mut())
            .ok_or(BitmapError::Disposed)
    }

    /// A number that is different after every operation that may have changed the bitmap's
    /// pixels.
    pub(crate) fn revision(&self, id: BitmapId) -> Result<u64, BitmapError> {
        Ok(self.get(id)?.revision)
    }

    pub fn width(&self, id: BitmapId) -> Result<u32, BitmapError> {
        Ok(self.get(id)?.width as u32)
    }

    pub fn height(&self, id: BitmapId) -> Result<u32, BitmapError> {
        Ok(self.get(id)?.height as u32)
    }

    /// The rectangle covering the whole bitmap.
    pub fn rect(&self, id: BitmapId) -> Result<Rect, BitmapError> {
        let bitmap = self.get(id)?;
        // Sides are at most `MAX_BITMAP_PIXELS`, which fits `i32`.
        Ok(Rect::new(0, 0, bitmap.width as i32, bitmap.height as i32))
    }

    /// All pixels: red, green, blue and straight alpha bytes in row order.
    pub fn rgba(&self, id: BitmapId) -> Result<&[u8], BitmapError> {
        Ok(&self.get(id)?.rgba)
    }

    /// The font text is drawn into this bitmap with.
    pub fn font(&self, id: BitmapId) -> Result<&Font, BitmapError> {
        Ok(&self.get(id)?.font)
    }

    /// The bitmap's font, for changing single attributes.
    pub fn font_mut(&mut self, id: BitmapId) -> Result<&mut Font, BitmapError> {
        Ok(&mut self.get_mut(id)?.font)
    }

    /// Replaces the bitmap's font as a whole.
    pub fn set_font(&mut self, id: BitmapId, font: Font) -> Result<(), BitmapError> {
        self.get_mut(id)?.font = font;
        Ok(())
    }

    /// The colour of a pixel. Outside the bitmap it is the colour with all components 0.
    pub fn get_pixel(&self, id: BitmapId, x: i32, y: i32) -> Result<Color, BitmapError> {
        let bitmap = self.get(id)?;
        Ok(match bitmap.clip(Rect::new(x, y, 1, 1)) {
            Some(area) => Color::from_proto(vkxp_proto::Color::from_rgba(
                bitmap.pixel(area.left, area.top),
            )),
            None => Color::TRANSPARENT,
        })
    }

    /// Replaces all four channels of a pixel with `color` as a stored pixel
    /// ([`Color::to_pixel`]). Outside the bitmap nothing changes.
    pub fn set_pixel(
        &mut self,
        id: BitmapId,
        x: i32,
        y: i32,
        color: Color,
    ) -> Result<(), BitmapError> {
        self.fill_rect(id, Rect::new(x, y, 1, 1), color)
    }

    /// Sets every pixel to transparent black.
    pub fn clear(&mut self, id: BitmapId) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        bitmap.fill(bitmap.bounds(), [0; 4]);
        Ok(())
    }

    /// Sets every pixel inside both `rect` and the bitmap to `color` as a stored pixel
    /// ([`Color::to_pixel`]), alpha included, without blending. A rectangle with a width or height of 0 or less fills nothing.
    pub fn fill_rect(&mut self, id: BitmapId, rect: Rect, color: Color) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        if let Some(area) = bitmap.clip(rect) {
            bitmap.fill(area, color.to_pixel());
        }
        Ok(())
    }

    /// Block transfer: draws `src_rect` of `src` with its top left corner at (`x`, `y`) of
    /// `dst`, at `opacity`, combining with the blend the original runtime uses for a
    /// destination with alpha ([`vkxp_pixel::blend_normal`]).
    ///
    /// `opacity` is the integer a script gave: 1 to 255 draw, anything else draws nothing,
    /// values above 255 included ([`vkxp_pixel::transfer_opacity`]).
    ///
    /// Only the part of `src_rect` inside the source bitmap is drawn, at the offset it has
    /// within the rectangle, and only where it lands inside the destination. A negative width
    /// mirrors: column `i` takes source column `src_rect.x - 1 - i`; a negative height
    /// likewise. A mirrored rectangle that leaves the source bitmap is clipped before it is
    /// mirrored ([`vkxp_pixel::block_span`]): the part that exists is drawn mirrored within
    /// itself, where the unmirrored part would be.
    ///
    /// A disposed `src`, or `None` for a bitmap that has no id any more, is no error. The original then takes the destination as the source
    /// and the destination rectangle as the source rectangle: every pixel of the rectangle
    /// at (`x`, `y`) with the absolute size of `src_rect`, inside `dst`, is blended onto
    /// itself at `opacity`, which keeps its colour and raises its alpha. The position of
    /// `src_rect` is not used. A disposed `dst` fails as everywhere.
    ///
    /// `src` may be `dst`: the transfer then works in place. It writes the rows from top to
    /// bottom and the pixels of a row from left to right, and reads each source pixel as the
    /// bitmap is at that moment, pixels this transfer has already written included. A shift to
    /// the right or downwards therefore repeats the first columns or rows.
    ///
    /// Provenance: oracle measurement, RGSS103J, self transfer probe
    /// (`ph-scene-selfblt-probe.rb`: 197 cases of `blt` and `stretch_blt`, every pixel equal);
    /// the opacity, the mirrored clip and the disposed source: oracle measurement 2026-10-12,
    /// RGSS103J and RGSS202E, the same bytes on both.
    pub fn blt(
        &mut self,
        dst: BitmapId,
        x: i32,
        y: i32,
        src: impl Into<Option<BitmapId>>,
        src_rect: Rect,
        opacity: impl Into<i32>,
    ) -> Result<(), BitmapError> {
        let dst_rect = Rect::new(
            x,
            y,
            src_rect.width.saturating_abs(),
            src_rect.height.saturating_abs(),
        );
        self.transfer(dst, dst_rect, src.into(), src_rect, opacity.into(), true)
    }

    /// Stretched transfer: draws `src_rect` of `src` scaled into `dst_rect` of `dst` without
    /// interpolation, as measured on the original runtime. Column `i` of `dst_rect`, counted
    /// from its unclipped left edge, takes column
    /// `stretch_index(i, src_rect.width, dst_rect.width)` of `src_rect`, and rows likewise
    /// ([`vkxp_pixel::stretch_index`]). Blending and opacity are those of [`Bitmaps::blt`].
    ///
    /// A destination rectangle with a width or height of 0 or less draws nothing, as does a
    /// source rectangle with a width or height of 0. A negative source width or height mirrors.
    /// A source rectangle that leaves the source bitmap is clipped by the measured rules: where
    /// it starts `c` texels before the bitmap, the first column drawn is
    /// `c * dst_rect.width / src_rect.width` and takes the bitmap's first column, with `i`
    /// counted from there; where only its first `v` columns exist, columns stop before
    /// `(v * dst_rect.width - 1) / src_rect.width`. When both rectangles have the same absolute
    /// size the transfer is a block transfer, clipped as [`Bitmaps::blt`] is. A disposed `src`
    /// is treated as there, with `dst_rect` as the rectangle.
    pub fn stretch_blt(
        &mut self,
        dst: BitmapId,
        dst_rect: Rect,
        src: impl Into<Option<BitmapId>>,
        src_rect: Rect,
        opacity: impl Into<i32>,
    ) -> Result<(), BitmapError> {
        let block = i64::from(dst_rect.width) == i64::from(src_rect.width).abs()
            && i64::from(dst_rect.height) == i64::from(src_rect.height).abs();
        self.transfer(dst, dst_rect, src.into(), src_rect, opacity.into(), block)
    }

    /// The transfer behind [`Bitmaps::blt`] and [`Bitmaps::stretch_blt`]; `block` says that
    /// both rectangles have the same absolute size.
    fn transfer(
        &mut self,
        dst: BitmapId,
        dst_rect: Rect,
        src: Option<BitmapId>,
        src_rect: Rect,
        opacity: i32,
        block: bool,
    ) -> Result<(), BitmapError> {
        let (dst_width, dst_height) = {
            let bitmap = self.get(dst)?;
            (bitmap.width, bitmap.height)
        };
        // A disposed source: the destination rectangle is blended onto itself.
        let (src, src_rect, block) = match src.filter(|&src| !self.is_disposed(src)) {
            Some(src) => (src, src_rect, block),
            None => (dst, dst_rect, true),
        };
        let (src_width, src_height) = {
            let bitmap = self.get(src)?;
            (bitmap.width, bitmap.height)
        };
        let Some(opacity) = transfer_opacity(opacity) else {
            return Ok(());
        };
        if dst_rect.width <= 0 || dst_rect.height <= 0 {
            return Ok(());
        }
        let (columns, rows) = if block {
            (
                block_axis(
                    (dst_rect.x, dst_width),
                    (src_rect.x, src_rect.width, src_width),
                ),
                block_axis(
                    (dst_rect.y, dst_height),
                    (src_rect.y, src_rect.height, src_height),
                ),
            )
        } else {
            (
                transfer_axis(
                    (dst_rect.x, dst_rect.width, dst_width),
                    (src_rect.x, src_rect.width, src_width),
                ),
                transfer_axis(
                    (dst_rect.y, dst_rect.height, dst_height),
                    (src_rect.y, src_rect.height, src_height),
                ),
            )
        };
        if columns.is_empty() || rows.is_empty() {
            return Ok(());
        }

        if dst == src {
            // In place: every source pixel is read when its turn comes.
            let bitmap = self.get_mut(dst)?;
            blend_into(bitmap, &columns, &rows, opacity, |bitmap, column, row| {
                bitmap.pixel(columns[column].1, rows[row].1)
            });
            return Ok(());
        }

        let [dst_slot, src_slot] = self
            .slots
            .get_disjoint_mut([dst.index as usize, src.index as usize])
            .map_err(|_| BitmapError::Disposed)?;
        let (Some(dst_bitmap), Some(src_bitmap)) = (dst_slot.bitmap.as_mut(), &src_slot.bitmap)
        else {
            return Err(BitmapError::Disposed);
        };
        blend_into(dst_bitmap, &columns, &rows, opacity, |_, column, row| {
            src_bitmap.pixel(columns[column].1, rows[row].1)
        });
        Ok(())
    }

    /// Rotates the hue of every pixel by `degrees` ([`vkxp_pixel::rotate_hue`]), with the
    /// integer arithmetic measured on the original runtime. Alpha is kept. An angle of 0
    /// changes nothing; every other angle, 360 included, goes through the arithmetic, which
    /// does not give every colour back.
    pub fn hue_change(&mut self, id: BitmapId, degrees: i32) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        if degrees == 0 {
            return Ok(());
        }
        for pixel in bitmap.rgba.as_chunks_mut::<4>().0 {
            *pixel = rotate_hue(*pixel, degrees);
        }
        bitmap.mark(bitmap.bounds());
        Ok(())
    }
}

/// Helpers shared by the tests of this crate.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub(crate) fn defaults() -> FontDefaults {
        FontDefaults::new(vec!["Test".to_owned()])
    }

    /// Pixels that differ at every position and in every channel.
    pub(crate) fn pattern(width: u32, height: u32, seed: u8) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let n = (y * width + x) as u8;
                rgba.extend_from_slice(&[
                    seed.wrapping_add(n.wrapping_mul(7)),
                    seed.wrapping_mul(3).wrapping_add(n.wrapping_mul(13)),
                    seed.wrapping_mul(5).wrapping_add(n.wrapping_mul(29)),
                    seed.wrapping_mul(11).wrapping_add(n.wrapping_mul(37)),
                ]);
            }
        }
        rgba
    }

    /// The pixel at (`x`, `y`) of pixels `width` wide.
    pub(crate) fn at(rgba: &[u8], width: u32, x: u32, y: u32) -> Rgba {
        let offset = ((y * width + x) * 4) as usize;
        rgba[offset..offset + 4].try_into().unwrap()
    }

    /// A colour with the given integer channels.
    pub(crate) fn color(rgba: Rgba) -> Color {
        Color::from_proto(vkxp_proto::Color::from_rgba(rgba))
    }
}

#[cfg(test)]
mod tests;
