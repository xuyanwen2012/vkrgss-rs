//! The composition this crate drew tilemaps with before the tilemap op: the visible tiles
//! composed into cached bitmaps, one per priority and map layer that holds a tile, drawn as
//! ordinary quads. Its pictures are identical to the original runtime's on real games, so it
//! is kept, for the tests only, as what the op-based extraction is compared with frame for
//! frame.
//!
//! The cells that can show in the tilemap's area, and one more column and row, are composed
//! with the operations of the bitmap arena. The layers of the map are kept apart because the
//! original runtime blends every tile onto the screen by itself, in ascending priority and,
//! within a priority, in the order of the layers. A frame draws the bitmaps of priority 0 as
//! one quad each at z 0, and of every bitmap of a higher priority one quad for each row of
//! cells in which it holds a tile, at the z that ranks the row among sprites. The flash
//! overlay is one more bitmap, drawn additively right above the ground.

use vkxp_proto::{BitmapId, BlendMode};

use super::{ANIMATION_PERIOD, LAYERS, MAX_PRIORITY, Strip, TILE, flash_alpha, priority};
use crate::extract::{Assembly, Level, Look};
use crate::geometry::Geometry;
use crate::tiles::{Sheet, frames, pieces, sheet};
use crate::{Bitmaps, Color, FontDefaults, Rect, SharedTable, Tilemap};

/// The state of the cached composition of one tilemap.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Cached {
    /// Whether the tilemap is drawn from the cached bitmaps instead of as tilemap ops.
    pub(crate) active: bool,
    pub(crate) cache: TileCache,
}

/// A table and the revision it had.
type TableState = (SharedTable, u64);
/// A live bitmap and the revision it had.
type SheetState = (BitmapId, u64);

/// What the cached bitmaps were composed from, apart from the animation.
#[derive(Debug, Clone, PartialEq)]
struct Key {
    /// The cell in the top-left corner of the cache. Cells are counted across the whole
    /// plane, not wrapped into the map.
    first: (i64, i64),
    /// Columns and rows of cells.
    cells: (i32, i32),
    map: TableState,
    priorities: Option<TableState>,
    tileset: Option<SheetState>,
    autotiles: [Option<SheetState>; 7],
}

/// What the flash overlay was filled from.
#[derive(Debug, Clone, PartialEq)]
struct FlashKey {
    first: (i64, i64),
    cells: (i32, i32),
    table: TableState,
    alpha: u8,
}

/// The bitmaps a tilemap keeps in the bitmap arena.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TileCache {
    /// What the bitmaps were composed from, or would have been had there been anything.
    key: Option<Key>,
    /// The animation frame each autotile was drawn at; `None` for one no visible tile uses.
    frames: [Option<u32>; 7],
    /// `layers[p][l]` holds the tiles of priority `p` of layer `l` of the map; the bitmaps of
    /// priority 0 are the ground.
    layers: [[Option<BitmapId>; LAYERS]; MAX_PRIORITY + 1],
    /// For each priority from 1 on and each layer of the map, the rows of cells that hold a
    /// tile of them, ascending.
    rows: [[Vec<i32>; LAYERS]; MAX_PRIORITY],
    /// What `flash` was filled from; `None` without flash data.
    flash_key: Option<FlashKey>,
    /// The colours of the flashing cells, when there is one among the cached cells.
    flash: Option<BitmapId>,
}

/// One visible tile.
struct Tile {
    column: i32,
    row: i32,
    id: i16,
    /// The layer of the map.
    layer: usize,
    priority: usize,
    sheet: BitmapId,
    /// Width and height of the sheet.
    size: (i32, i32),
    frame: u32,
}

impl TileCache {
    pub(crate) fn dispose(&mut self, bitmaps: &mut Bitmaps) {
        for layer in self.layers.iter_mut().flatten().chain([&mut self.flash]) {
            if let Some(id) = layer.take() {
                bitmaps.dispose(id);
            }
        }
        *self = TileCache::default();
    }

    /// Whether a bitmap of the cache was disposed behind its back.
    fn lost(&self, bitmaps: &Bitmaps) -> bool {
        let layers = self.layers.iter().flatten().flatten();
        layers.into_iter().any(|&id| bitmaps.is_disposed(id))
    }

    /// Whether the cache shows what `key` and the current animation frames describe.
    fn fresh(&self, key: &Key, frames: &[u32; 7]) -> bool {
        let drawn = self.frames.iter().zip(frames);
        self.key.as_ref() == Some(key)
            && drawn
                .into_iter()
                .all(|(drawn, now)| drawn.is_none_or(|drawn| drawn == *now))
    }

    /// Makes `layer` an empty bitmap of `width` by `height` pixels if `wanted`, and gives its
    /// bitmap back otherwise. A cache too large for a bitmap goes without the layer.
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

    /// Composes every layer anew from `tiles`.
    fn compose(&mut self, bitmaps: &mut Bitmaps, key: Key, tiles: &[Tile]) {
        let (columns, rows) = key.cells;
        self.key = Some(key);
        self.frames = [None; 7];
        self.rows = Default::default();
        for priority in 0..=MAX_PRIORITY {
            for layer in 0..LAYERS {
                let wanted = tiles
                    .iter()
                    .any(|tile| tile.priority == priority && tile.layer == layer);
                // A side is at most the screen's and two tiles more.
                let bitmap = &mut self.layers[priority][layer];
                TileCache::prepare(bitmap, bitmaps, wanted, columns * TILE, rows * TILE);
            }
        }
        for tile in tiles {
            if let Some(Sheet::Autotile(index)) = sheet(tile.id) {
                self.frames[index] = Some(tile.frame);
            }
            let Some(layer) = self.layers[tile.priority][tile.layer] else {
                continue;
            };
            let (width, height) = tile.size;
            for piece in pieces(tile.id, width, height, tile.frame) {
                let (x, y) = (tile.column * TILE + piece.x, tile.row * TILE + piece.y);
                // Both bitmaps were found live when the tiles were collected.
                let _ = bitmaps.blt(layer, x, y, tile.sheet, piece.src, 255);
            }
            // Tiles come row by row.
            if tile.priority > 0 {
                let rows = &mut self.rows[tile.priority - 1][tile.layer];
                if rows.last() != Some(&tile.row) {
                    rows.push(tile.row);
                }
            }
        }
    }
}

impl Tilemap {
    /// Brings the flash overlay up to date for the `cells` cached from cell `first` on.
    fn refresh_cached_flash(
        &mut self,
        bitmaps: &mut Bitmaps,
        first: (i64, i64),
        cells: (i32, i32),
    ) {
        let key = self.flash_data.as_ref().map(|table| FlashKey {
            first,
            cells,
            table: (table.clone(), table.revision()),
            alpha: flash_alpha(self.updates),
        });
        let lost = self
            .cached
            .cache
            .flash
            .is_some_and(|id| bitmaps.is_disposed(id));
        if self.cached.cache.flash_key == key && !lost {
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
        let layer = &mut self.cached.cache.flash;
        TileCache::prepare(layer, bitmaps, wanted, cells.0 * TILE, cells.1 * TILE);
        if let (Some(bitmap), Some(key)) = (self.cached.cache.flash, &key) {
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
        self.cached.cache.flash_key = key;
    }

    /// Brings the cached bitmaps up to date with what this frame shows in `level`. An
    /// invisible tilemap gives them back; one that has no `level`, because its viewport is
    /// not drawn, keeps what it has.
    pub(crate) fn refresh_cached(&mut self, bitmaps: &mut Bitmaps, level: Option<&Level>) {
        if !self.visible {
            self.cached.cache.dispose(bitmaps);
            return;
        }
        let Some(level) = level else {
            return;
        };
        let Some(map) = self.map_data.clone() else {
            self.cached.cache.dispose(bitmaps);
            return;
        };
        let map_table = map.read();
        let (xsize, ysize) = (map_table.xsize(), map_table.ysize());
        if xsize == 0 || ysize == 0 {
            self.cached.cache.dispose(bitmaps);
            return;
        }

        // The pixel of the map under the top-left pixel of the area, and from it the cells
        // the area can show while the origin moves within a tile: the area rounded up to
        // whole tiles and one more.
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

        self.refresh_cached_flash(bitmaps, first, cells);

        let live = |id: Option<BitmapId>| id.and_then(|id| Some((id, bitmaps.revision(id).ok()?)));
        let size = |id: Option<BitmapId>| {
            let rect = bitmaps.rect(id?).ok()?;
            Some((rect.width, rect.height))
        };
        let autotile_sizes = self.autotiles.map(size);
        let now = self.updates / ANIMATION_PERIOD;
        let current = autotile_sizes.map(|size| {
            let (width, height) = size.unwrap_or((0, 0));
            // Less than the number of frames, which is a `u32`.
            (now % u64::from(frames(width, height))) as u32
        });
        let key = Key {
            first,
            cells,
            map: (map.clone(), map.revision()),
            priorities: self
                .priorities
                .as_ref()
                .map(|table| (table.clone(), table.revision())),
            tileset: live(self.tileset),
            autotiles: self.autotiles.map(live),
        };
        if self.cached.cache.fresh(&key, &current) && !self.cached.cache.lost(bitmaps) {
            return;
        }

        // PLACEHOLDER: that the map repeats beyond its edges is assumed, not yet checked
        // against the original runtime.
        let priorities = self.priorities.as_ref().map(|table| table.read());
        let tileset = self.tileset.zip(size(self.tileset));
        let mut tiles = Vec::new();
        for row in 0..cells.1 {
            // Remainders of sizes that are `i32`.
            let y = (first.1 + i64::from(row)).rem_euclid(i64::from(ysize)) as i32;
            for column in 0..cells.0 {
                let x = (first.0 + i64::from(column)).rem_euclid(i64::from(xsize)) as i32;
                for layer in 0..LAYERS {
                    let Some(id) = map_table.get(x, y, layer as i32) else {
                        continue;
                    };
                    let found = match sheet(id) {
                        None => None,
                        Some(Sheet::Tileset) => tileset.map(|(sheet, size)| (sheet, size, 0)),
                        Some(Sheet::Autotile(index)) => self.autotiles[index]
                            .zip(autotile_sizes[index])
                            .map(|(sheet, size)| (sheet, size, current[index])),
                    };
                    let Some((sheet, size, frame)) = found else {
                        continue;
                    };
                    if pieces(id, size.0, size.1, frame).next().is_none() {
                        continue;
                    }
                    tiles.push(Tile {
                        column,
                        row,
                        id,
                        layer,
                        priority: priority(priorities.as_deref(), id),
                        sheet,
                        size,
                        frame,
                    });
                }
            }
        }
        self.cached.cache.compose(bitmaps, key, &tiles);
    }

    /// The pieces this frame draws the tilemap with, each with its z. `scroll` is the vertical
    /// origin of the tilemap's viewport, 0 without one: the top edge of a row is where it is
    /// drawn in the viewport, which both origins move.
    ///
    /// PLACEHOLDER: the z of the tiles of priority 1 to 5, the y of their top edge in the
    /// viewport plus 32 for each step of priority and 32 more, is not yet checked against the
    /// original runtime; neither is that a sprite created after the tilemap wins a tie.
    pub(crate) fn cached_strips(&self, scroll: i64) -> impl Iterator<Item = (i64, Strip)> {
        let key = self.cached.cache.key.as_ref().filter(|_| self.visible);
        let top = key.map_or(0, |key| {
            key.first.1 * i64::from(TILE) - i64::from(self.oy) - scroll
        });
        let ground = key.map(|_| (0, Strip::Ground));
        // In the same place of the order as the ground, and after it.
        let flash = key
            .filter(|_| self.cached.cache.flash.is_some())
            .map(|_| (0, Strip::Flash));
        let rows = (1..=MAX_PRIORITY)
            .filter(move |_| key.is_some())
            .flat_map(move |priority| {
                // The rows that hold a tile of this priority in any layer of the map.
                let mut rows: Vec<i32> = self.cached.cache.rows[priority - 1].concat();
                rows.sort_unstable();
                rows.dedup();
                rows.into_iter().map(move |row| {
                    let y = top + i64::from(row) * i64::from(TILE);
                    let z = y + (priority as i64 + 1) * i64::from(TILE);
                    (z, Strip::Row { priority, row })
                })
            });
        ground.into_iter().chain(flash).chain(rows)
    }
}

impl Assembly<'_> {
    /// One piece of a tilemap, from its cached bitmaps: one quad for each layer of the map
    /// that holds a tile of the piece, in the order of the layers, so that every tile is
    /// blended onto the frame by itself.
    pub(crate) fn cached_tiles(&mut self, tilemap: &Tilemap, strip: Strip, level: &Level) {
        let Some(key) = tilemap
            .cached
            .cache
            .key
            .as_ref()
            .filter(|_| tilemap.visible)
        else {
            return;
        };
        let tile = i64::from(TILE);
        // Where the first cell of the cache is on the screen: at most a tile left of and
        // above the clip.
        let x = level.dx + key.first.0 * tile - i64::from(tilemap.ox);
        let y = level.dy + key.first.1 * tile - i64::from(tilemap.oy);
        match strip {
            Strip::Ground => {
                for bitmap in tilemap.cached.cache.layers[0].into_iter().flatten() {
                    if let Ok(whole) = self.bitmaps.rect(bitmap) {
                        let geometry = Geometry::at(x, y);
                        self.quad(bitmap, whole, &geometry, level.clip, Look::plain(255));
                    }
                }
            }
            Strip::Flash => {
                if let Some(bitmap) = tilemap.cached.cache.flash {
                    let look = Look {
                        blend: BlendMode::Add,
                        ..Look::plain(255)
                    };
                    let src = Rect::new(0, 0, key.cells.0 * TILE, key.cells.1 * TILE);
                    self.quad(bitmap, src, &Geometry::at(x, y), level.clip, look);
                }
            }
            Strip::Row { priority, row } => {
                let src = Rect::new(0, row * TILE, key.cells.0 * TILE, TILE);
                let geometry = Geometry::at(x, y + i64::from(row) * tile);
                let layers = tilemap.cached.cache.layers[priority].iter();
                for (bitmap, rows) in layers.zip(&tilemap.cached.cache.rows[priority - 1]) {
                    if let Some(bitmap) = bitmap.filter(|_| rows.binary_search(&row).is_ok()) {
                        self.quad(bitmap, src, &geometry, level.clip, Look::plain(255));
                    }
                }
            }
        }
    }
}
