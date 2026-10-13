//! Tilemaps: the tilemap layer ops that draw them, and the tables those ops read.
//!
//! A tilemap is not drawn tile by tile, and nothing of it is composed here: the compositor
//! evaluates a tilemap layer op per pixel, from the map's tile numbers, the priorities and
//! the sheets. What a frame needs from this module is which ops to emit. The cells that show
//! in the tilemap's area are scanned once per frame to find which priorities occur in which
//! layers of the map and, from priority 1 on, in which rows.
//!
//! The original runtime blends every tile onto the screen by itself, in ascending priority
//! and, within a priority, in the order of the layers. So a frame draws, for priority 0, the
//! ground, one op for each layer of the map that holds such a tile, at z 0; and for every
//! higher priority one op for each row of cells and layer that holds such a tile, clipped to
//! the row and at the z that ranks the row among sprites.
//!
//! The two tables of a tilemap are sent to the compositor when an op first reads them and
//! again only when they were written to; scrolling and animation send nothing. The
//! priorities are sent as they are: no op draws a tile whose priority is not 0 to 5.
//!
//! The cells the flash data marks are filled with their colour, at the strength of the
//! pulse, in a bitmap of the scene's own that covers the visible cells and one more column
//! and row, which is drawn additively right above the ground. It is filled again when the
//! flash data, the first cell or the pulse changes.
//!
//! A tilemap of RGSS2 is two tilemap plane ops and no bitmap of the scene's own: see the
//! submodule `rgss2`.

use vkxp_proto::tiles::{self, QUARTER_SIZE, Sheet, TILE_SIZE};
use vkxp_proto::{BitmapId, BlendMode, Op, Size, TableDelta, TableId, TilemapLayer};

use crate::extract::{Assembly, Level, Look, rows};
use crate::geometry::Geometry;
use crate::{Bitmaps, Color, FontDefaults, Rect, SharedTable, Table, Tilemap};

/// Side of a tile.
const TILE: i32 = TILE_SIZE as i32;
/// Number of layers of a map.
const LAYERS: usize = 3;
/// The highest priority a tile can have.
const MAX_PRIORITY: usize = 5;
/// Number of updates one animation frame of an autotile stays.
const ANIMATION_PERIOD: u64 = 16;
/// What a nibble of the flash data is multiplied by, less 4, step by step of the pulse.
const FLASH_PULSE: [u8; 8] = [0, 1, 2, 3, 4, 3, 2, 1];
/// Number of updates one step of the pulse of the flash data stays.
const FLASH_STEP: u64 = 4;

/// What a table is to the tilemap it was sent for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// The tile numbers, sent as they are, and every table of a tilemap of RGSS2.
    Map,
    /// The priorities, of which the first row is sent.
    Priorities,
}

/// A table the compositor has.
#[derive(Debug, Clone)]
struct SentTable {
    id: TableId,
    table: SharedTable,
    role: Role,
    /// The revision that was sent.
    revision: u64,
    /// Whether a tilemap still has the table, as far as this frame has looked.
    held: bool,
}

/// The tables the compositor has of a scene's tilemaps, and the deltas that bring it up to
/// date.
#[derive(Debug, Clone, Default)]
pub(crate) struct SentTables {
    sent: Vec<SentTable>,
    /// The table without elements that a tilemap without priorities names.
    empty: Option<TableId>,
    /// Ids that were given back.
    free: Vec<TableId>,
    /// The id to give out when none is free.
    next: u32,
    deltas: Vec<TableDelta>,
}

impl SentTables {
    fn allocate(&mut self) -> TableId {
        self.free.pop().unwrap_or_else(|| {
            let id = TableId::new(self.next);
            self.next += 1;
            id
        })
    }

    /// The delta that sends `table` as `id`.
    fn set(id: TableId, table: &Table, role: Role) -> TableDelta {
        // Sizes of a table are not negative.
        let (xsize, ysize, zsize) = (
            table.xsize().unsigned_abs(),
            table.ysize().unsigned_abs(),
            table.zsize().unsigned_abs(),
        );
        match role {
            Role::Map => TableDelta::Set {
                id,
                xsize,
                ysize,
                zsize,
                data: table.data().to_vec(),
            },
            Role::Priorities => {
                let entries = (0..table.xsize()).map_while(|id| table.get(id, 0, 0));
                let data: Vec<i16> = entries.collect();
                TableDelta::Set {
                    id,
                    xsize: data.len() as u32,
                    ysize: 1,
                    zsize: 1,
                    data,
                }
            }
        }
    }

    /// Notes that a tilemap has `table` in `role`, so that the compositor keeps it.
    fn hold(&mut self, table: &SharedTable, role: Role) {
        let found = self.sent.iter_mut();
        for sent in found.filter(|sent| sent.role == role && sent.table == *table) {
            sent.held = true;
        }
    }

    /// The id under which the compositor has `table` in `role` as it is now, sending it when
    /// it is new or was written to since it was sent.
    fn current(&mut self, table: &SharedTable, role: Role) -> TableId {
        let revision = table.revision();
        let mut found = self.sent.iter_mut();
        if let Some(sent) = found.find(|sent| sent.role == role && sent.table == *table) {
            sent.held = true;
            if sent.revision != revision {
                sent.revision = revision;
                self.deltas
                    .push(SentTables::set(sent.id, &table.read(), role));
            }
            return sent.id;
        }
        let id = self.allocate();
        self.deltas.push(SentTables::set(id, &table.read(), role));
        self.sent.push(SentTable {
            id,
            table: table.clone(),
            role,
            revision,
            held: true,
        });
        id
    }

    /// The id of a table without elements, for a tilemap without priorities.
    fn empty(&mut self) -> TableId {
        if let Some(id) = self.empty {
            return id;
        }
        let id = self.allocate();
        self.deltas.push(TableDelta::Set {
            id,
            xsize: 0,
            ysize: 1,
            zsize: 1,
            data: Vec::new(),
        });
        self.empty = Some(id);
        id
    }

    /// The deltas of this frame, the destruction of every table no tilemap has any more
    /// among them. The caller must deliver them.
    pub(crate) fn finish(&mut self) -> Vec<TableDelta> {
        let (free, deltas) = (&mut self.free, &mut self.deltas);
        self.sent.retain_mut(|sent| {
            let held = std::mem::take(&mut sent.held);
            if !held {
                deltas.push(TableDelta::Destroy { id: sent.id });
                free.push(sent.id);
            }
            held
        });
        std::mem::take(deltas)
    }
}

/// What the flash overlay was filled from.
#[derive(Debug, Clone, PartialEq)]
struct FlashKey {
    /// The cell of the map in the top-left corner of the overlay.
    start: (i64, i64),
    cells: (i32, i32),
    table: (SharedTable, u64),
    multiplier: u8,
}

/// What this frame draws of a tilemap.
#[derive(Debug, Clone, PartialEq)]
struct View {
    /// The cell in the top-left corner of the area. Cells are counted across the whole
    /// plane, not wrapped into the map.
    first: (i64, i64),
    /// Columns and rows of cells the flash overlay covers.
    cells: (i32, i32),
    /// Width and height of the map in pixels, after which it repeats.
    period: (i64, i64),
    /// The tables of the ops: tile numbers and priorities. `None` when no op is drawn.
    tables: Option<(TableId, TableId)>,
    /// The live sheets.
    tileset: Option<BitmapId>,
    autotiles: [Option<BitmapId>; 7],
    animation_step: u64,
    /// The layers of the map that show a tile of priority 0.
    ground: [bool; LAYERS],
    /// For each priority from 1 on and each layer of the map, the rows of cells, counted
    /// from `first`, that show a tile of them, ascending.
    rows: [[Vec<i32>; LAYERS]; MAX_PRIORITY],
}

/// What a tilemap keeps from frame to frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TileCache {
    /// What the frame draws; `None` for a tilemap that is not drawn.
    view: Option<View>,
    /// What the frame draws of a tilemap of RGSS2; `None` when it is not drawn.
    planes: Option<rgss2::Planes>,
    /// What `flash` was filled from; `None` without flash data.
    flash_key: Option<FlashKey>,
    /// The colours of the flashing cells, when there is one among the covered cells.
    flash: Option<BitmapId>,
}

impl TileCache {
    pub(crate) fn dispose(&mut self, bitmaps: &mut Bitmaps) {
        if let Some(id) = self.flash.take() {
            bitmaps.dispose(id);
        }
        *self = TileCache::default();
    }

    /// Makes `layer` an empty bitmap of `width` by `height` pixels if `wanted`, and gives its
    /// bitmap back otherwise. An overlay too large for a bitmap is gone without.
    fn prepare(
        layer: &mut Option<BitmapId>,
        bitmaps: &mut Bitmaps,
        wanted: bool,
        width: i32,
        height: i32,
    ) {
        let size = Rect::new(0, 0, width, height);
        let reusable = layer.filter(|&id| wanted && bitmaps.rect(id) == Ok(size));
        if let Some(id) = reusable {
            // The bitmap is live: its size was just read.
            let _ = bitmaps.clear(id);
            return;
        }
        if let Some(id) = layer.take() {
            bitmaps.dispose(id);
        }
        if wanted {
            let defaults = FontDefaults::new(Vec::new());
            *layer = bitmaps
                .create(width.unsigned_abs(), height.unsigned_abs(), &defaults)
                .ok();
        }
    }
}

/// The priority of tile `id`: the entry of the priorities table, 0 without one. `None` for
/// an entry that is not 0 to 5, whose tile is not drawn.
///
/// As measured on the original runtime, which draws nothing of a tile with a priority of 6
/// to 9 and crashes, or draws nothing, on the other values.
fn priority(priorities: Option<&Table>, id: i16) -> Option<usize> {
    let value = priorities.and_then(|table| table.get(i32::from(id), 0, 0));
    let priority = usize::try_from(value.unwrap_or(0)).ok()?;
    (priority <= MAX_PRIORITY).then_some(priority)
}

/// Whether tile `id` draws a pixel from its sheet of the size `sheet` at the animation step
/// `animation_step`: whether one of its four quarters starts inside the sheet.
fn shows(id: i16, sheet: Size, animation_step: u64) -> bool {
    let corners = [
        (0, 0),
        (QUARTER_SIZE, 0),
        (0, QUARTER_SIZE),
        (QUARTER_SIZE, QUARTER_SIZE),
    ];
    let mut texels = corners
        .into_iter()
        .map(|(x, y)| tiles::texel(id, x, y, sheet, animation_step));
    texels.any(|texel| texel.is_some())
}

/// What the nibbles of the flashing cells are multiplied by after `updates` updates: from 4
/// to 8 and back in 32 updates, each value for 4 of them, as measured on the original
/// runtime.
fn flash_multiplier(updates: u64) -> u8 {
    // An index below 8.
    4 + FLASH_PULSE[(updates / FLASH_STEP % FLASH_PULSE.len() as u64) as usize]
}

impl Tilemap {
    /// Brings the flash overlay up to date for the `cells` covered from the cell `start` of
    /// the map on.
    fn refresh_flash(&mut self, bitmaps: &mut Bitmaps, start: (i64, i64), cells: (i32, i32)) {
        let key = self.flash_data.as_ref().map(|table| FlashKey {
            start,
            cells,
            table: (table.clone(), table.revision()),
            multiplier: flash_multiplier(self.updates),
        });
        let lost = self.cache.flash.is_some_and(|id| bitmaps.is_disposed(id));
        if self.cache.flash_key == key && !lost {
            return;
        }

        // As measured on the original runtime, the first cell is one of the map, and from
        // it on the flash data is counted through, repeating after its own size. A negative
        // entry draws nothing, and bits 12 to 14 are not part of the colour.
        let mut flashing = Vec::new();
        if let Some(key) = &key {
            let table = key.table.0.read();
            let (xsize, ysize) = (i64::from(table.xsize()), i64::from(table.ysize()));
            for row in 0..cells.1 {
                for column in 0..cells.0 {
                    if xsize == 0 || ysize == 0 {
                        continue;
                    }
                    // Remainders of sizes that are `i32`.
                    let x = (start.0 + i64::from(column)).rem_euclid(xsize) as i32;
                    let y = (start.1 + i64::from(row)).rem_euclid(ysize) as i32;
                    let entry = table.get(x, y, 0).unwrap_or(0);
                    if entry > 0 && entry & 0xfff != 0 {
                        flashing.push((column, row, entry));
                    }
                }
            }
        }
        let wanted = !flashing.is_empty();
        let layer = &mut self.cache.flash;
        TileCache::prepare(layer, bitmaps, wanted, cells.0 * TILE, cells.1 * TILE);
        if let (Some(bitmap), Some(key)) = (self.cache.flash, &key) {
            for (column, row, entry) in flashing {
                // Four bits each, times the pulse: 120 at most. At full alpha the additive
                // blend adds just that.
                let multiplier = i16::from(key.multiplier);
                let component = |shift: u32| f64::from((entry >> shift & 0xf) * multiplier);
                let color = Color::new(component(8), component(4), component(0), 255.0);
                let cell = Rect::new(column * TILE, row * TILE, TILE, TILE);
                // The bitmap was just made or cleared.
                let _ = bitmaps.fill_rect(bitmap, cell, color);
            }
        }
        self.cache.flash_key = key;
    }

    /// Finds what this frame draws of the tilemap in `level`, sends its tables where the
    /// compositor lacks them and brings the flash overlay up to date. An invisible tilemap
    /// gives the overlay back; one that has no `level`, because its viewport is not drawn,
    /// keeps it.
    pub(crate) fn refresh(
        &mut self,
        bitmaps: &mut Bitmaps,
        level: Option<&Level>,
        tables: &mut SentTables,
    ) {
        if self.rgss2.is_some() {
            self.refresh_planes(bitmaps, level, tables);
            return;
        }
        if let Some(map) = &self.map_data {
            tables.hold(map, Role::Map);
        }
        if let Some(priorities) = &self.priorities {
            tables.hold(priorities, Role::Priorities);
        }
        if !self.visible {
            self.cache.dispose(bitmaps);
            return;
        }
        let Some(level) = level else {
            self.cache.view = None;
            return;
        };
        let Some(map) = self.map_data.clone() else {
            self.cache.dispose(bitmaps);
            return;
        };
        let map_table = map.read();
        let (xsize, ysize) = (map_table.xsize(), map_table.ysize());
        if xsize == 0 || ysize == 0 {
            self.cache.dispose(bitmaps);
            return;
        }

        // The pixel of the map under the top-left pixel of the area, and from it the cells
        // the flash overlay covers, which are those the area can show while the origin moves
        // within a tile: the area rounded up to whole tiles and one more.
        let clip = level.clip;
        let tile = i64::from(TILE);
        let left = i64::from(clip.x) - level.dx + i64::from(self.ox);
        let top = i64::from(clip.y) - level.dy + i64::from(self.oy);
        let first = (left.div_euclid(tile), top.div_euclid(tile));
        // The clip lies on the screen, so these are a few hundred at most.
        let cells = (
            clip.width.div_ceil(TILE as u32) as i32 + 1,
            clip.height.div_ceil(TILE as u32) as i32 + 1,
        );
        let start = (
            first.0.rem_euclid(i64::from(xsize)),
            first.1.rem_euclid(i64::from(ysize)),
        );
        self.refresh_flash(bitmaps, start, cells);

        let live = |id: Option<BitmapId>| {
            let rect = bitmaps.rect(id?).ok()?;
            // Sizes of a bitmap are not negative.
            let size = Size::new(rect.width.unsigned_abs(), rect.height.unsigned_abs());
            Some((id?, size))
        };
        let tileset = live(self.tileset);
        let autotiles = self.autotiles.map(live);
        let animation_step = self.updates / ANIMATION_PERIOD;

        // The cells that show: those the area reaches as the origin stands.
        let shown = |start: i64, length: u32| {
            ((start.rem_euclid(tile) + i64::from(length) + tile - 1) / tile) as i32
        };
        let (columns, shown_rows) = (shown(left, clip.width), shown(top, clip.height));
        // As measured on the original runtime, the map repeats beyond its edges in both
        // directions, tiles with a priority included.
        let priorities = self.priorities.as_ref().map(|table| table.read());
        let mut ground = [false; LAYERS];
        let mut rows: [[Vec<i32>; LAYERS]; MAX_PRIORITY] = Default::default();
        for row in 0..shown_rows {
            // Remainders of sizes that are `i32`.
            let y = (first.1 + i64::from(row)).rem_euclid(i64::from(ysize)) as i32;
            for column in 0..columns {
                let x = (first.0 + i64::from(column)).rem_euclid(i64::from(xsize)) as i32;
                for (layer, ground) in ground.iter_mut().enumerate() {
                    let Some(id) = map_table.get(x, y, layer as i32) else {
                        continue;
                    };
                    let found = match tiles::sheet(id) {
                        None => None,
                        Some(Sheet::Tileset) => tileset,
                        Some(Sheet::Autotile(index)) => autotiles[index],
                    };
                    let Some((_, size)) = found else {
                        continue;
                    };
                    if !shows(id, size, animation_step) {
                        continue;
                    }
                    match priority(priorities.as_deref(), id) {
                        None => {}
                        Some(0) => *ground = true,
                        Some(priority) => {
                            // Cells come row by row.
                            let rows = &mut rows[priority - 1][layer];
                            if rows.last() != Some(&row) {
                                rows.push(row);
                            }
                        }
                    }
                }
            }
        }
        drop(priorities);
        drop(map_table);

        let drawn = ground.contains(&true) || rows.iter().flatten().any(|rows| !rows.is_empty());
        let sent = drawn.then(|| {
            let priorities = match &self.priorities {
                Some(priorities) => tables.current(priorities, Role::Priorities),
                None => tables.empty(),
            };
            (tables.current(&map, Role::Map), priorities)
        });
        self.cache.view = Some(View {
            first,
            cells,
            period: (i64::from(xsize) * tile, i64::from(ysize) * tile),
            tables: sent,
            tileset: tileset.map(|(id, _)| id),
            autotiles: autotiles.map(|sheet| sheet.map(|(id, _)| id)),
            animation_step,
            ground,
            rows,
        });
    }

    /// The pieces this frame draws the tilemap with, each with its z.
    ///
    /// As measured on the original runtime, the z of the tiles of priority 1 to 5 is the y
    /// of their row's top edge, 32 times the row less the tilemap's origin, plus 32 for each
    /// step of priority and 32 more; neither the origin nor the rectangle of the viewport
    /// enters.
    pub(crate) fn strips(&self) -> Vec<(i64, Strip)> {
        if self.rgss2.is_some() {
            return self.plane_strips();
        }
        let Some(view) = self.cache.view.as_ref().filter(|_| self.visible) else {
            return Vec::new();
        };
        let top = view.first.1 * i64::from(TILE) - i64::from(self.oy);
        let mut strips = vec![(0, Strip::Ground)];
        // In the same place of the order as the ground, and after it.
        if self.cache.flash.is_some() {
            strips.push((0, Strip::Flash));
        }
        for priority in 1..=MAX_PRIORITY {
            // The rows that hold a tile of this priority in any layer of the map.
            let mut rows: Vec<i32> = view.rows[priority - 1].concat();
            rows.sort_unstable();
            rows.dedup();
            strips.extend(rows.into_iter().map(|row| {
                let y = top + i64::from(row) * i64::from(TILE);
                let z = y + (priority as i64 + 1) * i64::from(TILE);
                (z, Strip::Row { priority, row })
            }));
        }
        strips
    }
}

/// One piece of a tilemap that takes a place of its own in the draw order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Strip {
    /// Every tile of priority 0, layer by layer of the map.
    Ground,
    /// The colours of the flashing cells.
    Flash,
    /// The tiles of one priority from 1 on in one row of cells, counted from the first row
    /// that shows, layer by layer of the map.
    Row { priority: usize, row: i32 },
    /// One plane of a tilemap of RGSS2.
    Plane(vkxp_proto::Plane),
}

impl Assembly<'_> {
    /// One piece of a tilemap: one tilemap layer op for each layer of the map that holds a
    /// tile of the piece, in the order of the layers, so that every tile is blended onto the
    /// frame by itself.
    pub(crate) fn tiles(&mut self, tilemap: &Tilemap, strip: Strip, level: &Level) {
        if let Strip::Plane(plane) = strip {
            self.tilemap_plane(tilemap, plane, level);
            return;
        }
        let Some(view) = tilemap.cache.view.as_ref().filter(|_| tilemap.visible) else {
            return;
        };
        let tile = i64::from(TILE);
        // Where the first cell is on the screen: at most a tile left of and above the clip.
        let x = level.dx + view.first.0 * tile - i64::from(tilemap.ox);
        let y = level.dy + view.first.1 * tile - i64::from(tilemap.oy);
        match strip {
            Strip::Ground => {
                for layer in (0..LAYERS).filter(|&layer| view.ground[layer]) {
                    self.tilemap_layer(tilemap, view, level, level.clip, layer, 0);
                }
            }
            Strip::Flash => {
                if let Some(bitmap) = tilemap.cache.flash {
                    let look = Look {
                        blend: BlendMode::Add,
                        ..Look::plain(255)
                    };
                    let src = Rect::new(0, 0, view.cells.0 * TILE, view.cells.1 * TILE);
                    self.quad(bitmap, src, &Geometry::at(x, y), level.clip, look);
                }
            }
            Strip::Plane(_) => {}
            Strip::Row { priority, row } => {
                let top = y + i64::from(row) * tile;
                let band = rows(level.clip, top, top + tile);
                for (layer, rows) in view.rows[priority - 1].iter().enumerate() {
                    if rows.binary_search(&row).is_ok() {
                        self.tilemap_layer(tilemap, view, level, band, layer, priority);
                    }
                }
            }
        }
    }

    /// The op that draws the tiles of `priority` of `layer` of the map inside `clip`.
    fn tilemap_layer(
        &mut self,
        tilemap: &Tilemap,
        view: &View,
        level: &Level,
        clip: vkxp_proto::Rect,
        layer: usize,
        priority: usize,
    ) {
        let Some((map, priorities)) = view.tables.filter(|_| !clip.is_empty()) else {
            return;
        };
        // The map pixel under the top-left pixel of the screen. The map repeats, so an
        // origin that `i32` does not hold is replaced by the one inside the map that shows
        // the same.
        let along = |origin: i32, offset: i64, period: i64| {
            let origin = i64::from(origin) - offset;
            i32::try_from(origin)
                .or_else(|_| i32::try_from(origin.rem_euclid(period)))
                .unwrap_or(i32::MAX)
        };
        let sheets = view.autotiles.into_iter().chain([view.tileset]);
        self.referenced.extend(sheets.flatten());
        self.ops.push(Op::TilemapLayer(TilemapLayer {
            clip,
            tileset: view.tileset,
            autotiles: view.autotiles,
            map,
            priorities,
            origin: (
                along(tilemap.ox, level.dx, view.period.0),
                along(tilemap.oy, level.dy, view.period.1),
            ),
            // Below 3 and at most 5.
            layer: layer as u8,
            priority: priority as u8,
            animation_step: view.animation_step,
        }));
    }
}

mod rgss2;

#[cfg(test)]
mod tests;
