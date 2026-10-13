//! Tile numbering of RGSS2: which texel of which of the nine sheets a tile number of a
//! tilemap of RGSS2 shows, the shapes of its autotiles, and the rules of a tile's neighbours.
//!
//! This is the one Rust definition of these rules, as [`crate::tiles`] is for RGSS1. The
//! reference compositor evaluates a [`crate::TilemapPlane`] with it; the compute shader of the
//! Vulkan backend has a copy, whose tables a test compares with [`FLOOR_SHAPES`],
//! [`WALL_SHAPES`] and [`FALL_SHAPES`].
//!
//! Everything here was measured on the original runtime of VX (`RGSS202E.dll`, versions
//! 2.0.0.1 and 2.0.2.1 alike). Where the original reads outside its arrays, numbers 1024 to
//! 1535 and negative numbers, a tile draws nothing here.
//!
//! The sheets are the slots of `Tilemap#bitmaps`: 0 to 3 hold autotiles (A1 to A4), 4 whole
//! tiles (A5), 5 to 8 whole tiles (B to E). An autotile is drawn as four quarters of 16 by 16
//! pixels, each a piece of the autotile's block that a shape table names.

/// Side of a tile.
pub const TILE_SIZE: u32 = 32;
/// Side of a quarter of an autotile.
pub const QUARTER_SIZE: u32 = 16;
/// Number of sheets of a tilemap.
pub const SHEETS: usize = 9;
/// Cells in a row of a plane.
pub const PLANE_COLUMNS: u32 = 21;
/// Rows of cells of a plane.
pub const PLANE_ROWS: u32 = 16;
/// Width of a plane in pixels.
pub const PLANE_WIDTH: u32 = PLANE_COLUMNS * TILE_SIZE;
/// Height of a plane in pixels.
pub const PLANE_HEIGHT: u32 = PLANE_ROWS * TILE_SIZE;
/// The bit of an entry of the passages table that sends a tile of the third layer to the
/// upper plane.
pub const STAR: i16 = 0x10;
/// Number of shapes a tile number of an autotile can name.
pub const SHAPES: i16 = 48;
/// The first tile number of an autotile.
pub const FIRST_AUTOTILE: i16 = 2048;
/// Rows of a table tile's lower pieces that are drawn below its cell.
pub const OVERHANG: u32 = 8;
/// Calls of `Tilemap#update` per animation step.
pub const UPDATES_PER_STEP: u64 = 30;

/// The sheet of the whole tiles A5.
const SHEET_A5: usize = 4;
/// The sheet of the whole tiles B; C, D and E follow.
const SHEET_B: usize = 5;
/// The first number of A5.
const FIRST_A5: i16 = 1536;
/// The first number of A3 and one past the last of A4 as far as walls give shadows.
const FIRST_WALL: i16 = 4352;
const END_WALL: i16 = 8192;
/// The first number of A2.
const FIRST_A2: i16 = 2816;

/// For each of the 48 shapes of a floor autotile, the corner of the piece of its top-left,
/// top-right, bottom-left and bottom-right quarter inside the autotile's block of 64 by 96
/// pixels.
#[rustfmt::skip]
pub const FLOOR_SHAPES: [[(u8, u8); 4]; 48] = [
    [(32, 64), (16, 64), (32, 48), (16, 48)], [(32,  0), (16, 64), (32, 48), (16, 48)],
    [(32, 64), (48,  0), (32, 48), (16, 48)], [(32,  0), (48,  0), (32, 48), (16, 48)],
    [(32, 64), (16, 64), (32, 48), (48, 16)], [(32,  0), (16, 64), (32, 48), (48, 16)],
    [(32, 64), (48,  0), (32, 48), (48, 16)], [(32,  0), (48,  0), (32, 48), (48, 16)],
    [(32, 64), (16, 64), (32, 16), (16, 48)], [(32,  0), (16, 64), (32, 16), (16, 48)],
    [(32, 64), (48,  0), (32, 16), (16, 48)], [(32,  0), (48,  0), (32, 16), (16, 48)],
    [(32, 64), (16, 64), (32, 16), (48, 16)], [(32,  0), (16, 64), (32, 16), (48, 16)],
    [(32, 64), (48,  0), (32, 16), (48, 16)], [(32,  0), (48,  0), (32, 16), (48, 16)],
    [( 0, 64), (16, 64), ( 0, 48), (16, 48)], [( 0, 64), (48,  0), ( 0, 48), (16, 48)],
    [( 0, 64), (16, 64), ( 0, 48), (48, 16)], [( 0, 64), (48,  0), ( 0, 48), (48, 16)],
    [(32, 32), (16, 32), (32, 48), (16, 48)], [(32, 32), (16, 32), (32, 48), (48, 16)],
    [(32, 32), (16, 32), (32, 16), (16, 48)], [(32, 32), (16, 32), (32, 16), (48, 16)],
    [(32, 64), (48, 64), (32, 48), (48, 48)], [(32, 64), (48, 64), (32, 16), (48, 48)],
    [(32,  0), (48, 64), (32, 48), (48, 48)], [(32,  0), (48, 64), (32, 16), (48, 48)],
    [(32, 64), (16, 64), (32, 80), (16, 80)], [(32,  0), (16, 64), (32, 80), (16, 80)],
    [(32, 64), (48,  0), (32, 80), (16, 80)], [(32,  0), (48,  0), (32, 80), (16, 80)],
    [( 0, 64), (48, 64), ( 0, 48), (48, 48)], [(32, 32), (16, 32), (32, 80), (16, 80)],
    [( 0, 32), (16, 32), ( 0, 48), (16, 48)], [( 0, 32), (16, 32), ( 0, 48), (48, 16)],
    [(32, 32), (48, 32), (32, 48), (48, 48)], [(32, 32), (48, 32), (32, 16), (48, 48)],
    [(32, 64), (48, 64), (32, 80), (48, 80)], [(32,  0), (48, 64), (32, 80), (48, 80)],
    [( 0, 64), (16, 64), ( 0, 80), (16, 80)], [( 0, 64), (48,  0), ( 0, 80), (16, 80)],
    [( 0, 32), (48, 32), ( 0, 48), (48, 48)], [( 0, 32), (16, 32), ( 0, 80), (16, 80)],
    [( 0, 64), (48, 64), ( 0, 80), (48, 80)], [(32, 32), (48, 32), (32, 80), (48, 80)],
    [( 0, 32), (48, 32), ( 0, 80), (48, 80)], [( 0,  0), (16,  0), ( 0, 16), (16, 16)],
];

/// The same for the 16 shapes of a wall autotile, whose block is 64 by 64 pixels. Bit 1 of
/// a shape is an edge on the left, 2 on top, 4 on the right, 8 at the bottom. A shape from 16
/// takes the piece at (0, 0) four times.
#[rustfmt::skip]
pub const WALL_SHAPES: [[(u8, u8); 4]; 16] = [
    [(32, 32), (16, 32), (32, 16), (16, 16)], [( 0, 32), (16, 32), ( 0, 16), (16, 16)],
    [(32,  0), (16,  0), (32, 16), (16, 16)], [( 0,  0), (16,  0), ( 0, 16), (16, 16)],
    [(32, 32), (48, 32), (32, 16), (48, 16)], [( 0, 32), (48, 32), ( 0, 16), (48, 16)],
    [(32,  0), (48,  0), (32, 16), (48, 16)], [( 0,  0), (48,  0), ( 0, 16), (48, 16)],
    [(32, 32), (16, 32), (32, 48), (16, 48)], [( 0, 32), (16, 32), ( 0, 48), (16, 48)],
    [(32,  0), (16,  0), (32, 48), (16, 48)], [( 0,  0), (16,  0), ( 0, 48), (16, 48)],
    [(32, 32), (48, 32), (32, 48), (48, 48)], [( 0, 32), (48, 32), ( 0, 48), (48, 48)],
    [(32,  0), (48,  0), (32, 48), (48, 48)], [( 0,  0), (48,  0), ( 0, 48), (48, 48)],
];

/// The same for the 4 shapes of a waterfall autotile, whose block is 64 by 32 pixels per
/// animation frame. Bit 1 of a shape is an edge on the left, 2 on the right. A shape from 4
/// takes the piece at (0, 0) four times.
#[rustfmt::skip]
pub const FALL_SHAPES: [[(u8, u8); 4]; 4] = [
    [(32,  0), (16,  0), (32, 16), (16, 16)], [( 0,  0), (16,  0), ( 0, 16), (16, 16)],
    [(32,  0), (48,  0), (32, 16), (48, 16)], [( 0,  0), (48,  0), ( 0, 16), (48, 16)],
];

/// The block of an animated floor autotile of A1 moves right by 64 pixels times this, by the
/// animation step modulo 4.
pub const FLOOR_FRAMES: [u32; 4] = [0, 1, 2, 1];

/// The flash level rises and falls by this over 32 updates, four updates a value.
const FLASH_PULSE: [u8; 8] = [0, 1, 2, 3, 4, 3, 2, 1];

/// The animation step of a tilemap that was updated `updates` times.
pub const fn animation_step(updates: u64) -> u64 {
    updates / UPDATES_PER_STEP
}

/// The flash level of a tilemap that was updated `updates` times: 4 to 8.
pub const fn flash_level(updates: u64) -> u8 {
    4 + FLASH_PULSE[(updates / 4 % 8) as usize]
}

/// What a plane adds to the red, green and blue of every pixel of a cell whose entry of the
/// flash table is `entry`, at the flash level `level`, or `None` for an entry that does not
/// flash, which is one not above 0. The entry holds a nibble for each channel; its bits 12
/// to 14 are ignored. A channel is limited to 255.
pub const fn flash_amounts(entry: i16, level: u8) -> Option<[u16; 3]> {
    if entry <= 0 {
        return None;
    }
    let (entry, level) = (entry as u16, level as u16);
    Some([
        (entry >> 8 & 15) * level,
        (entry >> 4 & 15) * level,
        (entry & 15) * level,
    ])
}

/// The value of a colour channel under a shadow.
pub const fn shadow(value: u8) -> u8 {
    ((value as u16 * 127) >> 8) as u8
}

/// Whether tile `number` is a wall, a tile of A3 or A4 up to 8191: such tiles give shadows
/// and take no edge of a table.
pub const fn is_wall(number: i16) -> bool {
    number >= FIRST_WALL && number < END_WALL
}

/// Whether tile `number` is a table: an autotile of the last column of A2.
pub const fn is_table(number: i16) -> bool {
    number >= FIRST_A2 && number < FIRST_WALL && (number - FIRST_AUTOTILE) / SHAPES % 8 == 7
}

/// Whether tile `number` in the first layer takes a shadow: a tile of A5 from its 17th on,
/// or one of A2 that is no table.
pub const fn takes_shadow(number: i16) -> bool {
    (number >= FIRST_A5 + 16 && number < FIRST_AUTOTILE)
        || (number >= FIRST_A2 && number < FIRST_WALL && !is_table(number))
}

/// Which table gives the pieces of a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shapes {
    /// No pieces: the block is the tile.
    Whole,
    /// [`FLOOR_SHAPES`].
    Floor,
    /// [`WALL_SHAPES`].
    Wall,
    /// [`FALL_SHAPES`].
    Fall,
}

/// A tile number resolved: where its pixels come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile {
    /// The slot of the sheet, from 0 to 8.
    pub sheet: usize,
    /// The corner of the tile's block in the sheet, at its animation frame.
    pub block: (u32, u32),
    pub shapes: Shapes,
    /// The shape, from 0 to 47; 0 for a whole tile.
    pub shape: u8,
    /// Whether the tile is a table, see [`is_table`].
    pub table: bool,
}

/// Tile `number` at the animation step `animation_step`, or `None` for a number that draws
/// nothing: 0, a negative number and 1024 to 1535.
///
/// Numbers 1 to 1023 name the whole tiles of the sheets B to E, 256 each, and 1536 to 2047
/// those of A5 by the same formula: 16 tiles in a row, the first 128 numbers of a sheet in
/// its left half. From 2048 a number names shape `(number - 2048) mod 48` of the autotile
/// `k = (number - 2048) div 48`: 0 to 15 in A1, 16 to 47 in A2, 48 to 79 in A3, from 80 in
/// A4, whose rows of autotiles alternate between floors and walls and go on below the sheet.
/// Of A1, the autotiles 0, 1 and the even ones from 4 are animated floors, 2 and 3 are still
/// floors, and the odd ones from 5 are waterfalls.
pub const fn tile(number: i16, animation_step: u64) -> Option<Tile> {
    if number <= 0 {
        return None;
    }
    if number < FIRST_AUTOTILE {
        let sheet = if number >= FIRST_A5 {
            SHEET_A5
        } else if number < 1024 {
            SHEET_B + number as usize / 256
        } else {
            return None;
        };
        let number = number as u32;
        return Some(Tile {
            sheet,
            block: (
                (number % 8 + number / 128 % 2 * 8) * TILE_SIZE,
                number % 256 / 8 % 16 * TILE_SIZE,
            ),
            shapes: Shapes::Whole,
            shape: 0,
            table: false,
        });
    }
    let index = (number - FIRST_AUTOTILE) as u32;
    let (kind, shape) = (index / SHAPES as u32, (index % SHAPES as u32) as u8);
    let (column, row) = (kind % 8, kind / 8);
    let (sheet, block, shapes) = if row < 2 {
        let fall = kind >= 4 && kind % 2 == 1;
        let (mut x, mut y) = if kind < 4 {
            (kind / 2 * 192, kind % 2 * 96)
        } else {
            (
                column / 4 * 256 + if fall { 192 } else { 0 },
                (column / 2 % 2 + row * 2) * 96,
            )
        };
        if fall {
            y += 32 * (animation_step % 3) as u32;
        } else if kind != 2 && kind != 3 {
            x += 64 * FLOOR_FRAMES[(animation_step % 4) as usize];
        }
        (0, (x, y), if fall { Shapes::Fall } else { Shapes::Floor })
    } else if row < 6 {
        (1, (column * 64, (row - 2) * 96), Shapes::Floor)
    } else if row < 10 {
        (2, (column * 64, (row - 6) * 64), Shapes::Wall)
    } else if row % 2 == 0 {
        (3, (column * 64, (row - 10) * 80), Shapes::Floor)
    } else {
        (3, (column * 64, (row - 10) * 80 + 16), Shapes::Wall)
    };
    Some(Tile {
        sheet,
        block,
        shapes,
        shape,
        table: is_table(number),
    })
}

impl Tile {
    /// The corner, inside the block, of the piece of quarter `quarter`, counted from 0 as the
    /// shape tables count them.
    const fn piece(&self, quarter: usize) -> (u32, u32) {
        let shape = self.shape as usize;
        let (x, y) = match self.shapes {
            Shapes::Whole => (0, 0),
            Shapes::Floor => FLOOR_SHAPES[shape][quarter],
            Shapes::Wall if shape < WALL_SHAPES.len() => WALL_SHAPES[shape][quarter],
            Shapes::Fall if shape < FALL_SHAPES.len() => FALL_SHAPES[shape][quarter],
            Shapes::Wall | Shapes::Fall => (0, 0),
        };
        (x as u32, y as u32)
    }

    /// Whether the piece at row `y` of the block, of a lower quarter, is one that a table
    /// draws 8 pixels lower: the lower edge and the inner corner.
    const fn sinks(&self, y: u32) -> bool {
        self.table && (y == 16 || y == 80)
    }

    /// Width of a row of one transfer of the tile: the tile for a whole tile, a quarter for an
    /// autotile. [`crate::TilemapPlane::is_tail`] counts the pixels of such a row.
    pub const fn span(&self) -> u32 {
        match self.shapes {
            Shapes::Whole => TILE_SIZE,
            _ => QUARTER_SIZE,
        }
    }

    /// The texels of the sheet that the tile blends, in this order, over pixel (`x`, `y`) of
    /// its cell, both from 0 to 31. A table blends two where it fills the gap above a piece
    /// drawn 8 pixels lower: first the filling, the piece 48 pixels down the block in the
    /// column of the sunk piece, then that piece from its 9th row on.
    pub const fn texels(&self, x: u32, y: u32) -> [Option<(u32, u32)>; 2] {
        let (bx, by) = self.block;
        if matches!(self.shapes, Shapes::Whole) {
            return [Some((bx + x, by + y)), None];
        }
        let (qx, qy) = (x % QUARTER_SIZE, y % QUARTER_SIZE);
        let (px, py) = self.piece((y / QUARTER_SIZE * 2 + x / QUARTER_SIZE) as usize);
        if y < QUARTER_SIZE || !self.sinks(py) {
            return [Some((bx + px + qx, by + py + qy)), None];
        }
        let filling = if py == 16 {
            [0, 48, 32, 16][(px / QUARTER_SIZE) as usize]
        } else {
            px
        };
        let sunk = if qy >= OVERHANG {
            Some((bx + px + qx, by + py + qy - OVERHANG))
        } else {
            None
        };
        [Some((bx + filling + qx, by + 48 + qy)), sunk]
    }

    /// The texel the tile blends over pixel (`x`, `y`) of the cell below its own, `x` from 0
    /// to 31 and `y` from 0 to 7: the last rows of a piece that a table draws 8 pixels lower.
    /// `None` for a tile that draws nothing there.
    pub const fn overhang(&self, x: u32, y: u32) -> Option<(u32, u32)> {
        if matches!(self.shapes, Shapes::Whole) {
            return None;
        }
        let (px, py) = self.piece((2 + x / QUARTER_SIZE) as usize);
        if !self.sinks(py) {
            return None;
        }
        let (bx, by) = self.block;
        Some((
            bx + px + x % QUARTER_SIZE,
            by + py + QUARTER_SIZE - OVERHANG + y,
        ))
    }

    /// The texel of the edge that a table whose shape has the lower edge shows on the first
    /// layer of the cell below, for pixel (`x`, `y`) of that cell, `x` from 0 to 31 and `y`
    /// from 0 to 7: the rows 88 to 95 of the table's block. `None` for a tile that is no
    /// table or has no lower edge.
    pub const fn edge(&self, x: u32, y: u32) -> Option<(u32, u32)> {
        if !self.table || self.piece(2).1 != 80 {
            return None;
        }
        let (bx, by) = self.block;
        let (px, _) = self.piece((2 + x / QUARTER_SIZE) as usize);
        Some((bx + px + x % QUARTER_SIZE, by + 88 + y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn whole(number: i16) -> (usize, (u32, u32)) {
        let tile = tile(number, 0).unwrap();
        assert_eq!((tile.shapes, tile.span()), (Shapes::Whole, 32), "{number}");
        (tile.sheet, tile.block)
    }

    fn corners(number: i16, step: u64) -> [(u32, u32); 4] {
        let tile = tile(number, step).unwrap();
        [(0, 0), (16, 0), (0, 16), (16, 16)].map(|(x, y)| tile.texels(x, y)[0].unwrap())
    }

    #[test]
    fn whole_tiles_come_from_the_sheets_b_to_e_and_a5() {
        assert_eq!(whole(1), (5, (32, 0)));
        assert_eq!(whole(8), (5, (0, 32)));
        assert_eq!(whole(127), (5, (224, 480)));
        assert_eq!(whole(128), (5, (256, 0)));
        assert_eq!(whole(255), (5, (480, 480)));
        assert_eq!(whole(256), (6, (0, 0)));
        assert_eq!(whole(1023), (8, (480, 480)));
        assert_eq!(whole(1536), (4, (0, 0)));
        assert_eq!(whole(1536 + 9), (4, (32, 32)));
        assert_eq!(whole(1664), (4, (256, 0)), "the right half of a wide A5");
        assert_eq!(whole(1792), (4, (0, 0)), "the numbers repeat");
        assert_eq!(whole(2047), (4, (480, 480)));
        let tile = tile(37, 9).unwrap();
        assert_eq!(tile.texels(5, 6), [Some((165, 134)), None]);
        assert_eq!(tile.overhang(5, 6), None);
        assert_eq!(tile.edge(5, 6), None);
    }

    #[test]
    fn undefined_numbers_draw_nothing() {
        for number in [0, -1, -256, -1280, i16::MIN, 1024, 1100, 1535] {
            assert_eq!(tile(number, 0), None, "{number}");
        }
    }

    #[test]
    fn autotiles_of_a1_have_their_measured_blocks() {
        let blocks = [
            (0, 0),
            (0, 96),
            (192, 0),
            (192, 96),
            (256, 0),
            (448, 0),
            (256, 96),
            (448, 96),
            (0, 192),
            (192, 192),
            (0, 288),
            (192, 288),
            (256, 192),
            (448, 192),
            (256, 288),
            (448, 288),
        ];
        for (kind, block) in blocks.into_iter().enumerate() {
            let found = tile(2048 + 48 * kind as i16, 0).unwrap();
            assert_eq!((found.sheet, found.block), (0, block), "autotile {kind}");
            let fall = kind >= 5 && kind % 2 == 1;
            let shapes = if fall { Shapes::Fall } else { Shapes::Floor };
            assert_eq!(found.shapes, shapes, "autotile {kind}");
        }
    }

    /// Scenario "Animation, measured on the original".
    #[test]
    fn a1_moves_by_the_animation_step() {
        let block = |kind: i16, step| tile(2048 + 48 * kind, step).unwrap().block;
        for (step, (floor, fall)) in [(0, 0), (64, 32), (128, 64), (64, 0), (0, 32), (64, 64)]
            .into_iter()
            .enumerate()
        {
            let step = step as u64;
            for kind in [0, 1, 4, 6, 8, 14] {
                let still = block(kind, 0);
                assert_eq!(block(kind, step), (still.0 + floor, still.1), "{kind}");
            }
            for kind in [5, 7, 9, 15] {
                let still = block(kind, 0);
                assert_eq!(block(kind, step), (still.0, still.1 + fall), "{kind}");
            }
            for kind in [2, 3] {
                assert_eq!(block(kind, step), block(kind, 0), "{kind}");
            }
            // Nothing else is animated.
            for number in [2816, 4352, 5888, 37, 1536] {
                assert_eq!(tile(number, step), tile(number, 0), "{number}");
            }
        }
    }

    #[test]
    fn autotiles_of_a2_a3_and_a4_have_their_blocks() {
        let found = |number| {
            let tile = tile(number, 3).unwrap();
            (tile.sheet, tile.block, tile.shapes)
        };
        assert_eq!(found(2816), (1, (0, 0), Shapes::Floor));
        assert_eq!(found(2816 + 48 * 9 + 5), (1, (64, 96), Shapes::Floor));
        assert_eq!(found(4351), (1, (448, 288), Shapes::Floor));
        assert_eq!(found(4352), (2, (0, 0), Shapes::Wall));
        assert_eq!(found(5887), (2, (448, 192), Shapes::Wall));
        assert_eq!(found(5888), (3, (0, 0), Shapes::Floor));
        assert_eq!(found(5888 + 48 * 8), (3, (0, 96), Shapes::Wall));
        assert_eq!(found(5888 + 48 * 16), (3, (0, 160), Shapes::Floor));
        assert_eq!(found(8191), (3, (448, 416), Shapes::Wall));
        // From 8192 the formula goes on below a sheet of the usual height.
        assert_eq!(found(8192), (3, (0, 480), Shapes::Floor));
        assert_eq!(found(i16::MAX), (3, (448, 5536), Shapes::Wall));
    }

    #[test]
    fn shapes_take_their_pieces_from_the_tables() {
        assert_eq!(
            corners(2816 + 48 * 9, 0),
            [(96, 160), (80, 160), (96, 144), (80, 144)]
        );
        assert_eq!(corners(2816 + 47, 0), [(0, 0), (16, 0), (0, 16), (16, 16)]);
        assert_eq!(corners(4352 + 15, 0), [(0, 0), (48, 0), (0, 48), (48, 48)]);
        assert_eq!(
            corners(2048 + 48 * 5 + 3, 0),
            [(448, 0), (496, 0), (448, 16), (496, 16)]
        );
        let tile = tile(2816 + 20, 0).unwrap();
        assert_eq!(tile.span(), 16);
        assert_eq!(tile.texels(21, 30), [Some((16 + 5, 48 + 14)), None]);
    }

    #[test]
    fn padded_shapes_take_the_first_piece_four_times() {
        for number in [4352 + 16, 4352 + 47, 5888 + 48 * 8 + 20, 2048 + 48 * 5 + 4] {
            let block = tile(number, 0).unwrap().block;
            assert_eq!(corners(number, 0), [block; 4], "{number}");
        }
    }

    #[test]
    fn tables_walls_and_shadow_targets_are_told_apart() {
        for kind in 16..48 {
            let number = 2048 + 48 * kind;
            assert_eq!(is_table(number), kind % 8 == 7, "{kind}");
            assert_eq!(is_table(number + 47), kind % 8 == 7, "{kind}");
            assert_eq!(takes_shadow(number), kind % 8 != 7, "{kind}");
            assert_eq!(tile(number, 0).unwrap().table, kind % 8 == 7, "{kind}");
        }
        // A column 7 outside A2 is no table.
        for number in [2048 + 48 * 7, 4352 + 48 * 7, 5888 + 48 * 7] {
            assert!(!is_table(number), "{number}");
            assert!(!tile(number, 0).unwrap().table, "{number}");
        }
        let walls: Vec<i16> = [4351, 4352, 5888, 8191, 8192, 0, -1, 37]
            .into_iter()
            .filter(|&number| is_wall(number))
            .collect();
        assert_eq!(walls, [4352, 5888, 8191]);
        let shaded: Vec<i16> = [1536, 1551, 1552, 2047, 2048, 2815, 2816, 4351, 4352, 37, 0]
            .into_iter()
            .filter(|&number| takes_shadow(number))
            .collect();
        assert_eq!(shaded, [1552, 2047, 2816]);
    }

    /// No upper quarter of a floor shape is a piece that a table sinks, so only the lower
    /// half of a table tile has a filling and reaches below the cell.
    #[test]
    fn only_lower_quarters_of_a_floor_shape_sink() {
        for (shape, pieces) in FLOOR_SHAPES.iter().enumerate() {
            for piece in &pieces[..2] {
                assert!(piece.1 != 16 && piece.1 != 80, "shape {shape}");
            }
            // A lower edge is on both lower quarters or on neither.
            assert_eq!(pieces[2].1 == 80, pieces[3].1 == 80, "shape {shape}");
        }
    }

    #[test]
    fn a_table_fills_the_gap_and_draws_its_lower_pieces_8_pixels_lower() {
        // The first table of A2, shape 28: both lower pieces are the lower edge.
        let table = tile(2816 + 48 * 7 + 28, 0).unwrap();
        assert!(table.table);
        let block = (448, 0);
        assert_eq!(table.block, block);
        // The upper half is as of any floor.
        assert_eq!(table.texels(3, 4), [Some((448 + 32 + 3, 64 + 4)), None]);
        // Rows 16 to 23: the filling alone, from 48 pixels down the block.
        assert_eq!(table.texels(3, 16), [Some((448 + 32 + 3, 48)), None]);
        assert_eq!(table.texels(20, 23), [Some((448 + 16 + 4, 55)), None]);
        // Rows 24 to 31: the filling, then the piece from its first row.
        assert_eq!(
            table.texels(3, 24),
            [Some((448 + 32 + 3, 56)), Some((448 + 32 + 3, 80))]
        );
        assert_eq!(
            table.texels(20, 31),
            [Some((448 + 16 + 4, 63)), Some((448 + 16 + 4, 87))]
        );
        // Below the cell: the last 8 rows of the piece.
        assert_eq!(table.overhang(3, 0), Some((448 + 32 + 3, 88)));
        assert_eq!(table.overhang(20, 7), Some((448 + 16 + 4, 95)));
        assert_eq!(table.edge(3, 0), Some((448 + 32 + 3, 88)));
        assert_eq!(table.edge(20, 7), Some((448 + 16 + 4, 95)));

        // Shape 4: the lower right quarter is an inner corner, the lower left is not.
        let corner = tile(2816 + 48 * 7 + 4, 0).unwrap();
        assert_eq!(corner.texels(3, 28), [Some((448 + 32 + 3, 60)), None]);
        assert_eq!(
            corner.texels(19, 28),
            [Some((448 + 16 + 3, 60)), Some((448 + 48 + 3, 20))],
            "the filling of the piece at x 48 is the one at x 16"
        );
        assert_eq!(corner.overhang(3, 2), None);
        assert_eq!(corner.overhang(19, 2), Some((448 + 48 + 3, 26)));
        assert_eq!(corner.edge(19, 2), None, "no lower edge");

        // The same shapes of a floor that is no table.
        let floor = tile(2816 + 28, 0).unwrap();
        assert_eq!(floor.texels(3, 24), [Some((32 + 3, 88)), None]);
        assert_eq!(floor.overhang(3, 0), None);
        assert_eq!(floor.edge(3, 0), None);
    }

    #[test]
    fn the_filling_of_an_inner_corner_is_measured_per_column() {
        // Shape 47: both lower pieces are at y 16, at x 0 and 16.
        let table = tile(2816 + 48 * 15 + 47, 0).unwrap();
        let (bx, by) = table.block;
        assert_eq!((bx, by), (448, 96));
        assert_eq!(table.texels(0, 16)[0], Some((bx, by + 48)));
        assert_eq!(table.texels(16, 16)[0], Some((bx + 48, by + 48)));
        assert_eq!(table.texels(0, 24)[1], Some((bx, by + 16)));
        assert_eq!(table.texels(16, 24)[1], Some((bx + 16, by + 16)));
    }

    /// Scenario "Counter, measured on the original" of `core-scene`.
    #[test]
    fn the_step_and_the_flash_level_follow_the_updates() {
        assert_eq!(animation_step(0), 0);
        assert_eq!(animation_step(29), 0);
        assert_eq!(animation_step(30), 1);
        assert_eq!(animation_step(100_000), 3333);
        let levels: Vec<u8> = (0..40).step_by(4).map(flash_level).collect();
        assert_eq!(levels, [4, 5, 6, 7, 8, 7, 6, 5, 4, 5]);
        assert_eq!(flash_level(3), 4);
        assert_eq!(flash_level(u64::MAX), flash_level(u64::MAX % 32));
    }

    #[test]
    fn a_flash_entry_holds_three_nibbles() {
        assert_eq!(flash_amounts(0, 4), None);
        assert_eq!(flash_amounts(-1, 4), None);
        assert_eq!(flash_amounts(i16::MIN, 4), None);
        assert_eq!(flash_amounts(0x111, 4), Some([4, 4, 4]));
        assert_eq!(flash_amounts(0xf52, 8), Some([120, 40, 16]));
        assert_eq!(
            flash_amounts(0x7fff, 5),
            Some([75, 75, 75]),
            "bits 12 to 14"
        );
        assert_eq!(flash_amounts(0x1000, 5), Some([0, 0, 0]));
        assert_eq!(flash_amounts(0xfff, 255), Some([3825; 3]));
    }

    /// Scenario "Shadow, measured on the original".
    #[test]
    fn a_shadow_darkens_by_127_of_256() {
        assert_eq!(shadow(200), 99);
        assert_eq!(shadow(255), 126);
        assert_eq!(shadow(0), 0);
        assert_eq!(shadow(1), 0);
    }
}
