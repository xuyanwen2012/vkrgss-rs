//! Tile identifiers: which pixels of the tileset or of an autotile sheet a tile is drawn from.
//!
//! A tileset is 8 tiles wide. An autotile sheet holds, per animation frame, a 6 by 8 grid of
//! 16-pixel quarters from which each of the 48 patterns takes four; a sheet 32 pixels high
//! holds whole tiles instead, one per animation frame.

use crate::Rect;

/// Side of a tile.
pub(crate) const TILE: i32 = 32;
/// Side of a quarter of an autotile.
const QUARTER: i32 = 16;
/// Width of one animation frame of an autotile sheet.
const FRAME: i32 = 96;
/// Number of patterns of an autotile, and the first identifier that names one.
const PATTERNS: i32 = 48;
/// The first identifier that names a tile of the tileset.
const FIRST_TILESET: i32 = 384;
/// Tiles in a row of the tileset.
const TILESET_COLUMNS: i32 = 8;
/// Quarters in a row of an autotile frame.
const QUARTER_COLUMNS: i32 = 6;

/// For each autotile pattern, its top-left, top-right, bottom-left and bottom-right quarter.
/// Quarters are numbered from 1 to 48 row by row over the 6 by 8 grid of a frame.
///
/// Source: the table of the original runtime, read from `RGSS104E.dll`; it equals the table
/// published with community tilemap scripts.
#[rustfmt::skip]
pub(crate) const AUTOTILES: [[u8; 4]; PATTERNS as usize] = [
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sheet {
    Tileset,
    /// One of the seven autotile sheets, counted from 0.
    Autotile(usize),
}

/// The sheet tile `id` is drawn from, or `None` for an identifier that draws nothing.
///
/// PLACEHOLDER: that identifiers 1 to 47 draw nothing is not yet checked against the original
/// runtime.
pub(crate) fn sheet(id: i16) -> Option<Sheet> {
    let id = i32::from(id);
    if id >= FIRST_TILESET {
        Some(Sheet::Tileset)
    } else if id >= PATTERNS {
        // Identifiers 48 to 383 give 0 to 6.
        Some(Sheet::Autotile((id / PATTERNS - 1) as usize))
    } else {
        None
    }
}

/// Whether an autotile sheet of this height holds whole tiles rather than quarters.
///
/// PLACEHOLDER: the original runtime's treatment of sheets lower than 32 pixels, and of
/// sheets between 32 and 128 pixels high, is not yet checked; here the former count as
/// single-tile sheets and the latter draw the quarters they have.
fn single_tile(height: i32) -> bool {
    height <= TILE
}

/// Number of animation frames of an autotile sheet of `width` by `height` pixels, at least 1.
pub(crate) fn frames(width: i32, height: i32) -> u32 {
    let frame = if single_tile(height) { TILE } else { FRAME };
    (width / frame).max(1).unsigned_abs()
}

/// One transfer of a tile: `src` of the sheet, drawn at (`x`, `y`) inside the tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Piece {
    pub(crate) src: Rect,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

/// The transfers that draw tile `id` from its sheet of `width` by `height` pixels, an
/// autotile at animation frame `frame`. Pieces that lie outside the sheet are left out, as
/// is everything for an identifier that draws nothing.
pub(crate) fn pieces(id: i16, width: i32, height: i32, frame: u32) -> impl Iterator<Item = Piece> {
    let whole = |x: i32, y: i32| [Some((Rect::new(x, y, TILE, TILE), 0, 0)), None, None, None];
    let found = match sheet(id) {
        None => [None; 4],
        Some(Sheet::Tileset) => {
            let index = i32::from(id) - FIRST_TILESET;
            whole(
                index % TILESET_COLUMNS * TILE,
                index / TILESET_COLUMNS * TILE,
            )
        }
        Some(Sheet::Autotile(_)) => {
            // A frame number is less than the sheet's width.
            let frame = frame as i32;
            if single_tile(height) {
                whole(frame * TILE, 0)
            } else {
                let quarters = AUTOTILES[(i32::from(id) % PATTERNS) as usize];
                let corners = [(0, 0), (QUARTER, 0), (0, QUARTER), (QUARTER, QUARTER)];
                std::array::from_fn(|corner| {
                    let quarter = i32::from(quarters[corner]) - 1;
                    let src = Rect::new(
                        frame * FRAME + quarter % QUARTER_COLUMNS * QUARTER,
                        quarter / QUARTER_COLUMNS * QUARTER,
                        QUARTER,
                        QUARTER,
                    );
                    Some((src, corners[corner].0, corners[corner].1))
                })
            }
        }
    };
    found
        .into_iter()
        .flatten()
        .filter(move |(src, _, _)| src.x < width && src.y < height)
        .map(|(src, x, y)| Piece { src, x, y })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The source rectangles of the pieces, with where they land in the tile.
    fn found(id: i16, width: i32, height: i32, frame: u32) -> Vec<(Rect, i32, i32)> {
        pieces(id, width, height, frame)
            .map(|piece| (piece.src, piece.x, piece.y))
            .collect()
    }

    fn quarter(x: i32, y: i32) -> Rect {
        Rect::new(x, y, 16, 16)
    }

    #[test]
    fn identifiers_from_384_name_tiles_of_the_tileset() {
        assert_eq!(sheet(384), Some(Sheet::Tileset));
        assert_eq!(found(384, 256, 64, 0), [(Rect::new(0, 0, 32, 32), 0, 0)]);
        assert_eq!(
            found(384 + 8 + 3, 256, 64, 0),
            [(Rect::new(96, 32, 32, 32), 0, 0)]
        );
        assert_eq!(
            found(384 + 7, 256, 64, 5),
            [(Rect::new(224, 0, 32, 32), 0, 0)],
            "the tileset is not animated"
        );
        assert_eq!(
            found(i16::MAX, 256, 131_072, 0),
            [(Rect::new(224, 129_504, 32, 32), 0, 0)]
        );
    }

    #[test]
    fn identifiers_below_48_draw_nothing() {
        for id in [0, 1, 47, -1, i16::MIN] {
            assert_eq!(sheet(id), None, "{id}");
            assert_eq!(found(id, 256, 256, 0), [], "{id}");
        }
    }

    #[test]
    fn a_tile_outside_its_bitmap_draws_nothing() {
        // The tileset has two rows; the tile is in the third.
        assert_eq!(found(384 + 16, 256, 64, 0), []);
        // The tileset is four tiles wide; the tile is in the fifth column.
        assert_eq!(found(384 + 4, 128, 64, 0), []);
        assert_eq!(found(384 + 3, 128, 64, 0).len(), 1);
        // An autotile sheet without its lower half has only the quarters of the upper one.
        assert_eq!(found(48, 96, 64, 0), []);
        assert_eq!(found(48 + 47, 96, 64, 0).len(), 4);
    }

    #[test]
    fn identifiers_from_48_to_383_name_patterns_of_the_seven_autotiles() {
        assert_eq!(sheet(48), Some(Sheet::Autotile(0)));
        assert_eq!(sheet(95), Some(Sheet::Autotile(0)));
        assert_eq!(sheet(96), Some(Sheet::Autotile(1)));
        assert_eq!(sheet(383), Some(Sheet::Autotile(6)));
        assert_eq!(found(48, 96, 128, 0), found(96, 96, 128, 0));
        assert_eq!(found(95, 96, 128, 0), found(383, 96, 128, 0));
    }

    #[test]
    fn the_enclosed_pattern_takes_the_middle_of_the_lower_part() {
        assert_eq!(
            found(48, 96, 128, 0),
            [
                (quarter(32, 64), 0, 0),
                (quarter(48, 64), 16, 0),
                (quarter(32, 80), 0, 16),
                (quarter(48, 80), 16, 16),
            ]
        );
    }

    #[test]
    fn the_isolated_pattern_takes_the_top_left_tile() {
        assert_eq!(
            found(48 + 47, 96, 128, 0),
            [
                (quarter(0, 0), 0, 0),
                (quarter(16, 0), 16, 0),
                (quarter(0, 16), 0, 16),
                (quarter(16, 16), 16, 16),
            ]
        );
    }

    #[test]
    fn every_pattern_takes_each_corner_from_the_same_corner_of_a_tile() {
        // A quarter keeps its place: a top-left corner comes from an even column and an even
        // row of the grid of quarters, and so on. This holds for the whole table.
        for (pattern, quarters) in AUTOTILES.iter().enumerate() {
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
        assert_eq!(frames(96, 128), 1);
        assert_eq!(frames(384, 128), 4);
        assert_eq!(frames(200, 128), 2);
        assert_eq!(frames(50, 128), 1, "an undersized sheet has one frame");
        let moved = |(src, x, y): (Rect, i32, i32)| (Rect::new(src.x + 192, src.y, 16, 16), x, y);
        for id in [48, 60, 95] {
            let still: Vec<_> = found(id, 384, 128, 0).into_iter().map(moved).collect();
            assert_eq!(found(id, 384, 128, 2), still);
        }
    }

    #[test]
    fn a_sheet_32_pixels_high_supplies_the_whole_tile() {
        assert_eq!(frames(32, 32), 1);
        assert_eq!(frames(128, 32), 4);
        for id in [48, 70, 95] {
            assert_eq!(found(id, 32, 32, 0), [(Rect::new(0, 0, 32, 32), 0, 0)]);
            assert_eq!(found(id, 128, 32, 3), [(Rect::new(96, 0, 32, 32), 0, 0)]);
        }
    }
}
