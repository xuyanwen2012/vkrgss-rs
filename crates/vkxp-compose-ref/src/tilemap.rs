//! The texel a tilemap layer op draws at a pixel.

use vkxp_pixel::Rgba;
use vkxp_proto::TilemapLayer;

use crate::{Bitmap, Table};

/// Side of a tile.
const TILE: i64 = 32;
/// Side of a quarter of an autotile.
const QUARTER: u32 = 16;
/// Width of one animation frame of an autotile sheet.
const FRAME: u32 = 96;
/// Number of patterns of an autotile, and the first tile number that names one.
const PATTERNS: i32 = 48;
/// The first tile number that names a tile of the tileset.
const FIRST_TILESET: i32 = 384;

/// For each autotile pattern, its top-left, top-right, bottom-left and bottom-right quarter.
/// Quarters are numbered from 1 to 48 row by row over the 6 by 8 grid of a frame.
///
/// This is the table of `core-scene`; `AUTOTILE_QUARTERS` in the compute shader of the Vulkan
/// backend and the table of `vkxp-core` are copies that tests compare with this one.
#[rustfmt::skip]
pub const AUTOTILE_QUARTERS: [[u8; 4]; 48] = [
    [27, 28, 33, 34], [ 5, 28, 33, 34], [27,  6, 33, 34], [ 5,  6, 33, 34],
    [27, 28, 33, 12], [ 5, 28, 33, 12], [27,  6, 33, 12], [ 5,  6, 33, 12],
    [27, 28, 11, 34], [ 5, 28, 11, 34], [27,  6, 11, 34], [ 5,  6, 11, 34],
    [27, 28, 11, 12], [ 5, 28, 11, 12], [27,  6, 11, 12], [ 5,  6, 11, 12],
    [25, 26, 31, 32], [25,  6, 31, 32], [25, 26, 31, 12], [25,  6, 31, 12],
    [15, 16, 21, 22], [15, 16, 21, 12], [15, 16, 11, 22], [15, 16, 11, 12],
    [29, 30, 35, 36], [29, 30, 11, 36], [ 5, 30, 35, 36], [ 5, 30, 11, 36],
    [39, 40, 45, 46], [ 5, 40, 45, 46], [39,  6, 45, 46], [ 5,  6, 45, 46],
    [25, 30, 31, 36], [15, 16, 45, 46], [13, 14, 19, 20], [13, 14, 19, 12],
    [17, 18, 23, 24], [17, 18, 11, 24], [41, 42, 47, 48], [ 5, 42, 47, 48],
    [37, 38, 43, 44], [37,  6, 43, 44], [13, 18, 19, 24], [13, 14, 43, 44],
    [37, 42, 43, 48], [17, 18, 47, 48], [13, 18, 43, 48], [ 1,  2,  7,  8],
];

/// A checked tilemap layer op with everything it reads.
pub(crate) struct Layer<'a> {
    pub op: &'a TilemapLayer,
    pub map: &'a Table,
    pub priorities: &'a Table,
    pub tileset: Option<&'a Bitmap>,
    pub autotiles: [Option<&'a Bitmap>; 7],
}

impl Layer<'_> {
    /// The texel the op blends over screen pixel (`px`, `py`), or `None` when it draws
    /// nothing there. The clip is the caller's business.
    pub(crate) fn texel(&self, px: u16, py: u16) -> Option<Rgba> {
        let (map, op) = (self.map, self.op);
        let (xsize, ysize) = (i64::from(map.xsize()), i64::from(map.ysize()));
        let layer = i64::from(op.layer);
        if xsize == 0 || ysize == 0 || layer >= i64::from(map.zsize()) {
            return None;
        }
        let x = i64::from(px) + i64::from(op.origin.0);
        let y = i64::from(py) + i64::from(op.origin.1);
        let column = x.div_euclid(TILE).rem_euclid(xsize);
        let row = y.div_euclid(TILE).rem_euclid(ysize);
        // Inside the table, whose length the store checked.
        let number = i32::from(map.data()[((layer * ysize + row) * xsize + column) as usize]);

        let priorities = self.priorities;
        let entry = usize::try_from(number)
            .ok()
            .filter(|&index| index < priorities.xsize() as usize)
            .and_then(|index| priorities.data().get(index));
        if i32::from(entry.copied().unwrap_or(0)) != i32::from(op.priority) {
            return None;
        }

        // Both are from 0 to 31.
        let (tx, ty) = (x.rem_euclid(TILE) as u32, y.rem_euclid(TILE) as u32);
        let (bitmap, u, v) = if number >= FIRST_TILESET {
            let index = (number - FIRST_TILESET) as u32;
            (self.tileset?, index % 8 * 32 + tx, index / 8 * 32 + ty)
        } else if number >= PATTERNS {
            // Numbers 48 to 383 give 0 to 6.
            let bitmap = self.autotiles[(number / PATTERNS - 1) as usize]?;
            let frame = |width: u32| {
                (op.animation_step % u64::from((bitmap.width() / width).max(1))) as u32
            };
            if bitmap.height() <= 32 {
                // A sheet of whole tiles.
                (bitmap, frame(32) * 32 + tx, ty)
            } else {
                let quarters = AUTOTILE_QUARTERS[(number % PATTERNS) as usize];
                let quarter = u32::from(quarters[(ty / QUARTER * 2 + tx / QUARTER) as usize]) - 1;
                (
                    bitmap,
                    frame(FRAME) * FRAME + quarter % 6 * QUARTER + tx % QUARTER,
                    quarter / 6 * QUARTER + ty % QUARTER,
                )
            }
        } else {
            return None;
        };
        (u < bitmap.width() && v < bitmap.height()).then(|| bitmap.pixel(u, v))
    }
}
