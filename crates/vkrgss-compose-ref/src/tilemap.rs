//! The texel a tilemap layer op draws at a pixel.

use vkrgss_pixel::Rgba;
use vkrgss_proto::tiles::{self, Sheet, TILE_SIZE};
use vkrgss_proto::{Size, TilemapLayer};

use crate::{Bitmap, Table};

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
        let tile = i64::from(TILE_SIZE);
        let column = x.div_euclid(tile).rem_euclid(xsize);
        let row = y.div_euclid(tile).rem_euclid(ysize);
        // Inside the table, whose length the store checked.
        let number = map.data()[((layer * ysize + row) * xsize + column) as usize];

        let priorities = self.priorities;
        let entry = usize::try_from(number)
            .ok()
            .filter(|&index| index < priorities.xsize() as usize)
            .and_then(|index| priorities.data().get(index));
        if i32::from(entry.copied().unwrap_or(0)) != i32::from(op.priority) {
            return None;
        }

        let bitmap = match tiles::sheet(number)? {
            Sheet::Tileset => self.tileset?,
            Sheet::Autotile(index) => self.autotiles[index]?,
        };
        // Both are from 0 to 31.
        let (tx, ty) = (x.rem_euclid(tile) as u32, y.rem_euclid(tile) as u32);
        let size = Size::new(bitmap.width(), bitmap.height());
        let (u, v) = tiles::texel(number, tx, ty, size, op.animation_step)?;
        Some(bitmap.pixel(u, v))
    }
}
