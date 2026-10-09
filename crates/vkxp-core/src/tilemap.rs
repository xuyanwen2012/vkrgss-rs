//! Tilemaps: the visible tiles composed into cached bitmaps, and the quads that show them.
//!
//! A tilemap is not drawn tile by tile every frame. The cells that can show in its area, and
//! one more column and row so that the origin can move within a tile, are composed with the
//! operations of the bitmap arena into bitmaps of the scene's own: `ground` for the tiles of
//! priority 0, and one bitmap for each of the priorities 1 to 5 that a visible tile has. They
//! are composed again only when what they show changes: a scroll across a tile boundary, a
//! change of the area's size, of a table, of a sheet, or of the animation frame of a visible
//! autotile.
//!
//! A frame then draws `ground` as one quad at z 0, and of every priority bitmap one quad for
//! each row of cells that holds a tile of that priority, at the z that ranks the row among
//! sprites.
//!
//! The cells the flash data marks are filled with their colour, at the alpha of the pulse,
//! in one more bitmap of the same size, which is drawn additively right above `ground`. It is
//! filled again when the flash data, the first cell or the pulse changes.
//!
//! This is a stand-in for the tilemap op of `docs/decisions.md`, which is to replace the
//! cache without changing what the tilemap looks like from outside.

use vkxp_proto::{BitmapId, BlendMode};

use crate::extract::{Assembly, Level, Look};
use crate::geometry::Geometry;
use crate::tiles::{Sheet, TILE, frames, pieces, sheet};
use crate::{Bitmaps, Color, FontDefaults, Rect, SharedTable, Table, Tilemap};

/// Number of layers of a map.
const LAYERS: i32 = 3;
/// The highest priority a tile can have.
const MAX_PRIORITY: usize = 5;
/// Number of updates one animation frame of an autotile stays.
const ANIMATION_PERIOD: u64 = 16;
/// Number of updates one pulse of the flash data takes.
const FLASH_PERIOD: u64 = 32;

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
    /// `layers[0]` is `ground`, `layers[p]` holds the tiles of priority `p`.
    layers: [Option<BitmapId>; MAX_PRIORITY + 1],
    /// For each priority from 1 on, the rows of cells that hold a tile of it, ascending.
    rows: [Vec<i32>; MAX_PRIORITY],
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
    priority: usize,
    sheet: BitmapId,
    /// Width and height of the sheet.
    size: (i32, i32),
    frame: u32,
}

impl TileCache {
    pub(crate) fn dispose(&mut self, bitmaps: &mut Bitmaps) {
        for layer in self.layers.iter_mut().chain([&mut self.flash]) {
            if let Some(id) = layer.take() {
                bitmaps.dispose(id);
            }
        }
        *self = TileCache::default();
    }

    /// Whether a bitmap of the cache was disposed behind its back.
    fn lost(&self, bitmaps: &Bitmaps) -> bool {
        let layers = self.layers.iter().flatten();
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
            let wanted = tiles.iter().any(|tile| tile.priority == priority);
            // A side is at most the screen's and two tiles more.
            let layer = &mut self.layers[priority];
            TileCache::prepare(layer, bitmaps, wanted, columns * TILE, rows * TILE);
        }
        for tile in tiles {
            if let Some(Sheet::Autotile(index)) = sheet(tile.id) {
                self.frames[index] = Some(tile.frame);
            }
            let Some(layer) = self.layers[tile.priority] else {
                continue;
            };
            let (width, height) = tile.size;
            for piece in pieces(tile.id, width, height, tile.frame) {
                let (x, y) = (tile.column * TILE + piece.x, tile.row * TILE + piece.y);
                // Both bitmaps were found live when the tiles were collected.
                let _ = bitmaps.blt(layer, x, y, tile.sheet, piece.src, 255);
            }
            // Tiles come row by row.
            if tile.priority > 0 && self.rows[tile.priority - 1].last() != Some(&tile.row) {
                self.rows[tile.priority - 1].push(tile.row);
            }
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
    /// Brings the flash overlay up to date for the `cells` cached from cell `first` on.
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

    /// Brings the cached bitmaps up to date with what this frame shows in `level`. An
    /// invisible tilemap gives them back; one that has no `level`, because its viewport is
    /// not drawn, keeps what it has.
    pub(crate) fn refresh_cache(&mut self, bitmaps: &mut Bitmaps, level: Option<&Level>) {
        if !self.visible {
            self.cache.dispose(bitmaps);
            return;
        }
        let Some(level) = level else {
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

        self.refresh_flash(bitmaps, first, cells);

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
        if self.cache.fresh(&key, &current) && !self.cache.lost(bitmaps) {
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
                    let Some(id) = map_table.get(x, y, layer) else {
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
                        priority: priority(priorities.as_deref(), id),
                        sheet,
                        size,
                        frame,
                    });
                }
            }
        }
        self.cache.compose(bitmaps, key, &tiles);
    }

    /// The pieces this frame draws the tilemap with, each with its z. `scroll` is the vertical
    /// origin of the tilemap's viewport, 0 without one: the top edge of a row is where it is
    /// drawn in the viewport, which both origins move.
    ///
    /// PLACEHOLDER: the z of the tiles of priority 1 to 5, the y of their top edge in the
    /// viewport plus 32 for each step of priority and 32 more, is not yet checked against the
    /// original runtime; neither is that a sprite created after the tilemap wins a tie.
    pub(crate) fn strips(&self, scroll: i64) -> impl Iterator<Item = (i64, Strip)> {
        let key = self.cache.key.as_ref().filter(|_| self.visible);
        let top = key.map_or(0, |key| {
            key.first.1 * i64::from(TILE) - i64::from(self.oy) - scroll
        });
        let ground = key.map(|_| (0, Strip::Ground));
        // In the same place of the order as the ground, and after it.
        let flash = key
            .filter(|_| self.cache.flash.is_some())
            .map(|_| (0, Strip::Flash));
        let rows = (1..=MAX_PRIORITY)
            .filter(move |_| key.is_some())
            .flat_map(move |priority| {
                self.cache.rows[priority - 1].iter().map(move |&row| {
                    let y = top + i64::from(row) * i64::from(TILE);
                    let z = y + (priority as i64 + 1) * i64::from(TILE);
                    (z, Strip::Row { priority, row })
                })
            });
        ground.into_iter().chain(flash).chain(rows)
    }
}

/// One piece of a tilemap that takes a place of its own in the draw order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Strip {
    /// Every tile of priority 0.
    Ground,
    /// The colours of the flashing cells.
    Flash,
    /// The tiles of one priority from 1 on in one row of cells of the cache.
    Row { priority: usize, row: i32 },
}

impl Assembly<'_> {
    /// One piece of a tilemap, from its cached bitmaps.
    pub(crate) fn tiles(&mut self, tilemap: &Tilemap, strip: Strip, level: &Level) {
        let Some(key) = tilemap.cache.key.as_ref().filter(|_| tilemap.visible) else {
            return;
        };
        let tile = i64::from(TILE);
        // Where the first cell of the cache is on the screen: at most a tile left of and
        // above the clip.
        let x = level.dx + key.first.0 * tile - i64::from(tilemap.ox);
        let y = level.dy + key.first.1 * tile - i64::from(tilemap.oy);
        match strip {
            Strip::Ground => {
                if let Some(bitmap) = tilemap.cache.layers[0] {
                    self.whole(bitmap, x, y, level.clip, 255);
                }
            }
            Strip::Flash => {
                if let Some(bitmap) = tilemap.cache.flash {
                    let look = Look {
                        blend: BlendMode::Add,
                        ..Look::plain(255)
                    };
                    let src = Rect::new(0, 0, key.cells.0 * TILE, key.cells.1 * TILE);
                    self.quad(bitmap, src, &Geometry::at(x, y), level.clip, look);
                }
            }
            Strip::Row { priority, row } => {
                if let Some(bitmap) = tilemap.cache.layers[priority] {
                    let src = Rect::new(0, row * TILE, key.cells.0 * TILE, TILE);
                    let geometry = Geometry::at(x, y + i64::from(row) * tile);
                    self.quad(bitmap, src, &geometry, level.clip, Look::plain(255));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vkxp_compose_ref::Frame;
    use vkxp_pixel::Rgba;

    use super::*;
    use crate::bitmap::testing::color;
    use crate::extract::testing::*;
    use crate::tiles::AUTOTILES;
    use crate::{SpriteId, TilemapId};

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
    }

    #[test]
    fn a_tilemap_without_map_data_draws_nothing() {
        let mut stage = Stage::sized(96, 64);
        let id = tilemap(&mut stage, &map(0, 0));
        assert!(all(&stage.frame(), BLACK));
        stage.scene.tilemap_mut(id).unwrap().map_data = None;
        assert!(all(&stage.frame(), BLACK));
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
                let quarter = u32::from(AUTOTILES[pattern as usize][corner as usize]) - 1;
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
    }

    #[test]
    fn each_of_the_seven_autotiles_has_its_own_sheet() {
        for index in [1, 6] {
            let (mut stage, _) = patterns(96, 128, index);
            assert!(shows_patterns(&stage.frame(), 0), "autotile {index}");
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
    }

    #[test]
    fn scrolling_within_a_tile_composes_nothing_anew() {
        let mut stage = Stage::new();
        let id = tilemap(&mut stage, &full_map());
        // The deltas of the frame after the origin is set to `ox`.
        let mut uploads = |ox: i32| {
            stage.scene.tilemap_mut(id).unwrap().ox = ox;
            let packet = stage.scene.extract(&mut stage.bitmaps);
            let frame = stage.render(&packet).unwrap();
            assert!(shows_full_map(&frame, (ox, 0), SCREEN), "{ox}");
            packet.bitmaps.len()
        };
        assert_ne!(uploads(64), 0);
        for ox in [65, 80, 95, 64] {
            assert_eq!(uploads(ox), 0, "{ox}");
        }
        assert_ne!(uploads(96), 0);
        assert_ne!(uploads(63), 0);
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
        }
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
    }

    #[test]
    fn cached_bitmaps_are_disposed_with_the_tilemap_or_when_it_is_invisible() {
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
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before + 3, "ground and two priorities");

        // Composing anew does not let bitmaps pile up, and a priority no visible tile has
        // any more gives its bitmap back.
        stage.scene.tilemap_mut(id).unwrap().ox = 32;
        stage.frame();
        map.set(4, 4, 1, 387);
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before + 3);
        priorities.set(386, 0, 0, 1);
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before + 2);

        // An invisible tilemap keeps none.
        stage.scene.tilemap_mut(id).unwrap().visible = false;
        assert!(all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), before);
        stage.scene.tilemap_mut(id).unwrap().visible = true;
        assert!(!all(&stage.frame(), BLACK));
        assert_eq!(stage.bitmaps.len(), before + 2);

        stage.scene.dispose_tilemap(id, &mut stage.bitmaps);
        assert_eq!(stage.bitmaps.len(), before);
        // The compositor is told, and accepts the frame that follows.
        assert!(all(&stage.frame(), BLACK));
    }

    /// Not a check: prints how long composing the cache takes for a screen of 640 by 480,
    /// which is 21 by 16 cells, with three full layers. Run it in the release profile:
    /// `cargo test --release -p vkxp-core measure_a_rebuild -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement, run on request"]
    fn measure_a_rebuild() {
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

        let screen = Level {
            clip: vkxp_proto::Rect::new(0, 0, 640, 480),
            dx: 0,
            dy: 0,
        };
        let mut compose = Vec::new();
        let mut extract = Vec::new();
        for step in 1..=20 {
            // Each step scrolls across a tile boundary, so everything is composed anew.
            let tilemap = stage.scene.tilemap_mut(id).unwrap();
            tilemap.ox = step * 32;
            let start = std::time::Instant::now();
            tilemap.refresh_cache(&mut stage.bitmaps, Some(&screen));
            compose.push(start.elapsed());

            stage.scene.tilemap_mut(id).unwrap().oy = step * 32;
            let start = std::time::Instant::now();
            let packet = stage.scene.extract(&mut stage.bitmaps);
            extract.push(start.elapsed());
            assert_eq!(packet.bitmaps.len(), 6, "ground and five priorities");
        }
        compose.sort();
        extract.sort();
        println!(
            "composing 21 by 16 cells in three layers: median {:?}",
            compose[10]
        );
        println!(
            "extracting the frame with its uploads: median {:?}",
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
        assert_eq!(stage.bitmaps.len(), without - 1, "ground and overlay");
    }
}
