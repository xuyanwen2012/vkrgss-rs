//! Tile numbering: which texel of which sheet a tile number of a tilemap shows.
//!
//! This is the one Rust definition of these rules. The guest uses it to find which tiles of a
//! map draw anything, the reference compositor to evaluate a tilemap layer op; the compute
//! shader of the Vulkan backend has a copy, whose table a test compares with
//! [`AUTOTILE_QUARTERS`].
//!
//! A tileset is 8 tiles wide. An autotile sheet holds, per animation frame, a 6 by 8 grid of
//! 16-pixel quarters from which each of the 48 patterns takes four; a sheet 32 pixels high
//! holds whole tiles instead, one per animation frame.

use crate::Size;

/// Side of a tile.
pub const TILE_SIZE: u32 = 32;
/// Side of a quarter of an autotile.
pub const QUARTER_SIZE: u32 = 16;
/// Width of one animation frame of an autotile sheet that holds quarters.
pub const AUTOTILE_FRAME_WIDTH: u32 = 96;
/// Number of patterns of an autotile.
pub const AUTOTILE_PATTERNS: usize = 48;
/// The first tile number that names a pattern of an autotile.
pub const FIRST_AUTOTILE: i16 = 48;
/// The first tile number that names a tile of the tileset.
pub const FIRST_TILESET: i16 = 384;

/// Tiles in a row of the tileset.
const TILESET_COLUMNS: u32 = 8;
/// Quarters in a row of an autotile frame.
const QUARTER_COLUMNS: u32 = 6;

/// For each autotile pattern, its top-left, top-right, bottom-left and bottom-right quarter.
/// Quarters are numbered from 1 to 48 row by row over the 6 by 8 grid of a frame.
///
/// Source: the table of the original runtime, read from `RGSS104E.dll`; it equals the table
/// published with community tilemap scripts.
#[rustfmt::skip]
pub const AUTOTILE_QUARTERS: [[u8; 4]; AUTOTILE_PATTERNS] = [
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

/// The bitmap a tile is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sheet {
    Tileset,
    /// One of the seven autotile sheets, counted from 0.
    Autotile(usize),
}

/// The sheet tile `number` is drawn from, or `None` for a number that draws nothing.
///
/// That numbers 1 to 47 draw nothing was measured on the original runtime.
pub const fn sheet(number: i16) -> Option<Sheet> {
    if number >= FIRST_TILESET {
        Some(Sheet::Tileset)
    } else if number >= FIRST_AUTOTILE {
        // Numbers 48 to 383 give 0 to 6.
        Some(Sheet::Autotile(number as usize / AUTOTILE_PATTERNS - 1))
    } else {
        None
    }
}

/// Whether an autotile sheet of this height holds whole tiles rather than quarters.
///
/// As measured on the original runtime, sheets lower than 32 pixels count as single-tile
/// sheets, and sheets between 32 and 128 pixels high draw the quarters they have.
pub const fn is_single_tile_sheet(height: u32) -> bool {
    height <= TILE_SIZE
}

/// Number of animation frames of an autotile sheet of the size `sheet`, at least 1.
pub const fn autotile_frames(sheet: Size) -> u32 {
    let frame = if is_single_tile_sheet(sheet.height) {
        TILE_SIZE
    } else {
        AUTOTILE_FRAME_WIDTH
    };
    let frames = sheet.width / frame;
    if frames == 0 { 1 } else { frames }
}

/// The texel that pixel (`x`, `y`) of tile `number`, both from 0 to 31, takes from the
/// tile's sheet of the size `sheet_size`, an autotile at the animation frame of step
/// `animation_step`, which is the step modulo [`autotile_frames`]. `None` for a number that
/// draws nothing and for a texel outside the sheet. Every backend must match this.
pub const fn texel(
    number: i16,
    x: u32,
    y: u32,
    sheet_size: Size,
    animation_step: u64,
) -> Option<(u32, u32)> {
    let (u, v) = match sheet(number) {
        None => return None,
        Some(Sheet::Tileset) => {
            let index = (number - FIRST_TILESET) as u32;
            (
                index % TILESET_COLUMNS * TILE_SIZE + x,
                index / TILESET_COLUMNS * TILE_SIZE + y,
            )
        }
        Some(Sheet::Autotile(_)) => {
            // Less than the number of frames, which is a `u32`.
            let frame = (animation_step % autotile_frames(sheet_size) as u64) as u32;
            if is_single_tile_sheet(sheet_size.height) {
                (frame * TILE_SIZE + x, y)
            } else {
                let quarters = AUTOTILE_QUARTERS[number as usize % AUTOTILE_PATTERNS];
                let corner = y / QUARTER_SIZE * 2 + x / QUARTER_SIZE;
                let quarter = quarters[corner as usize] as u32 - 1;
                (
                    frame * AUTOTILE_FRAME_WIDTH
                        + quarter % QUARTER_COLUMNS * QUARTER_SIZE
                        + x % QUARTER_SIZE,
                    quarter / QUARTER_COLUMNS * QUARTER_SIZE + y % QUARTER_SIZE,
                )
            }
        }
    };
    if u < sheet_size.width && v < sheet_size.height {
        Some((u, v))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The texels of the four corners of the top-left and the bottom-right quarter of a tile.
    fn corners(number: i16, sheet: Size, step: u64) -> [Option<(u32, u32)>; 4] {
        [(0, 0), (15, 15), (16, 16), (31, 31)].map(|(x, y)| texel(number, x, y, sheet, step))
    }

    #[test]
    fn numbers_from_384_name_tiles_of_the_tileset() {
        let sheet_size = Size::new(256, 64);
        assert_eq!(sheet(384), Some(Sheet::Tileset));
        assert_eq!(texel(384, 0, 0, sheet_size, 0), Some((0, 0)));
        assert_eq!(texel(384, 31, 31, sheet_size, 0), Some((31, 31)));
        assert_eq!(texel(384 + 8 + 3, 5, 6, sheet_size, 0), Some((101, 38)));
        assert_eq!(
            texel(384 + 7, 0, 0, sheet_size, 5),
            Some((224, 0)),
            "the tileset is not animated"
        );
        assert_eq!(
            texel(i16::MAX, 31, 31, Size::new(256, 131_072), 0),
            Some((255, 129_535))
        );
    }

    #[test]
    fn numbers_below_48_draw_nothing() {
        for number in [0, 1, 47, -1, i16::MIN] {
            assert_eq!(sheet(number), None, "{number}");
            assert_eq!(
                texel(number, 0, 0, Size::new(256, 256), 0),
                None,
                "{number}"
            );
        }
    }

    #[test]
    fn a_texel_outside_the_sheet_is_not_drawn() {
        // The tileset has two rows; the tile is in the third.
        assert_eq!(texel(384 + 16, 0, 0, Size::new(256, 64), 0), None);
        // The tileset is four tiles wide; the tile is in the fifth column.
        assert_eq!(texel(384 + 4, 0, 0, Size::new(128, 64), 0), None);
        assert_eq!(texel(384 + 3, 31, 0, Size::new(128, 64), 0), Some((127, 0)));
        // A tileset that ends inside a tile shows the part it has.
        assert_eq!(texel(384, 9, 9, Size::new(10, 10), 0), Some((9, 9)));
        assert_eq!(texel(384, 10, 9, Size::new(10, 10), 0), None);
        // An autotile sheet without its lower half has only the quarters of the upper one.
        assert_eq!(corners(48, Size::new(96, 64), 0), [None; 4]);
        assert!(
            corners(48 + 47, Size::new(96, 64), 0)
                .iter()
                .all(Option::is_some)
        );
    }

    #[test]
    fn numbers_from_48_to_383_name_patterns_of_the_seven_autotiles() {
        assert_eq!(sheet(48), Some(Sheet::Autotile(0)));
        assert_eq!(sheet(95), Some(Sheet::Autotile(0)));
        assert_eq!(sheet(96), Some(Sheet::Autotile(1)));
        assert_eq!(sheet(383), Some(Sheet::Autotile(6)));
        let sheet_size = Size::new(96, 128);
        assert_eq!(corners(48, sheet_size, 0), corners(96, sheet_size, 0));
        assert_eq!(corners(95, sheet_size, 0), corners(383, sheet_size, 0));
    }

    #[test]
    fn the_enclosed_pattern_takes_the_middle_of_the_lower_part() {
        let found = [(0, 0), (16, 0), (0, 16), (16, 16)]
            .map(|(x, y)| texel(48, x, y, Size::new(96, 128), 0));
        let expected = [(32, 64), (48, 64), (32, 80), (48, 80)].map(Some);
        assert_eq!(found, expected);
        assert_eq!(texel(48, 21, 30, Size::new(96, 128), 0), Some((53, 94)));
    }

    #[test]
    fn the_isolated_pattern_takes_the_top_left_tile() {
        for (x, y) in [(0, 0), (15, 3), (16, 16), (31, 31)] {
            assert_eq!(texel(48 + 47, x, y, Size::new(96, 128), 0), Some((x, y)));
        }
    }

    #[test]
    fn every_pattern_takes_each_corner_from_the_same_corner_of_a_tile() {
        // A quarter keeps its place: a top-left corner comes from an even column and an even
        // row of the grid of quarters, and so on. This holds for the whole table.
        for (pattern, quarters) in AUTOTILE_QUARTERS.iter().enumerate() {
            for (corner, &quarter) in quarters.iter().enumerate() {
                assert!((1..=48).contains(&quarter), "pattern {pattern}");
                let (column, row) = ((quarter - 1) % 6, (quarter - 1) / 6);
                assert_eq!(
                    (usize::from(column % 2), usize::from(row % 2)),
                    (corner % 2, corner / 2),
                    "pattern {pattern}, corner {corner}"
                );
            }
        }
    }

    #[test]
    fn a_frame_of_an_autotile_is_96_pixels_wide() {
        assert_eq!(autotile_frames(Size::new(96, 128)), 1);
        assert_eq!(autotile_frames(Size::new(384, 128)), 4);
        assert_eq!(autotile_frames(Size::new(200, 128)), 2);
        assert_eq!(
            autotile_frames(Size::new(50, 128)),
            1,
            "an undersized sheet has one frame"
        );
        let sheet_size = Size::new(384, 128);
        for number in [48, 60, 95] {
            let moved = corners(number, sheet_size, 0).map(|texel| {
                let (u, v) = texel.unwrap();
                Some((u + 192, v))
            });
            assert_eq!(corners(number, sheet_size, 2), moved);
            assert_eq!(corners(number, sheet_size, 6), moved, "the steps repeat");
        }
    }

    #[test]
    fn a_sheet_32_pixels_high_supplies_the_whole_tile() {
        assert_eq!(autotile_frames(Size::new(32, 32)), 1);
        assert_eq!(autotile_frames(Size::new(128, 32)), 4);
        for number in [48, 70, 95] {
            assert_eq!(texel(number, 17, 9, Size::new(32, 32), 0), Some((17, 9)));
            assert_eq!(texel(number, 17, 9, Size::new(128, 32), 3), Some((113, 9)));
            assert_eq!(texel(number, 17, 9, Size::new(128, 32), 5), Some((49, 9)));
        }
    }
}
