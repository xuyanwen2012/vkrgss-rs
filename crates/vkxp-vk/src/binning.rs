//! CPU tile binning: which records each 16x16 tile has to evaluate.
//!
//! Pure functions without Vulkan types. Binning only decides which records a pixel's invocation
//! looks at; every record is still tested against its own clip in the shader, so the tile grid
//! cannot change a pixel as long as each tile lists, in op order, every record that reaches it.

use vkxp_proto::Size;

use crate::device::TILE_SIZE;
use crate::records::ClipRect;

/// The tile lists of one frame, in the form the shader reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TileLists {
    /// Size of the tile grid: the logical size divided by the tile size, rounded up.
    pub tiles_x: u32,
    pub tiles_y: u32,
    /// Per tile, in row order: the offset of its list in `indices` and the list's length.
    pub headers: Vec<[u32; 2]>,
    /// Record indices, tile after tile, each list in ascending (op) order.
    pub indices: Vec<u32>,
}

/// The inclusive ranges of tile columns and rows a clip rectangle reaches.
fn tile_span(clip: &ClipRect) -> (std::ops::RangeInclusive<u32>, std::ops::RangeInclusive<u32>) {
    (
        clip.x0 / TILE_SIZE..=(clip.x1 - 1) / TILE_SIZE,
        clip.y0 / TILE_SIZE..=(clip.y1 - 1) / TILE_SIZE,
    )
}

/// Assigns each record, given by its clip rectangle on a screen of `size`, to every tile the
/// rectangle intersects.
///
/// Fails with the number of list entries needed when that exceeds `max_indices`; nothing of
/// that size is allocated first.
pub(crate) fn bin(size: Size, clips: &[ClipRect], max_indices: u64) -> Result<TileLists, u64> {
    let tiles_x = size.width.div_ceil(TILE_SIZE);
    let tiles_y = size.height.div_ceil(TILE_SIZE);

    let needed: u64 = clips
        .iter()
        .map(|clip| {
            let (columns, rows) = tile_span(clip);
            u64::from(columns.end() - columns.start() + 1)
                * u64::from(rows.end() - rows.start() + 1)
        })
        .sum();
    if needed > max_indices.min(u64::from(u32::MAX)) {
        return Err(needed);
    }

    // First pass: the length of each tile's list.
    let mut headers = vec![[0u32; 2]; tiles_x as usize * tiles_y as usize];
    let tile_index = |column: u32, row: u32| row as usize * tiles_x as usize + column as usize;
    for clip in clips {
        let (columns, rows) = tile_span(clip);
        for row in rows {
            for column in columns.clone() {
                headers[tile_index(column, row)][1] += 1;
            }
        }
    }
    // Offsets, with each length reset to serve as the fill position of the second pass.
    let mut offset = 0;
    for header in &mut headers {
        header[0] = offset;
        offset += header[1];
        header[1] = 0;
    }
    // Second pass, in record order, so every list comes out in op order.
    let mut indices = vec![0u32; needed as usize];
    for (record, clip) in clips.iter().enumerate() {
        let (columns, rows) = tile_span(clip);
        for row in rows {
            for column in columns.clone() {
                let header = &mut headers[tile_index(column, row)];
                indices[(header[0] + header[1]) as usize] = record as u32;
                header[1] += 1;
            }
        }
    }

    Ok(TileLists {
        tiles_x,
        tiles_y,
        headers,
        indices,
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vkxp_proto::Rect;

    use super::*;

    fn clip(x0: u32, y0: u32, x1: u32, y1: u32) -> ClipRect {
        ClipRect { x0, y0, x1, y1 }
    }

    /// The list of the tile at (`column`, `row`).
    fn list(lists: &TileLists, column: u32, row: u32) -> &[u32] {
        let [offset, count] = lists.headers[(row * lists.tiles_x + column) as usize];
        &lists.indices[offset as usize..(offset + count) as usize]
    }

    #[test]
    fn grid_size_rounds_up() {
        for (size, expected) in [
            (Size::new(640, 480), (40, 30)),
            (Size::new(641, 479), (41, 30)),
            (Size::new(1, 1), (1, 1)),
            (Size::new(16, 17), (1, 2)),
            (Size::new(15, 16), (1, 1)),
        ] {
            let lists = bin(size, &[], u64::MAX).unwrap();
            assert_eq!((lists.tiles_x, lists.tiles_y), expected);
            assert_eq!(lists.headers.len() as u32, expected.0 * expected.1);
            assert!(lists.headers.iter().all(|header| header[1] == 0));
            assert!(lists.indices.is_empty());
        }
    }

    #[test]
    fn record_is_listed_in_exactly_the_tiles_it_reaches() {
        // Pixels 15..=16 by 31..=32: the corner where four tiles meet.
        let lists = bin(Size::new(48, 48), &[clip(15, 31, 17, 33)], u64::MAX).unwrap();
        for row in 0..3 {
            for column in 0..3 {
                let expected: &[u32] = if column < 2 && row >= 1 { &[0] } else { &[] };
                assert_eq!(
                    list(&lists, column, row),
                    expected,
                    "tile ({column}, {row})"
                );
            }
        }
    }

    #[test]
    fn clip_ending_on_a_tile_boundary_does_not_reach_the_next_tile() {
        let lists = bin(
            Size::new(48, 16),
            &[clip(0, 0, 16, 16), clip(16, 0, 32, 16)],
            u64::MAX,
        )
        .unwrap();
        assert_eq!(list(&lists, 0, 0), [0]);
        assert_eq!(list(&lists, 1, 0), [1]);
        assert_eq!(list(&lists, 2, 0), [0u32; 0]);
    }

    #[test]
    fn lists_are_in_op_order() {
        let full = clip(0, 0, 32, 32);
        let clips = [full, clip(0, 0, 1, 1), full, clip(31, 31, 32, 32), full];
        let lists = bin(Size::new(32, 32), &clips, u64::MAX).unwrap();
        assert_eq!(list(&lists, 0, 0), [0, 1, 2, 4]);
        assert_eq!(list(&lists, 1, 0), [0, 2, 4]);
        assert_eq!(list(&lists, 1, 1), [0, 2, 3, 4]);
    }

    #[test]
    fn too_many_list_entries_are_reported_before_allocating() {
        let size = Size::new(16384, 16384);
        let full = clip(0, 0, 16384, 16384);
        // 1024 * 1024 tiles for each of 5000 records.
        let clips = vec![full; 5000];
        assert_eq!(bin(size, &clips, 1 << 30), Err(5000 << 20));
        // The count is also bounded by what a 32-bit offset can address.
        assert_eq!(bin(size, &clips, u64::MAX), Err(5000 << 20));
        // Exactly at the limit is accepted.
        let small = [clip(0, 0, 32, 16); 3];
        assert_eq!(bin(Size::new(32, 16), &small, 5), Err(6));
        assert!(bin(Size::new(32, 16), &small, 6).is_ok());
    }

    prop_compose! {
        /// A logical size that is a multiple of the tile size or not, with clips on it: some
        /// arbitrary, some snapped to tile boundaries, some a single pixel.
        fn size_and_clips()(width in 1u32..120, height in 1u32..90, snap in any::<bool>())(
            rects in prop::collection::vec(
                (-20i32..140, -20i32..110, 0u32..150, 0u32..120, 0u8..4),
                0..24,
            ),
            width in Just(if snap { width.div_ceil(16) * 16 } else { width }),
            height in Just(if snap { height.div_ceil(16) * 16 } else { height }),
        ) -> (Size, Vec<ClipRect>) {
            let size = Size::new(width, height);
            let clips = rects
                .into_iter()
                .filter_map(|(x, y, w, h, shape)| {
                    let rect = match shape {
                        0 => Rect::new(x / 16 * 16, y / 16 * 16, w / 16 * 16, h / 16 * 16),
                        1 => Rect::new(x, y, 1, 1),
                        _ => Rect::new(x, y, w, h),
                    };
                    ClipRect::on_screen(rect, size)
                })
                .collect();
            (size, clips)
        }
    }

    proptest! {
        /// For every pixel, the list of its tile is, in op order, exactly the records whose
        /// clip contains a pixel of that tile.
        #[test]
        fn every_tile_lists_exactly_the_records_that_reach_it((size, clips) in size_and_clips()) {
            let lists = bin(size, &clips, u64::MAX).unwrap();
            prop_assert_eq!(lists.tiles_x, size.width.div_ceil(16));
            prop_assert_eq!(lists.tiles_y, size.height.div_ceil(16));
            prop_assert_eq!(lists.headers.len() as u32, lists.tiles_x * lists.tiles_y);

            // Which records contain at least one pixel of each tile, found pixel by pixel.
            let mut expected = vec![Vec::new(); lists.headers.len()];
            for (record, clip) in clips.iter().enumerate() {
                for y in 0..size.height {
                    for x in 0..size.width {
                        let tile = &mut expected[((y / 16) * lists.tiles_x + x / 16) as usize];
                        if clip.contains(x, y) && tile.last() != Some(&(record as u32)) {
                            tile.push(record as u32);
                        }
                    }
                }
            }
            for y in 0..size.height {
                for x in 0..size.width {
                    let (column, row) = (x / 16, y / 16);
                    prop_assert_eq!(
                        list(&lists, column, row),
                        &expected[(row * lists.tiles_x + column) as usize][..],
                        "pixel ({}, {}) of {:?}", x, y, size
                    );
                }
            }
            // The lists tile the index buffer without gaps or overlap.
            let mut offset = 0;
            for header in &lists.headers {
                prop_assert_eq!(header[0], offset);
                offset += header[1];
            }
            prop_assert_eq!(offset as usize, lists.indices.len());
        }
    }
}
