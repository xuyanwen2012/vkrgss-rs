//! Tilemaps of RGSS2: the two tilemap plane ops that draw one.
//!
//! As measured on the original runtime of VX, such a tilemap is two planes of 21 by 16
//! cells among the elements of its viewport, the lower at z 0 and the upper at z 200, both
//! drawn anew every frame from the tables and the sheets as they are then, whatever the
//! tilemap's `visible` is. The planes start at the corner of the viewport's rectangle, less
//! what the map is scrolled by within a cell; the ox and oy of the viewport only add to the
//! tilemap's own, as the last refresh of the tilemap found them.

use vkxp_proto::tiles_rgss2 as tiles;
use vkxp_proto::{BitmapId, Op, Plane, PlaneRules, TableId, TilemapPlane};

use super::{Role, SentTables, Strip};
use crate::extract::{Assembly, Level};
use crate::{Bitmaps, Tilemap};

/// The z of the upper plane.
const UPPER_Z: i64 = 200;

/// What this frame draws of a tilemap of RGSS2: what its two ops share.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Planes {
    corner: (i32, i32),
    first_cell: (u32, u32),
    animation_step: u64,
    flash_level: u8,
    map: TableId,
    passages: Option<TableId>,
    flash: Option<TableId>,
    /// The live sheets.
    bitmaps: [Option<BitmapId>; tiles::SHEETS],
}

impl Tilemap {
    /// Finds what this frame draws of a tilemap of RGSS2 in `level` and sends its tables
    /// where the compositor lacks them. Without a `level`, because its viewport is not
    /// drawn, and without cells in its map data it draws nothing.
    pub(super) fn refresh_planes(
        &mut self,
        bitmaps: &Bitmaps,
        level: Option<&Level>,
        tables: &mut SentTables,
    ) {
        self.cache.planes = None;
        let Some(rgss2) = &self.rgss2 else { return };
        let held = [&self.map_data, &rgss2.passages, &self.flash_data];
        for table in held.into_iter().flatten() {
            tables.hold(table, Role::Map);
        }
        let (Some(level), Some(map)) = (level, &self.map_data) else {
            return;
        };
        let (xsize, ysize) = {
            let map = map.read();
            (i64::from(map.xsize()), i64::from(map.ysize()))
        };
        if xsize <= 0 || ysize <= 0 {
            return;
        }

        let tile = i64::from(tiles::TILE_SIZE);
        let x = i64::from(self.ox) + i64::from(rgss2.latched.0);
        let y = i64::from(self.oy) + i64::from(rgss2.latched.1);
        // The corner of the viewport's rectangle, less the place of the origin in its cell.
        let corner = |rect: i64, origin: i64| {
            let corner = rect - origin.rem_euclid(tile);
            corner.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
        };
        // A remainder of a size that is an `i32`.
        let first = |origin: i64, size: i64| origin.div_euclid(tile).rem_euclid(size) as u32;
        let mut current = |table| tables.current(table, Role::Map);
        self.cache.planes = Some(Planes {
            corner: (
                corner(level.dx + level.ox, x),
                corner(level.dy + level.oy, y),
            ),
            first_cell: (first(x, xsize), first(y, ysize)),
            animation_step: tiles::animation_step(self.updates),
            flash_level: tiles::flash_level(self.updates),
            map: current(map),
            passages: rgss2.passages.as_ref().map(&mut current),
            flash: self.flash_data.as_ref().map(&mut current),
            bitmaps: rgss2
                .bitmaps
                .map(|id| id.filter(|&id| bitmaps.rect(id).is_ok())),
        });
    }

    /// The two planes this frame draws of a tilemap of RGSS2, each with its z.
    pub(super) fn plane_strips(&self) -> Vec<(i64, Strip)> {
        if self.cache.planes.is_none() {
            return Vec::new();
        }
        vec![
            (0, Strip::Plane(Plane::Lower)),
            (UPPER_Z, Strip::Plane(Plane::Upper)),
        ]
    }
}

impl Assembly<'_> {
    /// The op of one plane of a tilemap of RGSS2.
    pub(super) fn tilemap_plane(&mut self, tilemap: &Tilemap, plane: Plane, level: &Level) {
        let Some(planes) = &tilemap.cache.planes else {
            return;
        };
        self.referenced.extend(planes.bitmaps.into_iter().flatten());
        self.ops.push(Op::TilemapPlane(TilemapPlane {
            clip: level.clip,
            plane,
            rules: PlaneRules::Rgss2,
            corner: planes.corner,
            first_cell: planes.first_cell,
            animation_step: planes.animation_step,
            flash_level: planes.flash_level,
            map: planes.map,
            passages: planes.passages,
            flash: planes.flash,
            bitmaps: planes.bitmaps,
        }));
    }
}

#[cfg(test)]
mod tests;
