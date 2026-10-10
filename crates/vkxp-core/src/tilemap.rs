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
//! priorities are sent brought into 0 to 5, which is all an op tells apart.
//!
//! The cells the flash data marks are filled with their colour, at the alpha of the pulse,
//! in a bitmap of the scene's own that covers the visible cells and one more column and row,
//! which is drawn additively right above the ground. It is filled again when the flash data,
//! the first cell or the pulse changes.

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
/// Number of updates one pulse of the flash data takes.
const FLASH_PERIOD: u64 = 32;

/// What a table is to the tilemap it was sent for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// The tile numbers, sent as they are.
    Map,
    /// The priorities, of which the first row is sent, brought into 0 to 5.
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
                let data: Vec<i16> = entries
                    .map(|entry| entry.clamp(0, MAX_PRIORITY as i16))
                    .collect();
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
    first: (i64, i64),
    cells: (i32, i32),
    table: (SharedTable, u64),
    alpha: u8,
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

/// The priority of tile `id`: the entry of the priorities table, 0 without one.
///
/// PLACEHOLDER: what the original runtime does with a priority outside 0 to 5 is not yet
/// checked; here it is brought into that range.
fn priority(priorities: Option<&Table>, id: i16) -> usize {
    let value = priorities.and_then(|table| table.get(i32::from(id), 0, 0));
    value.unwrap_or(0).clamp(0, MAX_PRIORITY as i16) as usize
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

/// The alpha the flashing cells are drawn with after `updates` updates: it rises from 0 to
/// 255 in 16 updates and falls back in 16 more.
///
/// PLACEHOLDER: curve, period and peak of the pulse are not yet measured on the original
/// runtime; neither is how the overlay is blended.
fn flash_alpha(updates: u64) -> u8 {
    let step = updates % FLASH_PERIOD;
    let strength = step.min(FLASH_PERIOD - step);
    // At most 255, at the middle of the period.
    (strength * 255 / (FLASH_PERIOD / 2)) as u8
}

impl Tilemap {
    /// Brings the flash overlay up to date for the `cells` covered from cell `first` on.
    fn refresh_flash(&mut self, bitmaps: &mut Bitmaps, first: (i64, i64), cells: (i32, i32)) {
        let key = self.flash_data.as_ref().map(|table| FlashKey {
            first,
            cells,
            table: (table.clone(), table.revision()),
            alpha: flash_alpha(self.updates),
        });
        let lost = self.cache.flash.is_some_and(|id| bitmaps.is_disposed(id));
        if self.cache.flash_key == key && !lost {
            return;
        }

        // The flash data repeats beyond its edges as the map does.
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
                    let x = (first.0 + i64::from(column)).rem_euclid(xsize) as i32;
                    let y = (first.1 + i64::from(row)).rem_euclid(ysize) as i32;
                    let entry = table.get(x, y, 0).unwrap_or(0);
                    if entry != 0 {
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
                // Four bits each, which 17 spreads over 0 to 255.
                let component = |shift: u32| f64::from((entry >> shift & 0xf) * 17);
                let color = Color::new(
                    component(8),
                    component(4),
                    component(0),
                    f64::from(key.alpha),
                );
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
        self.refresh_flash(bitmaps, first, cells);

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
        // PLACEHOLDER: that the map repeats beyond its edges is assumed, not yet checked
        // against the original runtime.
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
                        0 => *ground = true,
                        priority => {
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

    /// The pieces this frame draws the tilemap with, each with its z. `scroll` is the vertical
    /// origin of the tilemap's viewport, 0 without one: the top edge of a row is where it is
    /// drawn in the viewport, which both origins move.
    ///
    /// PLACEHOLDER: the z of the tiles of priority 1 to 5, the y of their top edge in the
    /// viewport plus 32 for each step of priority and 32 more, is not yet checked against the
    /// original runtime; neither is that a sprite created after the tilemap wins a tie.
    pub(crate) fn strips(&self, scroll: i64) -> Vec<(i64, Strip)> {
        let Some(view) = self.cache.view.as_ref().filter(|_| self.visible) else {
            return Vec::new();
        };
        let top = view.first.1 * i64::from(TILE) - i64::from(self.oy) - scroll;
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
}

impl Assembly<'_> {
    /// One piece of a tilemap: one tilemap layer op for each layer of the map that holds a
    /// tile of the piece, in the order of the layers, so that every tile is blended onto the
    /// frame by itself.
    pub(crate) fn tiles(&mut self, tilemap: &Tilemap, strip: Strip, level: &Level) {
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

#[cfg(test)]
mod tests {
    use vkxp_compose_ref::Frame;
    use vkxp_pixel::Rgba;
    use vkxp_proto::FramePacket;

    use super::*;
    use crate::bitmap::testing::color;
    use crate::extract::testing::*;
    use crate::{SpriteId, TilemapId};
    use vkxp_proto::tiles::AUTOTILE_QUARTERS;

    /// Number of tiles of the tileset of the tests: 8 columns, 40 rows.
    const TILES: i32 = 320;

    /// The pixel at (`x`, `y`) of the tileset: it names its tile and its place in the tile.
    fn tileset_pixel(x: u32, y: u32) -> Rgba {
        let tile = x / 32 + y / 32 * 8;
        [
            (tile % 251) as u8 + 1,
            (x % 32) as u8 * 4 + (tile / 251) as u8,
            (y % 32) as u8,
            255,
        ]
    }

    /// Pixel (`x`, `y`) of tile `id` of the tileset.
    fn tile_pixel(id: i32, x: u32, y: u32) -> Rgba {
        let index = (id - 384) as u32;
        tileset_pixel(index % 8 * 32 + x, index / 8 * 32 + y)
    }

    fn painted(
        stage: &mut Stage,
        width: u32,
        height: u32,
        pixel: fn(u32, u32) -> Rgba,
    ) -> BitmapId {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&pixel(x, y));
            }
        }
        stage.bitmap(width, height, rgba)
    }

    fn tileset(stage: &mut Stage) -> BitmapId {
        painted(stage, 256, TILES as u32 / 8 * 32, tileset_pixel)
    }

    /// The pixel at (`x`, `y`) of an autotile sheet: it names its frame, its quarter and its
    /// place in the quarter.
    fn autotile_pixel(x: u32, y: u32) -> Rgba {
        let quarter = x % 96 / 16 + y / 16 * 6;
        [
            quarter as u8 + 1,
            (x % 16) as u8 * 16 + (y % 16) as u8,
            (x / 96) as u8 + 100,
            255,
        ]
    }

    /// A map of `width` by `height` cells, empty.
    fn map(width: i32, height: i32) -> SharedTable {
        SharedTable::new(Table::new_3d(width, height, 3).unwrap())
    }

    /// The tile the lowest layer of a [`full_map`] holds in cell (`x`, `y`).
    fn cell_tile(x: i32, y: i32) -> i32 {
        384 + (x + y * 20) % TILES
    }

    /// A map of 20 by 15 cells in which every cell of the lowest layer holds another tile.
    fn full_map() -> SharedTable {
        let map = map(20, 15);
        for y in 0..15 {
            for x in 0..20 {
                map.set(x, y, 0, i64::from(cell_tile(x, y)));
            }
        }
        map
    }

    /// A tilemap on the screen with a tileset and `map`.
    fn tilemap(stage: &mut Stage, map: &SharedTable) -> TilemapId {
        let tileset = tileset(stage);
        let id = stage.scene.create_tilemap(None).unwrap();
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.map_data = Some(map.clone());
        id
    }

    /// Whether the frame shows a [`full_map`] scrolled to (`ox`, `oy`), inside the rectangle
    /// at (`left`, `top`) of `width` by `height` pixels, and black elsewhere.
    fn shows_full_map(frame: &Frame, (ox, oy): (i32, i32), area: Rect) -> bool {
        (0..frame.size.height as i32).all(|y| {
            (0..frame.size.width as i32).all(|x| {
                let inside = (area.x..area.x + area.width).contains(&x)
                    && (area.y..area.y + area.height).contains(&y);
                let expected = if inside {
                    let (map_x, map_y) = (x - area.x + ox, y - area.y + oy);
                    let cell = (
                        map_x.div_euclid(32).rem_euclid(20),
                        map_y.div_euclid(32).rem_euclid(15),
                    );
                    let within = (map_x.rem_euclid(32) as u32, map_y.rem_euclid(32) as u32);
                    tile_pixel(cell_tile(cell.0, cell.1), within.0, within.1)
                } else {
                    BLACK
                };
                frame.pixel(x as u32, y as u32) == expected
            })
        })
    }

    const SCREEN: Rect = Rect::new(0, 0, 640, 480);

    /// Whether the 32 by 32 pixels at (`x`, `y`) of the frame are `expected` of their place.
    fn tile_shows(frame: &Frame, x: u32, y: u32, expected: impl Fn(u32, u32) -> Rgba) -> bool {
        (0..32).all(|py| (0..32).all(|px| frame.pixel(x + px, y + py) == expected(px, py)))
    }

    #[test]
    fn a_tileset_tile_shows_its_32_by_32_pixels_of_the_tileset() {
        let mut stage = Stage::sized(96, 64);
        let map = map(3, 2);
        map.set(1, 1, 0, 384 + 8 + 3);
        tilemap(&mut stage, &map);
        let frame = stage.frame();
        assert!(tile_shows(&frame, 32, 32, |x, y| tileset_pixel(
            96 + x,
            32 + y
        )));
        // The cells that hold 0 show nothing.
        assert!(tile_shows(&frame, 0, 0, |_, _| BLACK));
        assert!(tile_shows(&frame, 64, 32, |_, _| BLACK));
        assert!(tile_shows(&frame, 32, 0, |_, _| BLACK));
        stage.assert_pinned("a_tileset_tile_shows_its_32_by_32_pixels_of_the_tileset");
    }

    #[test]
    fn identifiers_below_48_and_tiles_outside_the_tileset_draw_nothing() {
        let mut stage = Stage::sized(96, 64);
        let map = map(3, 2);
        map.set(0, 0, 0, 1);
        map.set(1, 0, 1, 47);
        map.set(2, 0, 2, -5);
        map.set(0, 1, 0, i64::from(384 + TILES));
        map.set(1, 1, 0, i64::from(i16::MAX));
        tilemap(&mut stage, &map);
        let before = stage.bitmaps.len();
        assert!(all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), before, "nothing to cache");
        stage.assert_pinned("identifiers_below_48_and_tiles_outside_the_tileset_draw_nothing");
    }

    #[test]
    fn a_tilemap_without_a_live_sheet_draws_nothing_for_its_tiles() {
        let mut stage = Stage::sized(96, 64);
        let map = map(3, 2);
        map.set(0, 0, 0, 384);
        map.set(1, 0, 0, 48);
        let id = stage.scene.create_tilemap(None).unwrap();
        stage.scene.tilemap_mut(id).unwrap().map_data = Some(map.clone());
        assert!(all(&stage.frame(), BLACK));

        // With the sheets the tiles appear, and without them they go again.
        let tileset = tileset(&mut stage);
        let autotile = painted(&mut stage, 96, 128, autotile_pixel);
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.autotiles[0] = Some(autotile);
        let frame = stage.frame();
        assert!(tile_shows(&frame, 0, 0, |x, y| tile_pixel(384, x, y)));
        assert_ne!(frame.pixel(32, 0), BLACK);
        stage.bitmaps.dispose(tileset);
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), BLACK);
        assert_ne!(frame.pixel(32, 0), BLACK);
        stage.bitmaps.dispose(autotile);
        assert!(all(&stage.frame(), BLACK));
        stage.assert_pinned("a_tilemap_without_a_live_sheet_draws_nothing_for_its_tiles");
    }

    #[test]
    fn a_tilemap_without_map_data_draws_nothing() {
        let mut stage = Stage::sized(96, 64);
        let id = tilemap(&mut stage, &map(0, 0));
        assert!(all(&stage.frame(), BLACK));
        stage.scene.tilemap_mut(id).unwrap().map_data = None;
        assert!(all(&stage.frame(), BLACK));
        stage.assert_pinned("a_tilemap_without_map_data_draws_nothing");
    }

    /// A stage of 8 by 6 tiles showing the 48 patterns of autotile `index`, pattern `p` in
    /// cell (`p mod 8`, `p div 8`), from `sheet`.
    fn patterns(sheet_width: u32, sheet_height: u32, index: usize) -> (Stage, TilemapId) {
        let mut stage = Stage::sized(256, 192);
        let map = map(8, 6);
        for pattern in 0..48 {
            let id = 48 * (index as i32 + 1) + pattern;
            map.set(pattern % 8, pattern / 8, 0, i64::from(id));
        }
        let sheet = painted(&mut stage, sheet_width, sheet_height, autotile_pixel);
        let id = stage.scene.create_tilemap(None).unwrap();
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.autotiles[index] = Some(sheet);
        tilemap.map_data = Some(map);
        (stage, id)
    }

    /// Whether the frame of a [`patterns`] stage shows every pattern assembled from the
    /// quarters the table names, taken from animation frame `shown` of the sheet.
    fn shows_patterns(frame: &Frame, shown: u32) -> bool {
        (0..48u32).all(|pattern| {
            let (left, top) = (pattern % 8 * 32, pattern / 8 * 32);
            (0..4u32).all(|corner| {
                let quarter = u32::from(AUTOTILE_QUARTERS[pattern as usize][corner as usize]) - 1;
                let (sheet_x, sheet_y) = (shown * 96 + quarter % 6 * 16, quarter / 6 * 16);
                let (x, y) = (left + corner % 2 * 16, top + corner / 2 * 16);
                (0..16).all(|py| {
                    (0..16).all(|px| {
                        frame.pixel(x + px, y + py) == autotile_pixel(sheet_x + px, sheet_y + py)
                    })
                })
            })
        })
    }

    #[test]
    fn every_autotile_pattern_is_assembled_from_the_quarters_the_table_names() {
        let (mut stage, _) = patterns(96, 128, 0);
        let frame = stage.frame();
        assert!(shows_patterns(&frame, 0));

        // The enclosed pattern, 0, and the isolated one, 47, spelled out.
        for (pattern, quarters) in [
            (0, [(32, 64), (48, 64), (32, 80), (48, 80)]),
            (47, [(0, 0), (16, 0), (0, 16), (16, 16)]),
        ] {
            let (left, top) = (pattern % 8 * 32, pattern / 8 * 32);
            for (corner, (sheet_x, sheet_y)) in quarters.into_iter().enumerate() {
                let (x, y) = (left + corner as u32 % 2 * 16, top + corner as u32 / 2 * 16);
                assert_eq!(frame.pixel(x, y), autotile_pixel(sheet_x, sheet_y));
                assert_eq!(
                    frame.pixel(x + 15, y + 15),
                    autotile_pixel(sheet_x + 15, sheet_y + 15)
                );
            }
        }
        stage
            .assert_pinned("every_autotile_pattern_is_assembled_from_the_quarters_the_table_names");
    }

    #[test]
    fn each_of_the_seven_autotiles_has_its_own_sheet() {
        for index in [1, 6] {
            let (mut stage, _) = patterns(96, 128, index);
            assert!(shows_patterns(&stage.frame(), 0), "autotile {index}");
            stage.assert_pinned(&format!(
                "each_of_the_seven_autotiles_has_its_own_sheet_{index}"
            ));
        }
    }

    #[test]
    fn a_sheet_32_pixels_high_shows_the_whole_sheet_for_every_pattern() {
        let (mut stage, _) = patterns(32, 32, 0);
        let frame = stage.frame();
        for pattern in 0..48 {
            assert!(
                tile_shows(&frame, pattern % 8 * 32, pattern / 8 * 32, autotile_pixel),
                "pattern {pattern}"
            );
        }
        stage.assert_pinned("a_sheet_32_pixels_high_shows_the_whole_sheet_for_every_pattern");
    }

    #[test]
    fn an_autotile_shows_its_next_frame_every_16_updates() {
        let (mut stage, id) = patterns(384, 128, 0);
        assert!(shows_patterns(&stage.frame(), 0));
        let mut updates = 0;
        for (after, shown) in [
            (15, 0),
            (16, 1),
            (31, 1),
            (32, 2),
            (48, 3),
            (63, 3),
            (64, 0),
        ] {
            while updates < after {
                stage.scene.tilemap_mut(id).unwrap().update();
                updates += 1;
            }
            assert!(
                shows_patterns(&stage.frame(), shown),
                "after {after} updates"
            );
        }
        stage.assert_pinned("an_autotile_shows_its_next_frame_every_16_updates");
    }

    #[test]
    fn a_single_tile_sheet_is_animated_in_frames_of_32_pixels() {
        let (mut stage, id) = patterns(96, 32, 0);
        for shown in [0, 1, 2, 0] {
            let frame = stage.frame();
            assert!(tile_shows(&frame, 64, 32, |x, y| autotile_pixel(
                shown * 32 + x,
                y
            )));
            for _ in 0..16 {
                stage.scene.tilemap_mut(id).unwrap().update();
            }
        }
        stage.assert_pinned("a_single_tile_sheet_is_animated_in_frames_of_32_pixels");
    }

    #[test]
    fn without_updates_the_animation_stands_still() {
        let (mut stage, id) = patterns(384, 128, 0);
        for _ in 0..16 {
            stage.scene.tilemap_mut(id).unwrap().update();
        }
        let first = stage.scene.extract(&mut stage.bitmaps);
        assert!(shows_patterns(&stage.render(&first).unwrap(), 1));
        for _ in 0..40 {
            let packet = stage.scene.extract(&mut stage.bitmaps);
            assert_eq!(packet.bitmaps, [], "nothing is composed again");
            assert_eq!(packet.ops, first.ops);
            assert!(shows_patterns(&stage.render(&packet).unwrap(), 1));
        }
    }

    #[test]
    fn cell_x_y_is_drawn_at_32_times_its_coordinates() {
        let mut stage = Stage::new();
        tilemap(&mut stage, &full_map());
        let frame = stage.frame();
        assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
            cell_tile(3, 2),
            x,
            y
        )));
        assert!(shows_full_map(&frame, (0, 0), SCREEN));
        stage.assert_pinned("cell_x_y_is_drawn_at_32_times_its_coordinates");
    }

    #[test]
    fn the_origin_scrolls_the_map_by_pixels() {
        let mut stage = Stage::new();
        let id = tilemap(&mut stage, &full_map());
        stage.scene.tilemap_mut(id).unwrap().ox = 40;
        let frame = stage.frame();
        // Cell (1, 0) starts 8 pixels left of the screen, cell (2, 0) at x 24.
        assert_eq!(frame.pixel(0, 0), tile_pixel(cell_tile(1, 0), 8, 0));
        assert_eq!(frame.pixel(23, 0), tile_pixel(cell_tile(1, 0), 31, 0));
        assert!(tile_shows(&frame, 24, 0, |x, y| tile_pixel(
            cell_tile(2, 0),
            x,
            y
        )));
        assert!(shows_full_map(&frame, (40, 0), SCREEN));

        for origin in [
            (0, 0),
            (31, 31),
            (32, 32),
            (-1, -1),
            (-33, 7),
            (1000, -1000),
        ] {
            let tilemap = stage.scene.tilemap_mut(id).unwrap();
            (tilemap.ox, tilemap.oy) = origin;
            assert!(shows_full_map(&stage.frame(), origin, SCREEN), "{origin:?}");
        }
        stage.assert_pinned("the_origin_scrolls_the_map_by_pixels");
    }

    #[test]
    fn an_origin_at_the_limits_of_its_type_is_drawn() {
        let mut stage = Stage::sized(64, 64);
        let id = tilemap(&mut stage, &full_map());
        for origin in [(i32::MAX, i32::MIN), (i32::MIN, i32::MAX)] {
            let tilemap = stage.scene.tilemap_mut(id).unwrap();
            (tilemap.ox, tilemap.oy) = origin;
            let frame = stage.frame();
            let area = Rect::new(0, 0, 64, 64);
            let wrapped = (origin.0.rem_euclid(640), origin.1.rem_euclid(480));
            assert!(shows_full_map(&frame, wrapped, area), "{origin:?}");
        }
        stage.assert_pinned("an_origin_at_the_limits_of_its_type_is_drawn");
    }

    #[test]
    fn scrolling_uploads_nothing() {
        let mut stage = Stage::new();
        let id = tilemap(&mut stage, &full_map());
        let autotile = painted(&mut stage, 384, 128, autotile_pixel);
        let priorities = SharedTable::new(Table::new(1000).unwrap());
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.autotiles[0] = Some(autotile);
        tilemap.priorities = Some(priorities);
        // The packet of the frame after the origin is set to `ox`.
        let mut packet = |ox: i32| {
            stage.scene.tilemap_mut(id).unwrap().ox = ox;
            let packet = stage.scene.extract(&mut stage.bitmaps);
            let frame = stage.render(&packet).unwrap();
            assert!(shows_full_map(&frame, (ox, 0), SCREEN), "{ox}");
            packet
        };
        let first = packet(0);
        assert_eq!(first.tables.len(), 2, "tile numbers and priorities");
        assert!(!first.bitmaps.is_empty(), "the sheets");
        // 100 pixels in ten frames, across three tile boundaries.
        for frame in 1..=10 {
            let packet = packet(frame * 10);
            assert_eq!(packet.tables, [], "frame {frame}");
            assert_eq!(packet.bitmaps, [], "frame {frame}");
        }
    }

    #[test]
    fn the_upper_layer_covers_the_lower_one() {
        let mut stage = Stage::sized(96, 64);
        let map = map(3, 2);
        map.set(1, 0, 0, 384 + 1);
        map.set(1, 0, 2, 384 + 2);
        map.set(2, 1, 0, 384 + 3);
        map.set(2, 1, 1, 384 + 4);
        map.set(2, 1, 2, 384 + 5);
        map.set(0, 1, 1, 384 + 6);
        tilemap(&mut stage, &map);
        let frame = stage.frame();
        assert!(tile_shows(&frame, 32, 0, |x, y| tile_pixel(384 + 2, x, y)));
        assert!(tile_shows(&frame, 64, 32, |x, y| tile_pixel(384 + 5, x, y)));
        assert!(tile_shows(&frame, 0, 32, |x, y| tile_pixel(384 + 6, x, y)));
        stage.assert_pinned("the_upper_layer_covers_the_lower_one");
    }

    #[test]
    fn an_upper_tile_lets_the_lower_one_show_through_its_transparent_pixels() {
        let mut stage = Stage::sized(32, 32);
        let map = map(1, 1);
        map.set(0, 0, 0, 384);
        map.set(0, 0, 1, 385);
        let id = tilemap(&mut stage, &map);
        let tileset = stage.scene.tilemap(id).unwrap().tileset.unwrap();
        let hole = Rect::new(32 + 8, 8, 16, 16);
        stage
            .bitmaps
            .fill_rect(tileset, hole, color(CLEAR))
            .unwrap();
        let frame = stage.frame();
        assert!(tile_shows(&frame, 0, 0, |x, y| {
            let upper = !((8..24).contains(&x) && (8..24).contains(&y));
            tile_pixel(if upper { 385 } else { 384 }, x, y)
        }));
        stage.assert_pinned("an_upper_tile_lets_the_lower_one_show_through_its_transparent_pixels");
    }

    #[test]
    fn the_map_repeats_beyond_its_edges() {
        let mut stage = Stage::new();
        let id = tilemap(&mut stage, &full_map());
        stage.scene.tilemap_mut(id).unwrap().ox = 320;
        let frame = stage.frame();
        // The right half of the screen shows columns 0 to 9.
        for column in 0..10 {
            assert!(
                tile_shows(&frame, 320 + column * 32, 64, |x, y| {
                    tile_pixel(cell_tile(column as i32, 2), x, y)
                }),
                "column {column}"
            );
        }
        assert!(shows_full_map(&frame, (320, 0), SCREEN));
        stage.assert_pinned("the_map_repeats_beyond_its_edges");
    }

    #[test]
    fn a_tilemap_in_a_viewport_is_drawn_inside_its_rectangle() {
        let mut stage = Stage::new();
        let area = Rect::new(100, 50, 200, 100);
        let viewport = stage.scene.create_viewport(area).unwrap();
        let tileset = tileset(&mut stage);
        let id = stage.scene.create_tilemap(Some(viewport)).unwrap();
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.map_data = Some(full_map());
        assert!(shows_full_map(&stage.frame(), (0, 0), area));

        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        (tilemap.ox, tilemap.oy) = (45, -7);
        assert!(shows_full_map(&stage.frame(), (45, -7), area));

        // The origin of the viewport scrolls the map like every element.
        stage.scene.viewport_mut(viewport).unwrap().ox = 20;
        assert!(shows_full_map(&stage.frame(), (65, -7), area));

        // A viewport that reaches off the screen shows the part that is on it.
        let viewport = stage.scene.viewport_mut(viewport).unwrap();
        viewport.ox = 0;
        viewport.rect = Rect::new(-50, -20, 200, 100);
        let frame = stage.frame();
        assert!(shows_full_map(&frame, (95, 13), Rect::new(0, 0, 150, 80)));
        stage.assert_pinned("a_tilemap_in_a_viewport_is_drawn_inside_its_rectangle");
    }

    /// The stage of the priority scenarios: a tile of `priority` in cell (2, 3), whose top
    /// edge is at y 96, a tile of priority 0 next to it in cell (3, 3), and a blue sprite
    /// over both, created before the tilemap.
    fn with_priority(priority: i64) -> (Stage, SpriteId, TilemapId) {
        let mut stage = Stage::sized(160, 160);
        let blue = stage.solid(64, 32, BLUE);
        let sprite = stage.scene.create_sprite(None).unwrap();
        let sprite_mut = stage.scene.sprite_mut(sprite).unwrap();
        sprite_mut.set_bitmap(Some(blue), &stage.bitmaps);
        (sprite_mut.x, sprite_mut.y) = (64, 96);

        let map = map(5, 5);
        map.set(2, 3, 0, 400);
        map.set(3, 3, 0, 384);
        let priorities = SharedTable::new(Table::new(500).unwrap());
        priorities.set(400, 0, 0, priority);
        let id = tilemap(&mut stage, &map);
        stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities);
        (stage, sprite, id)
    }

    /// Whether the tile of priority, and the ground tile, are in front of the sprite.
    fn in_front(stage: &mut Stage) -> (bool, bool) {
        let frame = stage.frame();
        let tile = tile_shows(&frame, 64, 96, |x, y| tile_pixel(400, x, y));
        let ground = tile_shows(&frame, 96, 96, |x, y| tile_pixel(384, x, y));
        assert!(tile || tile_shows(&frame, 64, 96, |_, _| BLUE));
        assert!(ground || tile_shows(&frame, 96, 96, |_, _| BLUE));
        (tile, ground)
    }

    #[test]
    fn tiles_of_priority_0_are_drawn_at_z_0() {
        let (mut stage, sprite, _) = with_priority(0);
        assert_eq!(in_front(&mut stage), (true, true), "the tilemap is newer");
        stage.scene.set_sprite_z(sprite, 1).unwrap();
        assert_eq!(in_front(&mut stage), (false, false));
        stage.scene.set_sprite_z(sprite, -1).unwrap();
        assert_eq!(in_front(&mut stage), (true, true));
        stage.assert_pinned("tiles_of_priority_0_are_drawn_at_z_0");
    }

    #[test]
    fn a_tile_of_priority_1_ranks_with_the_sprites_one_row_below_it() {
        let (mut stage, sprite, _) = with_priority(1);
        stage.scene.set_sprite_z(sprite, 159).unwrap();
        assert_eq!(in_front(&mut stage), (true, false));
        stage.scene.set_sprite_z(sprite, 192).unwrap();
        assert_eq!(in_front(&mut stage), (false, false));
        // At the z of the tile the newer of the two is in front: here the sprite, which was
        // given its z after the tilemap was created.
        stage.scene.set_sprite_z(sprite, 160).unwrap();
        assert_eq!(in_front(&mut stage), (false, false));
        stage.scene.set_sprite_z(sprite, 161).unwrap();
        assert_eq!(in_front(&mut stage), (false, false));
        stage.assert_pinned("a_tile_of_priority_1_ranks_with_the_sprites_one_row_below_it");
    }

    #[test]
    fn a_tile_of_a_higher_priority_ranks_32_higher_for_each_step() {
        let (mut stage, sprite, _) = with_priority(2);
        stage.scene.set_sprite_z(sprite, 191).unwrap();
        assert_eq!(in_front(&mut stage), (true, false));
        stage.scene.set_sprite_z(sprite, 193).unwrap();
        assert_eq!(in_front(&mut stage), (false, false));

        for priority in 1..=5 {
            let (mut stage, sprite, _) = with_priority(priority);
            let z = 96 + (priority as i32 + 1) * 32;
            stage.scene.set_sprite_z(sprite, z - 1).unwrap();
            assert_eq!(in_front(&mut stage), (true, false), "priority {priority}");
            stage.scene.set_sprite_z(sprite, z + 1).unwrap();
            assert_eq!(in_front(&mut stage), (false, false), "priority {priority}");
            stage.assert_pinned(&format!(
                "a_tile_of_a_higher_priority_ranks_32_higher_for_each_step_{priority}"
            ));
        }
        stage.assert_pinned("a_tile_of_a_higher_priority_ranks_32_higher_for_each_step");
    }

    #[test]
    fn the_z_of_a_tile_follows_its_row_on_the_screen() {
        let (mut stage, sprite, id) = with_priority(1);
        // Scrolled up by a row and a half, the tile's top edge is at y 48.
        stage.scene.tilemap_mut(id).unwrap().oy = 48;
        stage.scene.sprite_mut(sprite).unwrap().y = 48;
        stage.scene.set_sprite_z(sprite, 48 + 64 - 1).unwrap();
        assert_eq!(in_front_at(&mut stage, 48), (true, false));
        stage.scene.set_sprite_z(sprite, 48 + 64 + 1).unwrap();
        assert_eq!(in_front_at(&mut stage, 48), (false, false));
        stage.assert_pinned("the_z_of_a_tile_follows_its_row_on_the_screen");
    }

    /// [`in_front`] for a tilemap scrolled so that the row of the tiles is at `y`.
    fn in_front_at(stage: &mut Stage, y: u32) -> (bool, bool) {
        let frame = stage.frame();
        (
            tile_shows(&frame, 64, y, |px, py| tile_pixel(400, px, py)),
            tile_shows(&frame, 96, y, |px, py| tile_pixel(384, px, py)),
        )
    }

    #[test]
    fn the_z_of_a_tile_follows_the_origin_of_its_viewport() {
        let mut stage = Stage::sized(160, 160);
        let viewport = stage.scene.create_viewport(Rect::new(0, 0, 160, 160));
        let viewport = viewport.unwrap();
        stage.scene.viewport_mut(viewport).unwrap().oy = 32;
        let blue = stage.solid(32, 32, BLUE);
        let sprite = stage.scene.create_sprite(Some(viewport)).unwrap();
        let sprite_mut = stage.scene.sprite_mut(sprite).unwrap();
        sprite_mut.set_bitmap(Some(blue), &stage.bitmaps);
        (sprite_mut.x, sprite_mut.y) = (64, 96);

        let map = map(5, 5);
        map.set(2, 3, 0, 400);
        let priorities = SharedTable::new(Table::new(500).unwrap());
        priorities.set(400, 0, 0, 1);
        let tileset = tileset(&mut stage);
        let id = stage.scene.create_tilemap(Some(viewport)).unwrap();
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.map_data = Some(map);
        tilemap.priorities = Some(priorities);

        // The viewport's origin moves the tile's top edge from y 96 to y 64 in the viewport,
        // so its z is 64 + 64. Tile and sprite are both drawn at (64, 64).
        let mut tile_in_front = |z: i32| {
            stage.scene.set_sprite_z(sprite, z).unwrap();
            let frame = stage.frame();
            let tile = tile_shows(&frame, 64, 64, |x, y| tile_pixel(400, x, y));
            assert!(tile || tile_shows(&frame, 64, 64, |_, _| BLUE));
            tile
        };
        assert!(!tile_in_front(144));
        assert!(!tile_in_front(129));
        assert!(tile_in_front(127));

        // Both origins count.
        stage.scene.tilemap_mut(id).unwrap().oy = -16;
        stage.scene.sprite_mut(sprite).unwrap().y = 112;
        let mut tile_in_front = |z: i32| {
            stage.scene.set_sprite_z(sprite, z).unwrap();
            let frame = stage.frame();
            tile_shows(&frame, 64, 80, |x, y| tile_pixel(400, x, y))
        };
        assert!(tile_in_front(80 + 64 - 1));
        assert!(!tile_in_front(80 + 64 + 1));
        stage.assert_pinned("the_z_of_a_tile_follows_the_origin_of_its_viewport");
    }

    #[test]
    fn an_invisible_tilemap_gives_its_bitmaps_back_also_in_a_hidden_viewport() {
        let mut stage = Stage::sized(160, 160);
        let viewport = stage.scene.create_viewport(Rect::new(0, 0, 160, 160));
        let viewport = viewport.unwrap();
        let map = map(5, 5);
        map.set(1, 1, 0, 384);
        let tileset = tileset(&mut stage);
        let id = stage.scene.create_tilemap(Some(viewport)).unwrap();
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.map_data = Some(map);
        // The one bitmap a tilemap has of its own is the overlay of its flashing cells.
        let flash = SharedTable::new(Table::new_2d(5, 5).unwrap());
        flash.set(1, 1, 0, 0xf00);
        tilemap.flash_data = Some(flash);
        let before = stage.bitmaps.len();
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before + 1);

        // A visible tilemap in a hidden viewport keeps its bitmaps.
        stage.scene.viewport_mut(viewport).unwrap().visible = false;
        assert!(all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), before + 1);

        // An invisible one gives them back, whatever its viewport: hidden or off the screen.
        stage.scene.tilemap_mut(id).unwrap().visible = false;
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before);

        stage.scene.viewport_mut(viewport).unwrap().visible = true;
        stage.scene.tilemap_mut(id).unwrap().visible = true;
        assert!(!all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), before + 1);
        stage.scene.viewport_mut(viewport).unwrap().rect = Rect::new(200, 0, 64, 64);
        stage.scene.tilemap_mut(id).unwrap().visible = false;
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before);

        stage.scene.viewport_mut(viewport).unwrap().rect = Rect::new(0, 0, 160, 160);
        stage.scene.tilemap_mut(id).unwrap().visible = true;
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before + 1);
        // A disposed viewport takes the tilemap along.
        stage.scene.dispose_viewport(viewport, &mut stage.bitmaps);
        assert!(stage.scene.is_tilemap_disposed(id));
        assert_eq!(stage.bitmaps.len(), before);
        assert!(all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), before);
        stage
            .assert_pinned("an_invisible_tilemap_gives_its_bitmaps_back_also_in_a_hidden_viewport");
    }

    #[test]
    fn a_tile_without_an_entry_in_the_priorities_has_priority_0() {
        let (mut stage, sprite, id) = with_priority(3);
        stage.scene.set_sprite_z(sprite, 1).unwrap();
        assert_eq!(in_front(&mut stage), (true, false));
        let short = SharedTable::new(Table::new(390).unwrap());
        short.set(384, 0, 0, 5);
        stage.scene.tilemap_mut(id).unwrap().priorities = Some(short);
        assert_eq!(in_front(&mut stage), (false, true));
        stage.scene.tilemap_mut(id).unwrap().priorities = None;
        assert_eq!(in_front(&mut stage), (false, false));
        stage.assert_pinned("a_tile_without_an_entry_in_the_priorities_has_priority_0");
    }

    #[test]
    fn a_change_to_a_shared_table_shows_in_the_next_frame() {
        let mut stage = Stage::new();
        let map = full_map();
        let id = tilemap(&mut stage, &map);
        assert!(shows_full_map(&stage.frame(), (0, 0), SCREEN));

        // The cell is changed through the handle the script kept, not through the tilemap.
        map.set(3, 2, 0, 384 + 300);
        let frame = stage.frame();
        assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
            384 + 300,
            x,
            y
        )));
        assert!(tile_shows(&frame, 128, 64, |x, y| tile_pixel(
            cell_tile(4, 2),
            x,
            y
        )));

        // So is a priority.
        let blue = stage.solid(32, 32, BLUE);
        let sprite = stage.scene.create_sprite(None).unwrap();
        stage.scene.set_sprite_z(sprite, 100).unwrap();
        let sprite = stage.scene.sprite_mut(sprite).unwrap();
        sprite.set_bitmap(Some(blue), &stage.bitmaps);
        (sprite.x, sprite.y) = (96, 64);
        let priorities = SharedTable::new(Table::new(1000).unwrap());
        stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities.clone());
        assert_eq!(stage.frame().pixel(96, 64), BLUE);
        priorities.set(384 + 300, 0, 0, 1);
        let frame = stage.frame();
        assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
            384 + 300,
            x,
            y
        )));

        // Another table with the same contents is another table.
        let other = SharedTable::new(map.read().clone());
        stage.scene.tilemap_mut(id).unwrap().map_data = Some(other);
        map.set(3, 2, 0, 384);
        let frame = stage.frame();
        assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
            384 + 300,
            x,
            y
        )));
        stage.assert_pinned("a_change_to_a_shared_table_shows_in_the_next_frame");
    }

    #[test]
    fn drawing_into_a_sheet_shows_in_the_next_frame() {
        let mut stage = Stage::sized(96, 64);
        let map = map(3, 2);
        map.set(0, 0, 0, 384);
        map.set(1, 0, 0, 48 + 47);
        let id = tilemap(&mut stage, &map);
        let autotile = painted(&mut stage, 96, 128, autotile_pixel);
        stage.scene.tilemap_mut(id).unwrap().autotiles[0] = Some(autotile);
        let tileset = stage.scene.tilemap(id).unwrap().tileset.unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(5, 6), tile_pixel(384, 5, 6));
        assert_eq!(frame.pixel(32 + 5, 6), autotile_pixel(5, 6));

        stage
            .bitmaps
            .set_pixel(tileset, 5, 6, color(WHITE))
            .unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(5, 6), WHITE);
        assert_eq!(frame.pixel(6, 6), tile_pixel(384, 6, 6));

        stage.bitmaps.set_pixel(autotile, 5, 6, color(RED)).unwrap();
        assert_eq!(stage.frame().pixel(32 + 5, 6), RED);

        // Another sheet with other pixels replaces the old one.
        let other = stage.solid(256, 64, GREEN);
        stage.scene.tilemap_mut(id).unwrap().tileset = Some(other);
        assert_eq!(stage.frame().pixel(5, 6), GREEN);
        stage.assert_pinned("drawing_into_a_sheet_shows_in_the_next_frame");
    }

    /// Measured on the original runtime (tilemap blend probe): an opaque tile of priority 0
    /// on the lowest layer and the same translucent tile of priority 2 on both layers above.
    /// Each of the two is blended onto the frame by itself; composed with each other first
    /// they would give (111, 106, 120).
    #[test]
    fn stacked_translucent_tiles_of_one_priority_are_blended_one_by_one() {
        let mut stage = Stage::sized(64, 64);
        let mut rgba = [6, 14, 2, 255].repeat(32 * 32);
        rgba.extend([138, 130, 150, 142].repeat(32 * 32));
        let tileset = stage.bitmap(32, 64, rgba);
        let map = map(3, 3);
        map.set(0, 0, 0, 384);
        map.set(0, 0, 1, 392);
        map.set(0, 0, 2, 392);
        // One translucent tile alone, and one opaque tile alone.
        map.set(1, 0, 2, 392);
        map.set(1, 1, 0, 384);
        let priorities = SharedTable::new(Table::new(500).unwrap());
        priorities.set(392, 0, 0, 2);
        let id = stage.scene.create_tilemap(None).unwrap();
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.map_data = Some(map.clone());
        tilemap.priorities = Some(priorities);

        let packet = stage.scene.extract(&mut stage.bitmaps);
        // The ground of the lowest layer, and a row of priority 2 of each layer above: the
        // lowest layer holds no tile of that priority and draws nothing for it.
        assert_eq!(packet.ops.len(), 3);
        let frame = stage.render(&packet).unwrap();
        assert_eq!(frame.pixel(5, 6), [111, 107, 120, 255]);
        let once = vkxp_pixel::blend_normal(BLACK, [138, 130, 150, 142], 255);
        assert_eq!(frame.pixel(40, 6), once);
        assert_eq!(frame.pixel(40, 40), [6, 14, 2, 255]);
        // The same picture once more, as a frame of the stage.
        assert_eq!(stage.frame(), frame);
        stage.assert_pinned("stacked_translucent_tiles_of_one_priority_are_blended_one_by_one");
    }

    /// The layers of the ground are blended one by one as well.
    #[test]
    fn stacked_translucent_tiles_of_the_ground_are_blended_one_by_one() {
        let mut stage = Stage::sized(64, 64);
        let mut rgba = [6, 14, 2, 255].repeat(32 * 32);
        rgba.extend([138, 130, 150, 142].repeat(32 * 32));
        let tileset = stage.bitmap(32, 64, rgba);
        let map = map(3, 3);
        map.set(0, 0, 0, 384);
        map.set(0, 0, 1, 392);
        map.set(0, 0, 2, 392);
        let id = stage.scene.create_tilemap(None).unwrap();
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.map_data = Some(map.clone());
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(packet.ops.len(), 3);
        let frame = stage.render(&packet).unwrap();
        assert_eq!(frame.pixel(5, 6), [111, 107, 120, 255]);
        assert_eq!(stage.frame(), frame);
        stage.assert_pinned("stacked_translucent_tiles_of_the_ground_are_blended_one_by_one");
    }

    #[test]
    fn tiles_are_drawn_without_bitmaps_of_the_scene() {
        let mut stage = Stage::sized(160, 160);
        let map = map(5, 5);
        map.set(0, 0, 0, 384);
        map.set(1, 1, 0, 385);
        map.set(2, 2, 0, 386);
        let priorities = SharedTable::new(Table::new(500).unwrap());
        priorities.set(385, 0, 0, 1);
        priorities.set(386, 0, 0, 5);
        let id = tilemap(&mut stage, &map);
        stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities.clone());
        let before = stage.bitmaps.len();
        assert_eq!(before, 1, "the tileset");
        for ox in [0, 32, 45] {
            stage.scene.tilemap_mut(id).unwrap().ox = ox;
            assert!(!all(&stage.frame(), BLACK));
            assert_eq!(stage.bitmaps.len(), before);
        }
        stage.scene.tilemap_mut(id).unwrap().visible = false;
        assert!(all(&stage.frame(), BLACK));
        stage.scene.tilemap_mut(id).unwrap().visible = true;
        assert!(!all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), before);
        stage.assert_pinned("tiles_are_drawn_without_bitmaps_of_the_scene");
    }

    /// The tilemap layer ops of a packet.
    fn layers(packet: &FramePacket) -> Vec<TilemapLayer> {
        let ops = packet.ops.iter().filter_map(|op| match op {
            Op::TilemapLayer(layer) => Some(*layer),
            _ => None,
        });
        ops.collect()
    }

    #[test]
    fn a_tilemap_is_one_op_per_layer_of_the_ground_and_per_row_and_layer_above() {
        let mut stage = Stage::sized(96, 96);
        let map = map(3, 3);
        // Ground on layers 0 and 2; priority 1 in row 1 on layers 0 and 1 and in row 2 on
        // layer 1; priority 3 in row 0 on layer 2.
        map.set(0, 0, 0, 384);
        map.set(2, 2, 2, 385);
        map.set(1, 1, 0, 386);
        map.set(2, 1, 1, 386);
        map.set(0, 2, 1, 386);
        map.set(1, 0, 2, 387);
        let priorities = SharedTable::new(Table::new(400).unwrap());
        priorities.set(386, 0, 0, 1);
        priorities.set(387, 0, 0, 3);
        let id = tilemap(&mut stage, &map);
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.priorities = Some(priorities);
        (tilemap.ox, tilemap.oy) = (5, -7);
        let tileset = tilemap.tileset;
        stage.frame();
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let found = layers(&packet);
        assert_eq!(found.len(), packet.ops.len());
        let screen = vkxp_proto::Rect::new(0, 0, 96, 96);
        let row = |row: i32| vkxp_proto::Rect::new(0, row * 32 + 7, 96, 32);
        // In draw order: the ground at z 0, then the rows by their z, which is 64 more than
        // their top edge for priority 1 and 128 more for priority 3. The map repeats: its
        // last row shows once more in the seven rows of pixels above its first.
        let expected = [
            (screen, 0, 0),
            (screen, 2, 0),
            (vkxp_proto::Rect::new(0, 0, 96, 7), 1, 1),
            (row(1), 0, 1),
            (row(1), 1, 1),
            (row(2), 1, 1),
            (row(0), 2, 3),
        ];
        let pieces: Vec<_> = found
            .iter()
            .map(|layer| (layer.clip, layer.layer, layer.priority))
            .collect();
        // The last row reaches below the screen and is clipped to it.
        let mut expected = expected.to_vec();
        expected[5].0 = vkxp_proto::Rect::new(0, 71, 96, 25);
        assert_eq!(expected[5].0.y, row(2).y);
        assert_eq!(pieces, expected);
        for layer in &found {
            assert_eq!(layer.origin, (5, -7));
            assert_eq!(layer.tileset, tileset);
            assert_eq!(
                (layer.map, layer.priorities),
                (found[0].map, found[0].priorities)
            );
            assert_ne!(layer.map, layer.priorities);
        }
        stage.assert_pinned(
            "a_tilemap_is_one_op_per_layer_of_the_ground_and_per_row_and_layer_above",
        );
    }

    #[test]
    fn a_changed_table_is_sent_again_and_a_table_no_tilemap_has_is_destroyed() {
        let mut stage = Stage::new();
        let map = full_map();
        let priorities = SharedTable::new(Table::new(1000).unwrap());
        let id = tilemap(&mut stage, &map);
        stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities.clone());
        let first = stage.scene.extract(&mut stage.bitmaps);
        stage.render(&first).unwrap();
        let op = layers(&first)[0];
        assert_eq!(first.tables.len(), 2);

        // One cell is changed through the handle the script kept: that table is replaced.
        map.set(3, 2, 0, 384 + 300);
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let expected = TableDelta::Set {
            id: op.map,
            xsize: 20,
            ysize: 15,
            zsize: 3,
            data: map.read().data().to_vec(),
        };
        assert_eq!(packet.tables, [expected]);
        let frame = stage.render(&packet).unwrap();
        assert!(tile_shows(&frame, 96, 64, |x, y| tile_pixel(
            384 + 300,
            x,
            y
        )));

        // A priority outside 0 to 5 is sent as the nearest of them.
        priorities.set(384, 0, 0, 9);
        priorities.set(385, 0, 0, -3);
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let [
            TableDelta::Set {
                id: sent,
                xsize,
                data,
                ..
            },
        ] = &packet.tables[..]
        else {
            panic!("{:?}", packet.tables);
        };
        assert_eq!((*sent, *xsize), (op.priorities, 1000));
        assert_eq!(data[384..386], [5, 0]);
        stage.render(&packet).unwrap();

        // Invisible, the tilemap keeps its tables with the compositor.
        stage.scene.tilemap_mut(id).unwrap().visible = false;
        assert_eq!(stage.scene.extract(&mut stage.bitmaps).tables, []);
        stage.scene.tilemap_mut(id).unwrap().visible = true;
        assert_eq!(stage.scene.extract(&mut stage.bitmaps).tables, []);

        // Without its priorities it names a table without entries, and the old one goes.
        stage.scene.tilemap_mut(id).unwrap().priorities = None;
        let packet = stage.scene.extract(&mut stage.bitmaps);
        let destroyed = TableDelta::Destroy { id: op.priorities };
        assert_eq!(packet.tables.len(), 2);
        assert!(packet.tables.contains(&destroyed));
        assert!(all_layers_name(&packet, op.map));
        stage.render(&packet).unwrap();

        stage.scene.dispose_tilemap(id, &mut stage.bitmaps);
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(packet.tables, [TableDelta::Destroy { id: op.map }]);
        // The compositor is told, and accepts the frame that follows.
        assert!(all(&stage.render(&packet).unwrap(), BLACK));
        assert!(all(&stage.frame(), BLACK));
    }

    fn all_layers_name(packet: &FramePacket, map: TableId) -> bool {
        let layers = layers(packet);
        !layers.is_empty() && layers.iter().all(|layer| layer.map == map)
    }

    #[test]
    fn two_tilemaps_share_the_tables_they_share() {
        let mut stage = Stage::sized(96, 64);
        let map = map(3, 2);
        map.set(0, 0, 0, 384);
        let first = tilemap(&mut stage, &map);
        let second = tilemap(&mut stage, &map);
        stage.scene.tilemap_mut(second).unwrap().ox = 32;
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(packet.tables.len(), 2, "the map and the empty priorities");
        let layers = layers(&packet);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].map, layers[1].map);
        stage.render(&packet).unwrap();
        // One of them disposed, the other still has the table.
        stage.scene.dispose_tilemap(first, &mut stage.bitmaps);
        assert_eq!(stage.scene.extract(&mut stage.bitmaps).tables, []);
    }

    #[test]
    fn a_sprite_between_two_rows_is_in_front_of_the_upper_and_behind_the_lower() {
        let mut stage = Stage::sized(96, 96);
        let map = map(3, 3);
        map.set(0, 0, 0, 384);
        map.set(0, 1, 0, 384);
        let priorities = SharedTable::new(Table::new(400).unwrap());
        priorities.set(384, 0, 0, 1);
        let id = tilemap(&mut stage, &map);
        stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities);
        // The rows rank at 64 and 96; the sprite covers both cells from in between.
        let blue = stage.solid(32, 64, BLUE);
        let sprite = stage.scene.create_sprite(None).unwrap();
        stage.scene.set_sprite_z(sprite, 80).unwrap();
        let sprite = stage.scene.sprite_mut(sprite).unwrap();
        sprite.set_bitmap(Some(blue), &stage.bitmaps);
        let frame = stage.frame();
        assert!(tile_shows(&frame, 0, 0, |_, _| BLUE));
        assert!(tile_shows(&frame, 0, 32, |x, y| tile_pixel(384, x, y)));
        stage.assert_pinned(
            "a_sprite_between_two_rows_is_in_front_of_the_upper_and_behind_the_lower",
        );
    }

    /// Not a check: prints how long extracting a frame takes for a screen of 640 by 480,
    /// which shows 21 by 16 cells, with three full layers. Run it in the release profile:
    /// `cargo test --release -p vkxp-core measure_an_extraction -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement, run on request"]
    fn measure_an_extraction() {
        let mut stage = Stage::new();
        let map = map(40, 30);
        for y in 0..30 {
            for x in 0..40 {
                // Autotiles below, tileset tiles above, some of them with a priority.
                map.set(x, y, 0, i64::from(48 + (x * 7 + y * 13) % 336));
                map.set(x, y, 1, i64::from(384 + (x + y * 3) % TILES));
                map.set(x, y, 2, i64::from(384 + (x * 5 + y) % TILES));
            }
        }
        let priorities = SharedTable::new(Table::new(384 + TILES).unwrap());
        for id in 384..384 + TILES {
            priorities.set(id, 0, 0, i64::from(id % 6));
        }
        let id = tilemap(&mut stage, &map);
        for index in 0..7 {
            let sheet = painted(&mut stage, 384, 128, autotile_pixel);
            stage.scene.tilemap_mut(id).unwrap().autotiles[index] = Some(sheet);
        }
        stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities);
        stage.frame();

        let mut extract = Vec::new();
        for step in 1..=20 {
            let tilemap = stage.scene.tilemap_mut(id).unwrap();
            (tilemap.ox, tilemap.oy) = (step * 32, step * 32);
            let start = std::time::Instant::now();
            let packet = stage.scene.extract(&mut stage.bitmaps);
            extract.push(start.elapsed());
            assert!(packet.bitmaps.is_empty() && packet.tables.is_empty());
        }
        extract.sort();
        println!(
            "extracting a frame of 21 by 16 cells in three layers: median {:?}",
            extract[10]
        );
    }

    /// A stage of 3 by 2 cells whose every cell holds a tileset tile, with flash data that
    /// is all zero, at the strongest point of the pulse.
    fn flashing() -> (Stage, TilemapId, SharedTable) {
        let mut stage = Stage::sized(96, 64);
        let map = map(3, 2);
        for cell in 0..6 {
            map.set(cell % 3, cell / 3, 0, i64::from(384 + cell));
        }
        let id = tilemap(&mut stage, &map);
        let flash = SharedTable::new(Table::new_2d(3, 2).unwrap());
        let tilemap = stage.scene.tilemap_mut(id).unwrap();
        tilemap.flash_data = Some(flash.clone());
        for _ in 0..16 {
            tilemap.update();
        }
        (stage, id, flash)
    }

    /// Whether `frame` is `plain` with `color` added at full strength to the 32 by 32 pixels
    /// at (`left`, `top`), as far as they are on the screen, and nothing else.
    fn flashes(frame: &Frame, plain: &Frame, (left, top): (i32, i32), color: Rgba) -> bool {
        (0..frame.size.height).all(|y| {
            (0..frame.size.width).all(|x| {
                let inside = (left..left + 32).contains(&(x as i32))
                    && (top..top + 32).contains(&(y as i32));
                let expected = if inside {
                    vkxp_pixel::blend_add(plain.pixel(x, y), color, 255)
                } else {
                    plain.pixel(x, y)
                };
                frame.pixel(x, y) == expected
            })
        })
    }

    #[test]
    fn a_cell_with_a_flash_entry_is_tinted_and_its_neighbours_are_not() {
        let (mut stage, id, flash) = flashing();
        let plain = stage.frame();
        flash.set(1, 1, 0, 0xf00);
        let frame = stage.frame();
        assert!(flashes(&frame, &plain, (32, 32), [255, 0, 0, 255]));
        let (tinted, before) = (frame.pixel(40, 40), plain.pixel(40, 40));
        assert!(tinted[0] > before[0], "redder");
        assert_eq!(tinted[1..], before[1..]);

        // Four bits each for red, green and blue; higher bits are not part of the colour.
        flash.set(1, 1, 0, 0x48c);
        assert!(flashes(
            &stage.frame(),
            &plain,
            (32, 32),
            [68, 136, 204, 255]
        ));
        flash.set(1, 1, 0, 0x7000 + 0x00f);
        assert!(flashes(&stage.frame(), &plain, (32, 32), [0, 0, 255, 255]));

        // Back at zero, and without flash data, the tilemap is drawn as before.
        flash.set(1, 1, 0, 0);
        assert_eq!(stage.frame().rgba, plain.rgba);
        flash.set(1, 1, 0, 0xf00);
        stage.scene.tilemap_mut(id).unwrap().flash_data = None;
        assert_eq!(stage.frame().rgba, plain.rgba);
        let stored = stage.scene.tilemap(id).unwrap().flash_data.clone();
        assert_eq!(stored, None);
        stage.assert_pinned("a_cell_with_a_flash_entry_is_tinted_and_its_neighbours_are_not");
    }

    #[test]
    fn the_flash_pulses_with_the_updates_of_the_tilemap() {
        let (mut stage, id, flash) = flashing();
        flash.set(0, 0, 0, 0xfff);
        let mut reds = Vec::new();
        for _ in 0..80 {
            // Without an update the strength stays.
            let red = stage.frame().pixel(8, 8)[0];
            assert_eq!(stage.frame().pixel(8, 8)[0], red);
            reds.push(red);
            stage.scene.tilemap_mut(id).unwrap().update();
        }
        // It starts at the strongest point, falls for 16 updates and rises for 16.
        assert!(reds[..17].windows(2).all(|pair| pair[0] > pair[1]));
        assert!(reds[16..33].windows(2).all(|pair| pair[0] < pair[1]));
        assert!((0..48).all(|update| reds[update] == reds[update + 32]));
        // At the weakest point nothing is added.
        flash.set(0, 0, 0, 0);
        assert_eq!(stage.frame().pixel(8, 8)[0], reds[16]);
        stage.assert_pinned("the_flash_pulses_with_the_updates_of_the_tilemap");
    }

    #[test]
    fn the_flash_overlay_scrolls_with_its_cell() {
        let (mut stage, id, flash) = flashing();
        let mut plain = Vec::new();
        let origins = [(0, 0), (16, 0), (16, -8), (40, 0), (-96, 64), (70, 70)];
        for (ox, oy) in origins {
            let tilemap = stage.scene.tilemap_mut(id).unwrap();
            (tilemap.ox, tilemap.oy) = (ox, oy);
            plain.push(stage.frame());
        }
        flash.set(1, 1, 0, 0x0f0);
        for ((ox, oy), plain) in origins.into_iter().zip(&plain) {
            let tilemap = stage.scene.tilemap_mut(id).unwrap();
            (tilemap.ox, tilemap.oy) = (ox, oy);
            let frame = stage.frame();
            // The map repeats every 96 by 64 pixels, and the flash data with it.
            let at = ((32 - ox).rem_euclid(96), (32 - oy).rem_euclid(64));
            let tinted = [
                at,
                (at.0 - 96, at.1),
                (at.0, at.1 - 64),
                (at.0 - 96, at.1 - 64),
            ]
            .into_iter()
            .fold(plain.clone(), |shown, corner| {
                let mut next = shown.clone();
                for y in 0..64 {
                    for x in 0..96 {
                        if flashes_at(corner, x, y) {
                            let pixel =
                                vkxp_pixel::blend_add(shown.pixel(x, y), [0, 255, 0, 255], 255);
                            let offset = ((y * 96 + x) * 4) as usize;
                            next.rgba[offset..offset + 4].copy_from_slice(&pixel);
                        }
                    }
                }
                next
            });
            assert_eq!(frame.rgba, tinted.rgba, "origin ({ox}, {oy})");
            assert_ne!(frame.rgba, plain.rgba, "origin ({ox}, {oy})");
        }
        stage.assert_pinned("the_flash_overlay_scrolls_with_its_cell");
    }

    /// Whether (`x`, `y`) is among the 32 by 32 pixels at `corner`.
    fn flashes_at(corner: (i32, i32), x: u32, y: u32) -> bool {
        (corner.0..corner.0 + 32).contains(&(x as i32))
            && (corner.1..corner.1 + 32).contains(&(y as i32))
    }

    #[test]
    fn the_flash_overlay_is_above_the_ground_and_below_tiles_with_a_priority() {
        let (mut stage, id, flash) = flashing();
        let priorities = SharedTable::new(Table::new(384 + TILES).unwrap());
        priorities.set(384 + 100, 0, 0, 1);
        stage.scene.tilemap_mut(id).unwrap().priorities = Some(priorities);
        let map = stage.scene.tilemap(id).unwrap().map_data.clone().unwrap();
        map.set(2, 0, 1, 384 + 100);
        let plain = stage.frame();
        flash.set(1, 0, 0, 0xfff);
        flash.set(2, 0, 0, 0xfff);
        let frame = stage.frame();
        assert!(flashes(&frame, &plain, (32, 0), WHITE));
        assert!(tile_shows(&frame, 64, 0, |x, y| tile_pixel(484, x, y)));
        stage
            .assert_pinned("the_flash_overlay_is_above_the_ground_and_below_tiles_with_a_priority");
    }

    #[test]
    fn the_flash_overlay_is_given_back_with_the_last_flashing_cell() {
        let (mut stage, id, flash) = flashing();
        stage.frame();
        let without = stage.bitmaps.len();
        flash.set(2, 1, 0, 0x00f);
        stage.frame();
        assert_eq!(stage.bitmaps.len(), without + 1);
        // Unchanged, the overlay is not uploaded again.
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert!(packet.bitmaps.is_empty());
        flash.set(2, 1, 0, 0);
        stage.frame();
        assert_eq!(stage.bitmaps.len(), without);
        flash.set(2, 1, 0, 0x00f);
        stage.frame();
        stage.scene.tilemap_mut(id).unwrap().visible = false;
        stage.frame();
        assert_eq!(stage.bitmaps.len(), without, "the overlay");
        stage.assert_pinned("the_flash_overlay_is_given_back_with_the_last_flashing_cell");
    }
}
