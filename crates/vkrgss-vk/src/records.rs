//! Packet validation and the encoding of ops into the records the shader reads.
//!
//! Validation happens here, on the CPU, before anything is written for the device: the checks
//! and their order are those of the reference compositor, so both reject the same packets with
//! the same error. The shader then never sees an invalid slot or source rectangle.

use vkrgss_proto::{
    BitmapId, BlendMode, Color, Op, Plane, PlaneRules, Rect, Sampling, Size, TilemapLayer,
    TilemapPlane, Tone, stretch_step, tiles_rgss2, tiles_rgss3,
};

use crate::VkError;
use crate::bitmaps::{BitmapTable, Entry};
use crate::shared::{self, NO_BITMAP, WHOLE_TILES};
use crate::tables::TableStore;

pub(crate) use vkrgss_proto::MAX_LOGICAL_DIMENSION;

/// Number of 32-bit words in one record; `OpRecord` in `shaders/records.glsl` has this layout.
pub(crate) const RECORD_WORDS: usize = shared::RECORD_WORDS as usize;

/// Number of 32-bit words in the record of a tilemap layer; `TilemapRecord` in
/// `shaders/records.glsl` has this layout.
pub(crate) const TILEMAP_WORDS: usize = shared::TILEMAP_WORDS as usize;

// The word each field of an op record is in. The encoder writes through these names, and a
// test shows that the field of the same name of the GLSL `OpRecord` is read from that word.
pub(crate) const OP_KIND: usize = 0;
pub(crate) const OP_CLIP_X0: usize = 1;
pub(crate) const OP_CLIP_Y0: usize = 2;
pub(crate) const OP_CLIP_X1: usize = 3;
pub(crate) const OP_CLIP_Y1: usize = 4;
pub(crate) const OP_BITMAP: usize = 5;
pub(crate) const OP_SRC_X: usize = 6;
pub(crate) const OP_SRC_Y: usize = 7;
pub(crate) const OP_SRC_WIDTH: usize = 8;
pub(crate) const OP_SRC_HEIGHT: usize = 9;
/// The first of the [`TRANSFORM_WORDS`] words of `transform`: `xx`, `xy`, `yx`, `yy`, `tx`,
/// `ty`.
pub(crate) const OP_TRANSFORM: usize = 10;
pub(crate) const OP_OPACITY: usize = 16;
pub(crate) const OP_BLEND: usize = 17;
pub(crate) const OP_COLOR: usize = 18;
pub(crate) const OP_TONE_RED: usize = 19;
pub(crate) const OP_TONE_GREEN: usize = 20;
pub(crate) const OP_TONE_BLUE: usize = 21;
pub(crate) const OP_TONE_GRAY: usize = 22;
pub(crate) const OP_SAMPLING: usize = 23;

/// Number of words of the `transform` field of an op record.
pub(crate) const TRANSFORM_WORDS: usize = 6;

// The word each field of a tilemap record is in, checked like those of an op record.
pub(crate) const TILEMAP_MAP_OFFSET: usize = 0;
pub(crate) const TILEMAP_MAP_XSIZE: usize = 1;
pub(crate) const TILEMAP_MAP_YSIZE: usize = 2;
pub(crate) const TILEMAP_PRIORITIES_OFFSET: usize = 3;
pub(crate) const TILEMAP_PRIORITIES_COUNT: usize = 4;
pub(crate) const TILEMAP_PRIORITY: usize = 5;
pub(crate) const TILEMAP_CELL_X: usize = 6;
pub(crate) const TILEMAP_CELL_Y: usize = 7;
pub(crate) const TILEMAP_SUB_X: usize = 8;
pub(crate) const TILEMAP_SUB_Y: usize = 9;
pub(crate) const TILEMAP_TILESET: usize = 10;
/// The first of the [`AUTOTILES`] words of `autotiles`.
pub(crate) const TILEMAP_AUTOTILES: usize = 11;
/// The first of the [`AUTOTILES`] words of `frames`.
pub(crate) const TILEMAP_FRAMES: usize = 18;

/// Number of autotiles of a tilemap layer.
pub(crate) const AUTOTILES: usize = 7;

/// What a tilemap layer op reads besides its clip, as the shader reads it: in this order, the
/// index of the first element of the map's layer in the table buffer, the map's width and
/// height in cells, the index of the first priority and the number of priorities, the
/// priority drawn, the cell under the screen's top-left pixel (wrapped into the map) and the
/// place of that pixel in the cell, the slot of the tileset, the slots of the seven
/// autotiles, and the animation frame of each autotile.
pub(crate) type TilemapRecord = [u32; TILEMAP_WORDS];

/// The `blend` word of a quad.
const fn blend_mode(blend: BlendMode) -> u32 {
    match blend {
        BlendMode::Normal => shared::BLEND_NORMAL,
        BlendMode::Add => shared::BLEND_ADD,
        BlendMode::Subtract => shared::BLEND_SUBTRACT,
        BlendMode::Window => shared::BLEND_WINDOW,
    }
}

/// Rejects a logical size outside 1 to [`MAX_LOGICAL_DIMENSION`] per side.
pub(crate) fn check_logical_size(size: Size) -> Result<(), VkError> {
    let valid = 1..=MAX_LOGICAL_DIMENSION;
    if valid.contains(&size.width) && valid.contains(&size.height) {
        Ok(())
    } else {
        Err(VkError::InvalidLogicalSize {
            size,
            limit: MAX_LOGICAL_DIMENSION,
        })
    }
}

/// A non-empty rectangle of screen pixels: `x0 <= x < x1` and `y0 <= y < y1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClipRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl ClipRect {
    /// The part of `clip` on a screen of `size`, or `None` when no pixel is.
    pub(crate) fn on_screen(clip: Rect, size: Size) -> Option<ClipRect> {
        let span = |start: i32, length: u32, limit: u32| {
            let start = i64::from(start);
            let low = start.max(0);
            let high = (start + i64::from(length)).min(i64::from(limit));
            (low < high).then_some((low as u32, high as u32))
        };
        let (x0, x1) = span(clip.x, clip.width, size.width)?;
        let (y0, y1) = span(clip.y, clip.height, size.height)?;
        Some(ClipRect { x0, y0, x1, y1 })
    }

    /// The part of this rectangle inside `rect`, or `None` when no pixel is.
    fn within(self, rect: Rect) -> Option<ClipRect> {
        let span = |low: u32, high: u32, start: i32, length: u32| {
            let start = i64::from(start);
            let low = i64::from(low).max(start);
            let high = i64::from(high).min(start + i64::from(length));
            (low < high).then_some((low as u32, high as u32))
        };
        let (x0, x1) = span(self.x0, self.x1, rect.x, rect.width)?;
        let (y0, y1) = span(self.y0, self.y1, rect.y, rect.height)?;
        Some(ClipRect { x0, y0, x1, y1 })
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, x: u32, y: u32) -> bool {
        (self.x0..self.x1).contains(&x) && (self.y0..self.y1).contains(&y)
    }
}

/// One validated op as the shader reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpRecord {
    /// The op's clip rectangle intersected with the screen.
    pub clip: ClipRect,
    words: [u32; RECORD_WORDS],
}

impl OpRecord {
    fn new(kind: u32, clip: ClipRect) -> OpRecord {
        let mut words = [0; RECORD_WORDS];
        words[OP_KIND] = kind;
        words[OP_CLIP_X0] = clip.x0;
        words[OP_CLIP_Y0] = clip.y0;
        words[OP_CLIP_X1] = clip.x1;
        words[OP_CLIP_Y1] = clip.y1;
        OpRecord { clip, words }
    }

    /// Fills the colour word and the four tone words, which a quad and a rectangle effect share.
    fn set_tone_and_color(&mut self, tone: Tone, color: Color) {
        self.words[OP_COLOR] = u32::from_le_bytes(color.to_rgba());
        // Sign-extended: the shader reads these three words as signed integers.
        self.words[OP_TONE_RED] = i32::from(tone.red) as u32;
        self.words[OP_TONE_GREEN] = i32::from(tone.green) as u32;
        self.words[OP_TONE_BLUE] = i32::from(tone.blue) as u32;
        self.words[OP_TONE_GRAY] = u32::from(tone.gray);
    }

    /// The record's words, in the order of the fields of the GLSL `OpRecord`.
    pub(crate) fn words(&self) -> &[u32; RECORD_WORDS] {
        &self.words
    }
}

/// The records of one frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Records {
    pub ops: Vec<OpRecord>,
    /// The record of a tilemap layer op, and of a tilemap plane op, names one of these by
    /// its index.
    pub tilemaps: Vec<TilemapRecord>,
}

/// [`encode_frame`] without tables, for ops that name none.
#[cfg(test)]
pub(crate) fn encode_ops(
    table: &BitmapTable,
    size: Size,
    ops: &[Op],
) -> Result<Vec<OpRecord>, VkError> {
    encode_frame(table, &TableStore::new(), size, ops).map(|records| records.ops)
}

/// Checks a tilemap layer op and encodes what it reads, or `None` when it cannot draw: its
/// map has no cells or lacks the layer.
///
/// Everything that needs 64-bit integers or the size of a sheet is worked out here: the
/// origin becomes a cell inside the map and a place inside that cell, and the animation step
/// becomes a frame of each autotile.
fn encode_tilemap(
    bitmaps: &BitmapTable,
    tables: &TableStore,
    index: usize,
    layer: &TilemapLayer,
) -> Result<Option<TilemapRecord>, VkError> {
    // A sheet without pixels has no image; nothing can be read from it.
    let sheet = |id: Option<BitmapId>| -> Result<Option<(u32, &Entry)>, VkError> {
        let Some(id) = id else { return Ok(None) };
        let entry = bitmaps
            .get(id)
            .map_err(|source| VkError::InvalidOpBitmap { index, source })?;
        Ok((entry.width > 0 && entry.height > 0).then_some((id.index, entry)))
    };
    let table = |id| {
        tables
            .get(id)
            .map_err(|source| VkError::InvalidOpTable { index, source })
    };
    let tileset = sheet(layer.tileset)?;
    let mut autotiles = [None; AUTOTILES];
    for (found, id) in autotiles.iter_mut().zip(layer.autotiles) {
        *found = sheet(id)?;
    }
    let (map, priorities) = (table(layer.map)?, table(layer.priorities)?);
    if map.xsize == 0 || map.ysize == 0 || u32::from(layer.layer) >= map.zsize {
        return Ok(None);
    }

    let cells = u64::from(map.xsize) * u64::from(map.ysize);
    let along = |origin: i32, size: u32| {
        let origin = i64::from(origin);
        // A remainder of a `u32`, and one of 32.
        let cell = origin.div_euclid(32).rem_euclid(i64::from(size)) as u32;
        (cell, origin.rem_euclid(32) as u32)
    };
    let (cell_x, sub_x) = along(layer.origin.0, map.xsize);
    let (cell_y, sub_y) = along(layer.origin.1, map.ysize);
    let mut record = [0; TILEMAP_WORDS];
    // Offsets and lengths are below the length of the table buffer, which `u32` holds for
    // every store within the device's limit; beyond it no frame is drawn.
    record[TILEMAP_MAP_OFFSET] = (map.offset + cells * u64::from(layer.layer)) as u32;
    record[TILEMAP_MAP_XSIZE] = map.xsize;
    record[TILEMAP_MAP_YSIZE] = map.ysize;
    record[TILEMAP_PRIORITIES_OFFSET] = priorities.offset as u32;
    record[TILEMAP_PRIORITIES_COUNT] = u64::from(priorities.xsize).min(priorities.len()) as u32;
    record[TILEMAP_PRIORITY] = u32::from(layer.priority);
    record[TILEMAP_CELL_X] = cell_x;
    record[TILEMAP_CELL_Y] = cell_y;
    record[TILEMAP_SUB_X] = sub_x;
    record[TILEMAP_SUB_Y] = sub_y;
    record[TILEMAP_TILESET] = tileset.map_or(NO_BITMAP, |(slot, _)| slot);
    for (slot, autotile) in autotiles.into_iter().enumerate() {
        let Some((bitmap, entry)) = autotile else {
            record[TILEMAP_AUTOTILES + slot] = NO_BITMAP;
            continue;
        };
        let (width, flag) = if entry.height <= 32 {
            (32, WHOLE_TILES)
        } else {
            (96, 0)
        };
        let frames = u64::from((entry.width / width).max(1));
        record[TILEMAP_AUTOTILES + slot] = bitmap;
        // Less than the number of frames, which is far below the flag.
        record[TILEMAP_FRAMES + slot] = (layer.animation_step % frames) as u32 | flag;
    }
    Ok(Some(record))
}

/// Checks a tilemap plane op, of RGSS2 or of RGSS3, and encodes what it reads, in the words of a
/// [`TilemapRecord`] as `shaders/records.glsl` says a plane reads them, or `None` when it
/// cannot draw: its map has no cells.
///
/// The sheets are checked first, then the map, the passages and the flash table, as the
/// reference compositor checks them. Everything that needs 64-bit integers is worked out
/// here: the first cell inside the map and inside the flash table, and the animation step
/// as the frames it selects.
fn encode_plane(
    bitmaps: &BitmapTable,
    tables: &TableStore,
    index: usize,
    plane: &TilemapPlane,
) -> Result<Option<TilemapRecord>, VkError> {
    let mut slots = [NO_BITMAP; tiles_rgss2::SHEETS];
    for (slot, id) in slots.iter_mut().zip(plane.bitmaps) {
        let Some(id) = id else { continue };
        let entry = bitmaps
            .get(id)
            .map_err(|source| VkError::InvalidOpBitmap { index, source })?;
        // A sheet without pixels has no image; nothing can be read from it.
        if entry.width > 0 && entry.height > 0 {
            *slot = id.index;
        }
    }
    let table = |id| {
        tables
            .get(id)
            .map_err(|source| VkError::InvalidOpTable { index, source })
    };
    let map = table(plane.map)?;
    let passages = plane.passages.map(table).transpose()?;
    let flash = plane.flash.map(table).transpose()?;
    if map.xsize == 0 || map.ysize == 0 {
        return Ok(None);
    }

    // Remainders of sizes that are `u32`.
    let inside = |first: u32, size: u32| (u64::from(first) % u64::from(size)) as u32;
    let step = plane.animation_step;
    // The layers the rule set reads, and the rule set, in bits 1 to 4.
    let rules = match plane.rules {
        PlaneRules::Rgss2 => map.zsize.min(3) << 1,
        PlaneRules::Rgss3 => map.zsize.min(tiles_rgss3::LAYERS) << 1 | 1 << 4,
    };
    let flags = u32::from(plane.plane == Plane::Upper)
        | rules
        | u32::from(plane.flash_level) << 8
        | tiles_rgss2::FLOOR_FRAMES[(step % 4) as usize] << 16
        | ((step % 3) as u32) << 18;
    let mut record = [0; TILEMAP_WORDS];
    // Offsets and lengths are below the length of the table buffer, which `u32` holds for
    // every store within the device's limit; beyond it no frame is drawn.
    record[TILEMAP_MAP_OFFSET] = map.offset as u32;
    record[TILEMAP_MAP_XSIZE] = map.xsize;
    record[TILEMAP_MAP_YSIZE] = map.ysize;
    if let Some(passages) = passages {
        record[TILEMAP_PRIORITIES_OFFSET] = passages.offset as u32;
        record[TILEMAP_PRIORITIES_COUNT] = u64::from(passages.xsize).min(passages.len()) as u32;
    }
    record[TILEMAP_PRIORITY] = flags;
    record[TILEMAP_CELL_X] = inside(plane.first_cell.0, map.xsize);
    record[TILEMAP_CELL_Y] = inside(plane.first_cell.1, map.ysize);
    record[TILEMAP_SUB_X] = plane.corner.0 as u32;
    record[TILEMAP_SUB_Y] = plane.corner.1 as u32;
    record[TILEMAP_TILESET] = slots[0];
    record[TILEMAP_AUTOTILES..TILEMAP_AUTOTILES + AUTOTILES].copy_from_slice(&slots[1..8]);
    record[TILEMAP_FRAMES] = slots[8];
    if let Some(flash) = flash.filter(|flash| flash.len() > 0) {
        record[TILEMAP_FRAMES + 1] = flash.offset as u32;
        record[TILEMAP_FRAMES + 2] = flash.xsize;
        record[TILEMAP_FRAMES + 3] = flash.ysize;
        record[TILEMAP_FRAMES + 4] = inside(plane.first_cell.0, flash.xsize);
        record[TILEMAP_FRAMES + 5] = inside(plane.first_cell.1, flash.ysize);
    }
    Ok(Some(record))
}

/// Checks every op, in order, and encodes those that can change a pixel.
///
/// An op whose clip misses the screen, a quad with an empty source rectangle, a tilemap
/// layer whose map has no cell for it and a tilemap plane whose map has no cells or which
/// lies beside its clip is checked like any other but yields no record: it
/// changes no pixel. So a record's clip is never empty
/// and a quad record always names a slot that holds an image.
///
/// A stretch quad covers exactly its destination rectangle, so its record's clip is the part
/// of the clip inside that rectangle, and the shader needs no test of its own. The two steps
/// are divided here, where 64-bit integers are at hand.
pub(crate) fn encode_frame(
    table: &BitmapTable,
    tables: &TableStore,
    size: Size,
    ops: &[Op],
) -> Result<Records, VkError> {
    let mut records = Vec::with_capacity(ops.len());
    let mut tilemaps = Vec::new();
    for (index, op) in ops.iter().enumerate() {
        let clip = ClipRect::on_screen(op.clip(), size);
        match op {
            Op::Clear { color, .. } => {
                let Some(clip) = clip else { continue };
                let mut record = OpRecord::new(shared::OP_CLEAR, clip);
                record.words[OP_COLOR] = u32::from_le_bytes(color.to_rgba());
                records.push(record);
            }
            Op::Quad(quad) => {
                let bitmap = table
                    .get(quad.bitmap)
                    .map_err(|source| VkError::InvalidOpBitmap { index, source })?;
                if !bitmap.bounds().contains_rect(&quad.src) {
                    return Err(VkError::SourceOutOfBounds {
                        index,
                        bitmap: quad.bitmap,
                        src: quad.src,
                        width: bitmap.width,
                        height: bitmap.height,
                    });
                }
                let Some(clip) = clip else { continue };
                if quad.src.is_empty() {
                    continue;
                }
                let src = quad.src;
                let (clip, kind, geometry) = match quad.sampling {
                    Sampling::Affine(t) => {
                        let coefficients = [t.xx, t.xy, t.yx, t.yy, t.tx, t.ty];
                        let words = coefficients.map(|c| c.to_bits() as u32);
                        (clip, shared::SAMPLING_AFFINE, words)
                    }
                    Sampling::Stretch { dest, mirror } => {
                        let Some(clip) = clip.within(dest) else {
                            continue;
                        };
                        // Not empty, or nothing of the clip would be inside it.
                        let step_x = stretch_step(src.width, dest.width);
                        let step_y = stretch_step(src.height, dest.height);
                        let kind = if mirror {
                            shared::SAMPLING_STRETCH_MIRRORED
                        } else {
                            shared::SAMPLING_STRETCH
                        };
                        let words = [
                            dest.x as u32,
                            dest.y as u32,
                            (step_x >> 32) as u32,
                            step_x as u32,
                            (step_y >> 32) as u32,
                            step_y as u32,
                        ];
                        (clip, kind, words)
                    }
                };
                let mut record = OpRecord::new(shared::OP_QUAD, clip);
                record.words[OP_BITMAP] = quad.bitmap.index;
                // Inside the bitmap and not empty, so the origin is not negative and every
                // value is at most the largest image dimension.
                record.words[OP_SRC_X] = src.x as u32;
                record.words[OP_SRC_Y] = src.y as u32;
                record.words[OP_SRC_WIDTH] = src.width;
                record.words[OP_SRC_HEIGHT] = src.height;
                record.words[OP_TRANSFORM..OP_TRANSFORM + TRANSFORM_WORDS]
                    .copy_from_slice(&geometry);
                record.words[OP_SAMPLING] = kind;
                record.words[OP_OPACITY] = u32::from(quad.opacity);
                record.words[OP_BLEND] = blend_mode(quad.blend);
                record.set_tone_and_color(quad.tone, quad.color);
                records.push(record);
            }
            Op::RectEffect(effect) => {
                let Some(clip) = clip else { continue };
                let mut record = OpRecord::new(shared::OP_RECT_EFFECT, clip);
                record.set_tone_and_color(effect.tone, effect.color);
                records.push(record);
            }
            Op::TilemapLayer(layer) => {
                let tilemap = encode_tilemap(table, tables, index, layer)?;
                let (Some(clip), Some(tilemap)) = (clip, tilemap) else {
                    continue;
                };
                let mut record = OpRecord::new(shared::OP_TILEMAP, clip);
                record.words[OP_BITMAP] = tilemaps.len() as u32;
                tilemaps.push(tilemap);
                records.push(record);
            }
            Op::TilemapPlane(plane) => {
                let tilemap = encode_plane(table, tables, index, plane)?;
                // The record's clip is the part of the clip inside the plane.
                let area = Rect::new(
                    plane.corner.0,
                    plane.corner.1,
                    tiles_rgss2::PLANE_WIDTH,
                    tiles_rgss2::PLANE_HEIGHT,
                );
                let clip = clip.and_then(|clip| clip.within(area));
                let (Some(clip), Some(tilemap)) = (clip, tilemap) else {
                    continue;
                };
                let mut record = OpRecord::new(shared::OP_TILEMAP_PLANE, clip);
                record.words[OP_BITMAP] = tilemaps.len() as u32;
                tilemaps.push(tilemap);
                records.push(record);
            }
        }
    }
    Ok(Records {
        ops: records,
        tilemaps,
    })
}

#[cfg(test)]
mod tests {
    use vkrgss_proto::{
        BitmapDelta, BitmapId, Color, Fixed, Quad, RectEffect, Sampling, Tone, Transform,
    };

    use super::*;
    use crate::{BitmapError, SlotState};

    const SIZE: Size = Size::new(40, 30);
    const SCREEN: Rect = Rect::from_size(SIZE);
    const ID: BitmapId = BitmapId::new(3, 1);

    /// A table holding an 8x6 bitmap as `ID`.
    fn table() -> BitmapTable {
        let mut table = BitmapTable::new(4096);
        table
            .apply(&BitmapDelta::Create {
                id: ID,
                width: 8,
                height: 6,
            })
            .unwrap();
        table
    }

    fn quad(bitmap: BitmapId, src: Rect) -> Op {
        Op::Quad(Quad {
            clip: SCREEN,
            bitmap,
            src,
            sampling: Sampling::Affine(Transform::IDENTITY),
            tone: Tone::NEUTRAL,
            color: Color::new(0, 0, 0, 0),
            opacity: 255,
            blend: BlendMode::Normal,
        })
    }

    fn clear(clip: Rect) -> Op {
        Op::Clear {
            clip,
            color: Color::new(1, 2, 3, 4),
        }
    }

    #[test]
    fn logical_size_must_be_between_one_and_the_limit() {
        for size in [
            Size::new(1, 1),
            Size::new(641, 479),
            Size::new(MAX_LOGICAL_DIMENSION, MAX_LOGICAL_DIMENSION),
        ] {
            assert_eq!(check_logical_size(size), Ok(()));
        }
        for size in [
            Size::new(0, 10),
            Size::new(10, 0),
            Size::new(MAX_LOGICAL_DIMENSION + 1, 1),
            Size::new(1, u32::MAX),
        ] {
            assert_eq!(
                check_logical_size(size),
                Err(VkError::InvalidLogicalSize { size, limit: 16384 })
            );
        }
    }

    #[test]
    fn clip_is_intersected_with_the_screen() {
        let on_screen = |x, y, w, h| ClipRect::on_screen(Rect::new(x, y, w, h), SIZE);
        let rect = |x0, y0, x1, y1| Some(ClipRect { x0, y0, x1, y1 });
        assert_eq!(on_screen(0, 0, 40, 30), rect(0, 0, 40, 30));
        assert_eq!(on_screen(5, 6, 7, 8), rect(5, 6, 12, 14));
        assert_eq!(on_screen(-5, -6, 7, 8), rect(0, 0, 2, 2));
        assert_eq!(on_screen(39, 29, 100, 100), rect(39, 29, 40, 30));
        assert_eq!(
            on_screen(i32::MIN, i32::MIN, u32::MAX, u32::MAX),
            rect(0, 0, 40, 30)
        );
        assert_eq!(on_screen(i32::MAX, 0, u32::MAX, 1), None);
        // Touching the screen from outside, or empty, covers no pixel.
        assert_eq!(on_screen(40, 0, 5, 5), None);
        assert_eq!(on_screen(0, 30, 5, 5), None);
        assert_eq!(on_screen(-5, 0, 5, 5), None);
        assert_eq!(on_screen(0, -5, 5, 5), None);
        assert_eq!(on_screen(3, 3, 0, 5), None);
        assert_eq!(on_screen(3, 3, 5, 0), None);
    }

    #[test]
    fn stale_bitmap_reference_names_the_op() {
        let stale = BitmapId::new(3, 0);
        let ops = [clear(SCREEN), quad(stale, Rect::new(0, 0, 1, 1))];
        assert_eq!(
            encode_ops(&table(), SIZE, &ops),
            Err(VkError::InvalidOpBitmap {
                index: 1,
                source: BitmapError::InvalidReference {
                    id: stale,
                    slot: SlotState::Occupied { generation: 1 },
                },
            })
        );
    }

    #[test]
    fn unknown_bitmap_reference_names_the_op() {
        let unknown = BitmapId::new(9, 0);
        let ops = [quad(unknown, Rect::new(0, 0, 1, 1))];
        assert_eq!(
            encode_ops(&table(), SIZE, &ops),
            Err(VkError::InvalidOpBitmap {
                index: 0,
                source: BitmapError::InvalidReference {
                    id: unknown,
                    slot: SlotState::Empty,
                },
            })
        );
    }

    #[test]
    fn source_rectangle_outside_the_bitmap_names_the_op() {
        for src in [
            Rect::new(0, 0, 9, 6),
            Rect::new(1, 0, 8, 1),
            Rect::new(-1, 0, 2, 2),
            Rect::new(0, 5, 1, 2),
        ] {
            let ops = [clear(SCREEN), clear(SCREEN), quad(ID, src)];
            assert_eq!(
                encode_ops(&table(), SIZE, &ops),
                Err(VkError::SourceOutOfBounds {
                    index: 2,
                    bitmap: ID,
                    src,
                    width: 8,
                    height: 6,
                })
            );
        }
    }

    #[test]
    fn the_first_invalid_op_is_the_one_reported() {
        let ops = [
            quad(ID, Rect::new(0, 0, 9, 9)),
            quad(BitmapId::new(9, 0), SCREEN),
        ];
        assert!(matches!(
            encode_ops(&table(), SIZE, &ops),
            Err(VkError::SourceOutOfBounds { index: 0, .. })
        ));
    }

    /// An op that cannot touch the screen is still checked.
    #[test]
    fn off_screen_ops_are_validated_but_emit_no_record() {
        let off = Rect::new(100, 100, 5, 5);
        let valid = [
            clear(off),
            Op::Quad(Quad {
                clip: off,
                bitmap: ID,
                src: Rect::new(0, 0, 8, 6),
                sampling: Sampling::Affine(Transform::IDENTITY),
                tone: Tone::NEUTRAL,
                color: Color::new(0, 0, 0, 0),
                opacity: 255,
                blend: BlendMode::Normal,
            }),
            Op::RectEffect(RectEffect {
                clip: off,
                tone: Tone::NEUTRAL,
                color: Color::default(),
            }),
        ];
        assert_eq!(encode_ops(&table(), SIZE, &valid), Ok(vec![]));

        let invalid = [Op::Quad(Quad {
            clip: off,
            bitmap: BitmapId::new(9, 0),
            src: Rect::new(0, 0, 1, 1),
            sampling: Sampling::Affine(Transform::IDENTITY),
            tone: Tone::NEUTRAL,
            color: Color::new(0, 0, 0, 0),
            opacity: 255,
            blend: BlendMode::Normal,
        })];
        assert!(matches!(
            encode_ops(&table(), SIZE, &invalid),
            Err(VkError::InvalidOpBitmap { index: 0, .. })
        ));
    }

    /// An empty source rectangle is inside any bitmap wherever it lies, and draws nothing.
    #[test]
    fn quad_with_an_empty_source_emits_no_record() {
        let mut table = table();
        let empty = BitmapId::new(4, 0);
        table
            .apply(&BitmapDelta::Create {
                id: empty,
                width: 0,
                height: 5,
            })
            .unwrap();
        let ops = [
            quad(ID, Rect::new(-3, 900, 0, 7)),
            quad(empty, Rect::new(0, 0, 0, 5)),
        ];
        assert_eq!(encode_ops(&table, SIZE, &ops), Ok(vec![]));
        // A source with pixels is outside a bitmap that has none.
        assert!(matches!(
            encode_ops(&table, SIZE, &[quad(empty, Rect::new(0, 0, 1, 1))]),
            Err(VkError::SourceOutOfBounds { index: 0, .. })
        ));
    }

    #[test]
    fn clear_record_layout() {
        let records = encode_ops(&table(), SIZE, &[clear(Rect::new(-2, 3, 10, 100))]).unwrap();
        let mut expected = [0; RECORD_WORDS];
        expected[..5].copy_from_slice(&[0, 0, 3, 8, 30]);
        expected[18] = 0x0403_0201;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].words(), &expected);
        assert_eq!(
            records[0].clip,
            ClipRect {
                x0: 0,
                y0: 3,
                x1: 8,
                y1: 30
            }
        );
    }

    #[test]
    fn quad_record_layout() {
        let op = Op::Quad(Quad {
            clip: Rect::new(1, 2, 3, 4),
            bitmap: ID,
            src: Rect::new(2, 1, 5, 4),
            sampling: Sampling::Affine(Transform {
                xx: Fixed::from_bits(11),
                xy: Fixed::from_bits(-12),
                yx: Fixed::from_bits(13),
                yy: Fixed::from_bits(i32::MIN),
                tx: Fixed::from_bits(15),
                ty: Fixed::from_bits(-1),
            }),
            tone: Tone::new(-255, i16::MAX, i16::MIN, 77),
            color: Color::new(255, 0, 128, 96),
            opacity: 200,
            blend: BlendMode::Subtract,
        });
        let records = encode_ops(&table(), SIZE, &[op]).unwrap();
        #[rustfmt::skip]
        let expected = [
            1, 1, 2, 4, 6,
            3, 2, 1, 5, 4,
            11, -12i32 as u32, 13, 0x8000_0000, 15, u32::MAX,
            200, 2,
            0x6080_00FF, -255i32 as u32, 32767, -32768i32 as u32, 77,
            0,
        ];
        assert_eq!(records[0].words(), &expected);
    }

    fn stretch(clip: Rect, src: Rect, dest: Rect, mirror: bool) -> Op {
        Op::Quad(Quad {
            clip,
            bitmap: ID,
            src,
            sampling: Sampling::Stretch { dest, mirror },
            tone: Tone::NEUTRAL,
            color: Color::new(0, 0, 0, 0),
            opacity: 255,
            blend: BlendMode::Normal,
        })
    }

    #[test]
    fn stretch_quad_record_layout() {
        let src = Rect::new(2, 1, 5, 4);
        let op = stretch(SCREEN, src, Rect::new(-3, 7, 15, 6), true);
        let records = encode_ops(&table(), SIZE, &[op]).unwrap();
        // 5 texels into 15 pixels and 4 into 6, with 20 fractional bits and rounded down.
        let (step_x, step_y) = ((5u32 << 20) / 15, (4u32 << 20) / 6);
        #[rustfmt::skip]
        let expected = [
            1, 0, 7, 12, 13,
            3, 2, 1, 5, 4,
            -3i32 as u32, 7, 0, step_x, 0, step_y,
            255, 0,
            0, 0, 0, 0, 0,
            2,
        ];
        assert_eq!(records[0].words(), &expected);

        let plain = stretch(SCREEN, src, Rect::new(-3, 7, 15, 6), false);
        let records = encode_ops(&table(), SIZE, &[plain]).unwrap();
        assert_eq!(records[0].words()[23], 1);
    }

    /// A step that does not fit 32 bits: many texels into few pixels.
    #[test]
    fn stretch_steps_are_split_into_two_words() {
        let mut table = BitmapTable::new(8192);
        let wide = BitmapId::new(0, 0);
        let delta = BitmapDelta::Create {
            id: wide,
            width: 8000,
            height: 1,
        };
        table.apply(&delta).unwrap();
        let op = Op::Quad(Quad {
            clip: SCREEN,
            bitmap: wide,
            src: Rect::new(0, 0, 8000, 1),
            sampling: Sampling::Stretch {
                dest: Rect::new(0, 0, 1, 1),
                mirror: false,
            },
            tone: Tone::NEUTRAL,
            color: Color::new(0, 0, 0, 0),
            opacity: 255,
            blend: BlendMode::Normal,
        });
        let records = encode_ops(&table, SIZE, &[op]).unwrap();
        let step = 8000u64 << 20;
        assert!(step > u64::from(u32::MAX));
        let words = records[0].words();
        assert_eq!((u64::from(words[12]) << 32) | u64::from(words[13]), step);
        assert_eq!(words[14..16], [0, 1 << 20]);
    }

    /// A stretch quad's record is clipped to its destination, wherever that lies.
    #[test]
    fn stretch_quad_is_clipped_to_its_destination() {
        let src = Rect::new(0, 0, 8, 6);
        let clip_of = |clip, dest| {
            let records = encode_ops(&table(), SIZE, &[stretch(clip, src, dest, false)]).unwrap();
            records.first().map(|record| record.clip)
        };
        let rect = |x0, y0, x1, y1| Some(ClipRect { x0, y0, x1, y1 });
        assert_eq!(clip_of(SCREEN, Rect::new(5, 6, 7, 8)), rect(5, 6, 12, 14));
        assert_eq!(
            clip_of(Rect::new(10, 10, 5, 5), Rect::new(-100, -100, 112, 300)),
            rect(10, 10, 12, 15)
        );
        assert_eq!(
            clip_of(SCREEN, Rect::new(i32::MIN, i32::MIN, u32::MAX, u32::MAX)),
            rect(0, 0, 40, 30)
        );
        // Beside the clip, beside the screen, or without pixels: no record.
        for dest in [
            Rect::new(40, 0, 10, 10),
            Rect::new(-10, 0, 10, 10),
            Rect::new(i32::MAX, i32::MAX, u32::MAX, u32::MAX),
            Rect::new(3, 3, 0, 10),
            Rect::new(3, 3, 10, 0),
        ] {
            assert_eq!(clip_of(SCREEN, dest), None, "{dest:?}");
        }
        assert_eq!(
            clip_of(Rect::new(0, 0, 5, 5), Rect::new(5, 0, 10, 10)),
            None
        );
    }

    /// The words an old quad left at zero are exactly the neutral tone and zero-alpha colour.
    #[test]
    fn neutral_quad_leaves_the_tone_and_colour_words_zero() {
        let records = encode_ops(&table(), SIZE, &[quad(ID, Rect::new(0, 0, 8, 6))]).unwrap();
        assert_eq!(records[0].words()[18..], [0; 6]);
    }

    #[test]
    fn rect_effect_record_layout() {
        let op = Op::RectEffect(RectEffect {
            clip: SCREEN,
            tone: Tone::new(-255, i16::MAX, i16::MIN, 77),
            color: Color::new(255, 0, 128, 96),
        });
        let records = encode_ops(&table(), SIZE, &[op]).unwrap();
        let mut expected = [0; RECORD_WORDS];
        expected[..5].copy_from_slice(&[2, 0, 0, 40, 30]);
        expected[18] = 0x6080_00FF;
        expected[19..23].copy_from_slice(&[-255i32 as u32, 32767, -32768i32 as u32, 77]);
        assert_eq!(records[0].words(), &expected);
    }

    #[test]
    fn records_keep_op_order_and_blend_modes_are_distinct() {
        let blends = [
            BlendMode::Normal,
            BlendMode::Add,
            BlendMode::Subtract,
            BlendMode::Window,
        ];
        let ops: Vec<Op> = blends
            .iter()
            .map(|&blend| {
                Op::Quad(Quad {
                    clip: SCREEN,
                    bitmap: ID,
                    src: Rect::new(0, 0, 8, 6),
                    sampling: Sampling::Affine(Transform::IDENTITY),
                    tone: Tone::NEUTRAL,
                    color: Color::new(0, 0, 0, 0),
                    opacity: 255,
                    blend,
                })
            })
            .collect();
        let records = encode_ops(&table(), SIZE, &ops).unwrap();
        let modes: Vec<u32> = records.iter().map(|record| record.words()[17]).collect();
        assert_eq!(modes, [0, 1, 2, 3]);
    }
}
