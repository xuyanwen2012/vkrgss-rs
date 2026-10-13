//! What a tilemap plane op of RGSS2 makes of a pixel.

use vkxp_pixel::Rgba;
use vkxp_proto::tiles_rgss2::{self as tiles, OVERHANG, QUARTER_SIZE, SHEETS, TILE_SIZE, Tile};
use vkxp_proto::{Plane, Size, TilemapPlane};

use crate::{Bitmap, Table};

/// A checked tilemap plane op with everything it reads.
pub(crate) struct PlaneStep<'a> {
    pub op: &'a TilemapPlane,
    pub map: &'a Table,
    pub passages: Option<&'a Table>,
    pub flash: Option<&'a Table>,
    pub bitmaps: [Option<&'a Bitmap>; SHEETS],
}

impl PlaneStep<'_> {
    /// The value of screen pixel (`px`, `py`) of a frame of `frame` pixels after the op, given
    /// its value before, in the order the op's documentation gives. The clip is the caller's
    /// business.
    pub(crate) fn apply(&self, mut pixel: Rgba, frame: Size, px: u16, py: u16) -> Rgba {
        let op = self.op;
        let (xsize, ysize) = (u64::from(self.map.xsize()), u64::from(self.map.ysize()));
        let x = i64::from(px) - i64::from(op.corner.0);
        let y = i64::from(py) - i64::from(op.corner.1);
        let inside = (0..i64::from(tiles::PLANE_WIDTH)).contains(&x)
            && (0..i64::from(tiles::PLANE_HEIGHT)).contains(&y);
        if !inside || xsize == 0 || ysize == 0 {
            return pixel;
        }
        // Both are inside the plane.
        let (x, y) = (x as u32, y as u32);
        let (column, row) = (u64::from(x / TILE_SIZE), u64::from(y / TILE_SIZE));
        let (tx, ty) = (x % TILE_SIZE, y % TILE_SIZE);
        // The cell of the map, inside it.
        let cell_x = ((u64::from(op.first_cell.0) + column) % xsize) as i64;
        let cell_y = ((u64::from(op.first_cell.1) + row) % ysize) as i64;
        // The screen column of the cell's left edge.
        let left = i64::from(px) - i64::from(tx);
        let upper = op.plane == Plane::Upper;
        let layers = if upper { 2..3 } else { 0..3 };

        let top = i64::from(py) - i64::from(ty);
        if ty < OVERHANG && row > 0 && i64::from(py) / 8 * 8 <= top {
            let above = ((u64::from(op.first_cell.1) + row - 1) % ysize) as i64;
            for layer in layers.clone() {
                let tile = self.drawn(self.number(cell_x, above, layer));
                if let Some(tile) = tile
                    && let Some(texel) = tile.overhang(tx, ty)
                {
                    pixel = self.blend(pixel, &tile, texel, frame, left, tx, px);
                }
            }
        }
        for layer in layers {
            let number = self.number(cell_x, cell_y, layer);
            if let Some(tile) = self.drawn(number) {
                for texel in tile.texels(tx, ty).into_iter().flatten() {
                    pixel = self.blend(pixel, &tile, texel, frame, left, tx, px);
                }
            }
            if layer != 0 || upper {
                continue;
            }
            if tx < QUARTER_SIZE
                && tiles::takes_shadow(number)
                && tiles::is_wall(self.number(cell_x - 1, cell_y, 0))
                && tiles::is_wall(self.number(cell_x - 1, cell_y - 1, 0))
            {
                for channel in &mut pixel[..3] {
                    *channel = tiles::shadow(*channel);
                }
            }
            if ty < OVERHANG && !tiles::is_wall(number) && !tiles::is_table(number) {
                let above = self.number(cell_x, cell_y - 1, 0);
                if let Some(tile) = tiles::tile(above, op.animation_step)
                    && let Some(texel) = tile.edge(tx, ty)
                {
                    pixel = self.blend(pixel, &tile, texel, frame, left, tx, px);
                }
            }
        }

        if let Some(flash) = self.flash
            && !flash.data().is_empty()
        {
            let (xsize, ysize) = (u64::from(flash.xsize()), u64::from(flash.ysize()));
            let flash_x = (u64::from(op.first_cell.0) + column) % xsize;
            let flash_y = (u64::from(op.first_cell.1) + row) % ysize;
            // Inside the first layer of the table, whose length the store checked.
            let entry = flash.data()[(flash_y * xsize + flash_x) as usize];
            if let Some(amounts) = tiles::flash_amounts(entry, op.flash_level) {
                for (channel, amount) in pixel.iter_mut().zip(amounts) {
                    *channel = (u16::from(*channel) + amount).min(255) as u8;
                }
            }
        }
        pixel
    }

    /// The tile number at cell (`x`, `y`) of layer `layer` of the map, or 0, which is no tile,
    /// outside the map and for a layer the map does not have.
    fn number(&self, x: i64, y: i64, layer: u32) -> i16 {
        let map = self.map;
        let (xsize, ysize) = (i64::from(map.xsize()), i64::from(map.ysize()));
        if !(0..xsize).contains(&x) || !(0..ysize).contains(&y) || layer >= map.zsize() {
            return 0;
        }
        // Inside the table, whose length the store checked.
        map.data()[((i64::from(layer) * ysize + y) * xsize + x) as usize]
    }

    /// Tile `number` when this plane draws it.
    fn drawn(&self, number: i16) -> Option<Tile> {
        let entry = self.passages.and_then(|passages| {
            let index = usize::try_from(number).ok()?;
            (index < passages.xsize() as usize).then(|| passages.data().get(index))?
        });
        let star = entry.copied().unwrap_or(0) & tiles::STAR != 0;
        if star != (self.op.plane == Plane::Upper) {
            return None;
        }
        tiles::tile(number, self.op.animation_step)
    }

    /// `pixel` with the texel `texel` of the tile's sheet blended over it, for the screen
    /// pixel column `px`, which is column `tx` of a cell whose left edge is the screen column
    /// `left`.
    #[expect(clippy::too_many_arguments, reason = "one pixel of one transfer")]
    fn blend(
        &self,
        pixel: Rgba,
        tile: &Tile,
        texel: (u32, u32),
        frame: Size,
        left: i64,
        tx: u32,
        px: u16,
    ) -> Rgba {
        let Some(bitmap) = self.bitmaps[tile.sheet] else {
            return pixel;
        };
        if texel.0 >= bitmap.width() || texel.1 >= bitmap.height() {
            return pixel;
        }
        let texel = bitmap.pixel(texel.0, texel.1);
        let span = tile.span();
        let row_left = left + i64::from(tx / span * span);
        if self.op.is_tail(frame, row_left, span, px) {
            vkxp_pixel::blend_normal(pixel, texel, 255)
        } else {
            vkxp_pixel::blend_window(pixel, texel, 255)
        }
    }
}
