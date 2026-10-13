//! What the tile numbering of RGSS3 adds to that of RGSS2: tables by a bit of the flags
//! table, a fourth layer of the map that holds shadows.
//!
//! Everything else of a tilemap of RGSS3 is [`crate::tiles_rgss2`]: the numbering, the shape
//! tables, the animation, the flash, [`tiles_rgss2::shadow`], [`tiles_rgss2::is_wall`] and the
//! texels of a [`Tile`]. The reference compositor evaluates a [`crate::TilemapPlane`] of
//! [`crate::PlaneRules::Rgss3`] with both; the compute shader of the Vulkan backend mirrors
//! them.
//!
//! Everything here was measured on the original runtime of VX Ace (`RGSS301.dll`, versions
//! 3.0.0.1 and 3.0.1.1 alike) on 2026-10-13. Where the original reads outside its arrays,
//! numbers 1024 to 1535, negative numbers and maps of fewer than four layers, a tile draws
//! nothing here and a layer the map lacks is empty.

use crate::tiles_rgss2::{self, QUARTER_SIZE, Tile};

/// The bit of an entry of the flags table that makes a tile of A2 a table.
pub const TABLE: i16 = 0x80;
/// Number of layers of a map that are read.
pub const LAYERS: u32 = 4;
/// The layer of the map that holds the shadow mask instead of tile numbers.
pub const SHADOW_LAYER: u32 = 3;

/// The numbers of A2: the first and one past the last.
const FIRST_A2: i16 = 2816;
const END_A2: i16 = 4352;

/// Whether tile `number`, whose entry of the flags table is `flags`, is a table: a tile of
/// A2 with the bit [`TABLE`], whatever the column of its autotile.
pub const fn is_table(number: i16, flags: i16) -> bool {
    number >= FIRST_A2 && number < END_A2 && flags & TABLE != 0
}

/// Tile `number`, whose entry of the flags table is `flags`, at the animation step
/// `animation_step`: [`tiles_rgss2::tile`] with [`Tile::table`] as [`is_table`] says.
pub const fn tile(number: i16, flags: i16, animation_step: u64) -> Option<Tile> {
    match tiles_rgss2::tile(number, animation_step) {
        Some(tile) => Some(Tile {
            table: is_table(number, flags),
            ..tile
        }),
        None => None,
    }
}

/// Whether the element `mask` of the shadow layer darkens pixel (`x`, `y`) of its cell, both
/// from 0 to 31: bit `q` of the mask, for `q` from 0 to 3, covers the quarter of 16 by 16
/// pixels at (`16 * (q & 1)`, `16 * (q >> 1)`). Higher bits do nothing.
pub const fn shadow_covers(mask: i16, x: u32, y: u32) -> bool {
    mask >> (y / QUARTER_SIZE * 2 + x / QUARTER_SIZE) & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_is_a_tile_of_a2_with_the_bit() {
        for number in [2816, 2844, 2864, 3180, 4351] {
            assert!(is_table(number, 0x80), "{number}");
            assert!(is_table(number, 0x90), "{number}");
            assert!(is_table(number, -1), "{number}");
            for flags in [0, 0x10, 0x6f, 0x100, 0x7f6f] {
                assert!(!is_table(number, flags), "{number} {flags:#x}");
            }
        }
        for number in [0, -1, 37, 1545, 2048, 2815, 4352, 4380, 5888, i16::MAX] {
            assert!(!is_table(number, 0x80), "{number}");
        }
    }

    #[test]
    fn a_tile_is_that_of_rgss2_but_for_the_table() {
        // Column 7 of A2, the table of RGSS2, is none without the bit.
        let column_7 = tile(3180, 0, 0).unwrap();
        assert!(!column_7.table);
        assert_eq!(
            Tile {
                table: true,
                ..column_7
            },
            tiles_rgss2::tile(3180, 0).unwrap()
        );
        // The first autotile of A2 in shape 28 is one with it.
        let table = tile(2844, 0x80, 0).unwrap();
        assert!(table.table);
        assert_eq!(table.texels(3, 24), [Some((35, 56)), Some((35, 80))]);
        assert_eq!(table.overhang(3, 0), Some((35, 88)));
        assert_eq!(table.edge(20, 7), Some((20, 95)));
        assert_eq!(tile(2844, 0, 0), tiles_rgss2::tile(2844, 0));
        for (number, step) in [(37, 0), (1545, 2), (2048, 1), (4380, 0), (8191, 3)] {
            assert_eq!(
                tile(number, 0x80, step),
                tiles_rgss2::tile(number, step),
                "{number}"
            );
        }
    }

    #[test]
    fn undefined_numbers_draw_nothing() {
        for number in [0, -1, -256, -1280, i16::MIN, 1024, 1300, 1535] {
            assert_eq!(tile(number, 0x90, 0), None, "{number}");
        }
    }

    /// Scenario "Quarters, measured on the original" of `frame-composition`.
    #[test]
    fn a_shadow_bit_covers_its_quarter() {
        let covered = |mask| {
            [(0, 0), (16, 0), (0, 16), (16, 16)].map(|(x, y)| shadow_covers(mask, x + 15, y + 3))
        };
        assert_eq!(covered(0), [false; 4]);
        assert_eq!(covered(1), [true, false, false, false]);
        assert_eq!(covered(2), [false, true, false, false]);
        assert_eq!(covered(4), [false, false, true, false]);
        assert_eq!(covered(8), [false, false, false, true]);
        assert_eq!(covered(5), [true, false, true, false]);
        assert_eq!(covered(-1), [true; 4]);
        assert_eq!(covered(0x3f0a), [false, true, false, true], "the region id");
        assert_eq!(covered(16), [false; 4]);
        assert_eq!(covered(i16::MIN), [false; 4]);
    }
}
