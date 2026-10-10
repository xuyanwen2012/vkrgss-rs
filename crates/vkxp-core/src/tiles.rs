//! The source rectangles a tile is drawn with, for the cached-bitmap composition the tests
//! compare tilemap ops with. The numbering itself is `vkxp_proto::tiles`.

use vkxp_proto::tiles::{
    AUTOTILE_FRAME_WIDTH, AUTOTILE_PATTERNS, AUTOTILE_QUARTERS, FIRST_TILESET, QUARTER_SIZE,
    TILE_SIZE, autotile_frames, is_single_tile_sheet,
};
pub(crate) use vkxp_proto::tiles::{Sheet, sheet};

use crate::Rect;

const TILE: i32 = TILE_SIZE as i32;
const QUARTER: i32 = QUARTER_SIZE as i32;
const FRAME: i32 = AUTOTILE_FRAME_WIDTH as i32;

/// Number of animation frames of an autotile sheet of `width` by `height` pixels, at least 1.
pub(crate) fn frames(width: i32, height: i32) -> u32 {
    autotile_frames(vkxp_proto::Size::new(
        width.unsigned_abs(),
        height.unsigned_abs(),
    ))
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
            let index = i32::from(id - FIRST_TILESET);
            whole(index % 8 * TILE, index / 8 * TILE)
        }
        Some(Sheet::Autotile(_)) => {
            // A frame number is less than the sheet's width.
            let frame = frame as i32;
            if is_single_tile_sheet(height.unsigned_abs()) {
                whole(frame * TILE, 0)
            } else {
                let quarters = AUTOTILE_QUARTERS[id.unsigned_abs() as usize % AUTOTILE_PATTERNS];
                let corners = [(0, 0), (QUARTER, 0), (0, QUARTER), (QUARTER, QUARTER)];
                std::array::from_fn(|corner| {
                    let quarter = i32::from(quarters[corner]) - 1;
                    let src = Rect::new(
                        frame * FRAME + quarter % 6 * QUARTER,
                        quarter / 6 * QUARTER,
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
