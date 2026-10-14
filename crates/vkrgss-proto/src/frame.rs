//! The frame packet: bitmap and table changes and the ordered op list.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{BitmapId, Color, Fixed, Rect, Size, TableId, Tone, tiles_rgss2};

/// Everything a backend needs to produce one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FramePacket {
    /// Counts frames from the start of the guest.
    pub frame_index: u64,
    /// Size of the logical screen in pixels, from 1 to [`MAX_LOGICAL_DIMENSION`] a side. Op
    /// coordinates are in this space.
    pub logical_size: Size,
    /// The frame rate the guest runs at. Compositors ignore it; the host paces to it.
    pub frame_rate: FrameRate,
    /// The brightness of the whole frame, from 0 to 255. Compositors darken the composed
    /// frame by it after the last op; 255 shows the frame as composed.
    pub brightness: u8,
    /// Bitmap changes since the previous packet, applied in order before any op is drawn.
    pub bitmaps: Vec<BitmapDelta>,
    /// Table changes since the previous packet, applied in order before any op is drawn.
    pub tables: Vec<TableDelta>,
    /// Draw ops. Their order is the draw order; backends never reorder them.
    pub ops: Vec<Op>,
}

/// A frame rate in whole frames per second, always from [`FrameRate::MIN`] to
/// [`FrameRate::MAX`].
///
/// The range is that of `Graphics.frame_rate`. A value outside it cannot be constructed and
/// does not decode, so no receiver has to check it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct FrameRate(u8);

impl FrameRate {
    /// The lowest frame rate.
    pub const MIN: FrameRate = FrameRate(10);
    /// The highest frame rate.
    pub const MAX: FrameRate = FrameRate(120);
    /// The frame rate a game starts with.
    pub const DEFAULT: FrameRate = FrameRate(40);

    /// The frame rate of `fps` frames per second, if that is in range.
    pub const fn new(fps: u8) -> Result<FrameRate, FrameRateOutOfRange> {
        if fps >= Self::MIN.0 && fps <= Self::MAX.0 {
            Ok(FrameRate(fps))
        } else {
            Err(FrameRateOutOfRange { found: fps })
        }
    }

    /// The frame rate nearest to `fps`, as `Graphics.frame_rate=` assigns it.
    pub const fn clamped(fps: i64) -> FrameRate {
        if fps < Self::MIN.0 as i64 {
            Self::MIN
        } else if fps > Self::MAX.0 as i64 {
            Self::MAX
        } else {
            FrameRate(fps as u8)
        }
    }

    /// Frames per second.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for FrameRate {
    fn default() -> FrameRate {
        FrameRate::DEFAULT
    }
}

impl TryFrom<u8> for FrameRate {
    type Error = FrameRateOutOfRange;

    fn try_from(fps: u8) -> Result<FrameRate, FrameRateOutOfRange> {
        FrameRate::new(fps)
    }
}

impl From<FrameRate> for u8 {
    fn from(rate: FrameRate) -> u8 {
        rate.get()
    }
}

impl std::fmt::Display for FrameRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A number of frames per second that is not a valid [`FrameRate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("frame rate {found} is outside {min} to {max}", min = FrameRate::MIN, max = FrameRate::MAX)]
pub struct FrameRateOutOfRange {
    pub found: u8,
}

/// A change to the set of bitmaps known to the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitmapDelta {
    /// Creates a bitmap whose pixels are all transparent black.
    Create {
        id: BitmapId,
        width: u32,
        height: u32,
    },
    /// Replaces the pixels inside `rect`. `rgba` holds `rect.width * rect.height` pixels in row
    /// order, four bytes each (red, green, blue, straight alpha).
    Update {
        id: BitmapId,
        rect: Rect,
        rgba: Vec<u8>,
    },
    /// Destroys a bitmap. Its id is invalid from here on, even if the slot is reused.
    Destroy { id: BitmapId },
}

/// A change to the set of tables known to the host.
///
/// A table is an array of signed 16-bit integers with one to three dimensions; a dimension it
/// does not use has size 1. Elements are in order with x varying fastest, then y, then z.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableDelta {
    /// Creates the table `id`, or replaces it when it exists. `data` holds
    /// `xsize * ysize * zsize` elements.
    Set {
        id: TableId,
        xsize: u32,
        ysize: u32,
        zsize: u32,
        data: Vec<i16>,
    },
    /// Destroys a table. Its id is invalid until it is set again.
    Destroy { id: TableId },
}

/// One step of drawing. Every op carries a clip rectangle in logical screen pixels and never
/// changes a pixel outside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    /// Sets every pixel in `clip` to `color`, alpha included.
    Clear { clip: Rect, color: Color },
    /// Draws part of a bitmap.
    Quad(Quad),
    /// Changes the pixels already present in a rectangle.
    RectEffect(RectEffect),
    /// Draws the tiles of one priority of one layer of a map.
    TilemapLayer(TilemapLayer),
    /// Draws one of the two planes of a tilemap of RGSS2.
    TilemapPlane(TilemapPlane),
}

impl Op {
    /// The kind of this op, for reports and errors.
    pub const fn kind(&self) -> OpKind {
        match self {
            Op::Clear { .. } => OpKind::Clear,
            Op::Quad(_) => OpKind::Quad,
            Op::RectEffect(_) => OpKind::RectEffect,
            Op::TilemapLayer(_) => OpKind::TilemapLayer,
            Op::TilemapPlane(_) => OpKind::TilemapPlane,
        }
    }

    /// The clip rectangle of this op.
    pub const fn clip(&self) -> Rect {
        match self {
            Op::Clear { clip, .. } => *clip,
            Op::Quad(quad) => quad.clip,
            Op::TilemapLayer(layer) => layer.clip,
            Op::TilemapPlane(plane) => plane.clip,
            Op::RectEffect(effect) => effect.clip,
        }
    }
}

/// The name of an op variant without its data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpKind {
    Clear,
    Quad,
    RectEffect,
    TilemapLayer,
    TilemapPlane,
}

impl std::fmt::Display for OpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OpKind::Clear => "Clear",
            OpKind::Quad => "Quad",
            OpKind::RectEffect => "RectEffect",
            OpKind::TilemapLayer => "TilemapLayer",
            OpKind::TilemapPlane => "TilemapPlane",
        })
    }
}

/// Draws the `src` rectangle of a bitmap, placed on the screen as `sampling` says.
///
/// `sampling` decides which screen pixels the quad covers and which texel each of them takes;
/// no texel is ever filtered. The texel is changed first by `tone`, then by `color` blended over
/// it with the colour's alpha, both keeping the texel's own alpha. The result is blended over
/// the pixel underneath at `opacity` in mode `blend`.
///
/// Tone and colour recolour what the quad draws, never the pixels underneath: that is
/// [`RectEffect`]. [`Tone::NEUTRAL`] and a colour of alpha 0 leave the texel as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quad {
    pub clip: Rect,
    pub bitmap: BitmapId,
    /// The part of the bitmap to draw, in bitmap pixels. It must lie inside the bitmap.
    pub src: Rect,
    pub sampling: Sampling,
    /// Applied to the sampled texel first.
    pub tone: Tone,
    /// Blended over the toned texel with its alpha; alpha 0 changes nothing.
    pub color: Color,
    /// 0 draws nothing, 255 draws the bitmap at its own alpha.
    pub opacity: u8,
    pub blend: BlendMode,
}

impl Quad {
    /// Whether screen pixel column `px` is the one pixel of a row that a quad in
    /// [`BlendMode::Window`] blends as in [`BlendMode::Normal`]: the last column of the quad's
    /// visible part, when that part has an odd number of columns. The visible part is what the
    /// quad covers inside its clip and a frame of `frame` pixels.
    ///
    /// This is defined only for a quad placed unscaled on whole pixels, as
    /// [`Sampling::place_at`] places it, which is how window layers are drawn. For any other
    /// quad the answer is `false`. Every backend must match this.
    pub const fn is_window_tail(&self, frame: Size, px: u16) -> bool {
        let Sampling::Affine(t) = self.sampling else {
            return false;
        };
        let one = Fixed::ONE.to_bits();
        let whole = (1 << Fixed::FRAC_BITS) - 1;
        if t.xx.to_bits() != one
            || t.xy.to_bits() != 0
            || t.yx.to_bits() != 0
            || t.yy.to_bits() != one
            || t.tx.to_bits() & whole != 0
            || t.ty.to_bits() & whole != 0
        {
            return false;
        }
        let left = -((t.tx.to_bits() >> Fixed::FRAC_BITS) as i64);
        let clip_low = self.clip.x as i64;
        let clip_high = clip_low + self.clip.width as i64;
        let mut low = if clip_low > 0 { clip_low } else { 0 };
        if left > low {
            low = left;
        }
        let mut high = if clip_high < frame.width as i64 {
            clip_high
        } else {
            frame.width as i64
        };
        if left + (self.src.width as i64) < high {
            high = left + self.src.width as i64;
        }
        px as i64 == high - 1 && (high - low) & 1 == 1
    }
}

/// How a quad maps screen pixels to the texels of its source rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sampling {
    /// A screen pixel is covered when the transform maps its centre into the source rectangle;
    /// it then takes the texel under that point.
    Affine(Transform),
    /// The source rectangle stretched into the rectangle `dest` of logical screen pixels, as
    /// the original runtime stretches: the pixel at column `i` and row `j` of `dest` takes the
    /// texel at column [`stretch_index`]`(i, `[`stretch_step`]`(src.width, dest.width))` and
    /// the row found likewise, the column counted from the right edge of the source rectangle
    /// when `mirror` is set. A `dest` without pixels draws nothing.
    Stretch { dest: Rect, mirror: bool },
}

impl Sampling {
    /// The source rectangle unscaled with its top-left corner at screen pixel (`x`, `y`).
    pub const fn place_at(x: i16, y: i16) -> Sampling {
        Sampling::Affine(Transform::place_at(x, y))
    }

    /// The texel that screen pixel (`px`, `py`) takes from a source rectangle of `width` by
    /// `height` texels, relative to that rectangle, or `None` when the pixel is not covered.
    /// Every backend must match this.
    pub const fn source_texel(
        &self,
        width: u32,
        height: u32,
        px: u16,
        py: u16,
    ) -> Option<(u32, u32)> {
        match self {
            Sampling::Affine(transform) => {
                let (u, v) = transform.source_position(px, py);
                let (column, row) = (u >> Fixed::FRAC_BITS, v >> Fixed::FRAC_BITS);
                if column >= 0 && column < width as i64 && row >= 0 && row < height as i64 {
                    Some((column as u32, row as u32))
                } else {
                    None
                }
            }
            Sampling::Stretch { dest, mirror } => {
                if !dest.contains(px as i32, py as i32) || width == 0 || height == 0 {
                    return None;
                }
                // Inside `dest`, so both are below its width and height.
                let i = (px as i64 - dest.x as i64) as u32;
                let j = (py as i64 - dest.y as i64) as u32;
                let column = stretch_index(i, stretch_step(width, dest.width));
                let row = stretch_index(j, stretch_step(height, dest.height));
                Some((if *mirror { width - 1 - column } else { column }, row))
            }
        }
    }
}

/// Largest width or height of a logical screen: the guest keeps its scene within it and the
/// compositors refuse a packet beyond it.
pub const MAX_LOGICAL_DIMENSION: u32 = 16384;

/// Number of fractional bits of a stretch step.
pub const STRETCH_FRAC_BITS: u32 = 20;

/// The step of a stretch of `src` texels into `dest` pixels: the texels one pixel advances by,
/// with [`STRETCH_FRAC_BITS`] fractional bits, rounded down. An exact ratio that is not a
/// binary fraction is therefore a little short, as in the original runtime.
///
/// # Panics
///
/// If `dest` is 0.
pub const fn stretch_step(src: u32, dest: u32) -> u64 {
    ((src as u64) << STRETCH_FRAC_BITS) / dest as u64
}

/// The texel of pixel `i` of a stretch with the step `step`, counted from the start of the
/// source: below the source length for every `i` below the destination length.
pub const fn stretch_index(i: u32, step: u64) -> u32 {
    // The product is below the source length shifted, which fits 52 bits.
    ((i as u64 * step) >> STRETCH_FRAC_BITS) as u32
}

/// How a quad combines with the pixels underneath.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    /// Adds the source to the destination, saturating at 255.
    Add,
    /// Subtracts the source from the destination, saturating at 0.
    Subtract,
    /// The blend of a window layer, `vkrgss_pixel::blend_window`. In each row, when the part of
    /// the quad that is visible has an odd number of pixels, the last of them is blended as in
    /// `Normal` instead: see [`Quad::is_window_tail`].
    Window,
}

/// A fixed-point affine map from logical screen space to the source rectangle of a quad.
///
/// For the screen pixel (`px`, `py`), with `X = px + 1/2` and `Y = py + 1/2` (its centre):
///
/// ```text
/// u = xx * X + xy * Y + tx
/// v = yx * X + yy * Y + ty
/// ```
///
/// (`u`, `v`) is a position inside the source rectangle, in bitmap pixels relative to the
/// rectangle's top-left corner. Sending the map in this direction means a backend evaluates it
/// per pixel and never has to invert a matrix.
///
/// The arithmetic is exact: each sum of products is formed without overflow and then rounded
/// towards negative infinity to 16 fractional bits before the translation is added. See
/// [`Transform::source_position`], which every backend must match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Transform {
    pub xx: Fixed,
    pub xy: Fixed,
    pub yx: Fixed,
    pub yy: Fixed,
    pub tx: Fixed,
    pub ty: Fixed,
}

impl Transform {
    /// Draws the source rectangle unscaled with its top-left corner at the origin.
    pub const IDENTITY: Transform = Transform::place_at(0, 0);

    /// Draws the source rectangle unscaled with its top-left corner at screen pixel (`x`, `y`).
    pub const fn place_at(x: i16, y: i16) -> Transform {
        Transform {
            xx: Fixed::ONE,
            xy: Fixed::ZERO,
            yx: Fixed::ZERO,
            yy: Fixed::ONE,
            tx: Self::offset(x),
            ty: Self::offset(y),
        }
    }

    /// The translation that moves screen coordinate `position` to source coordinate 0. The one
    /// value that does not fit, 32768 for `i16::MIN`, saturates.
    const fn offset(position: i16) -> Fixed {
        let bits = -(position as i64) << Fixed::FRAC_BITS;
        Fixed::from_bits(if bits > i32::MAX as i64 {
            i32::MAX
        } else {
            bits as i32
        })
    }

    /// The position (`u`, `v`) that the centre of screen pixel (`px`, `py`) maps to, as
    /// fixed-point numbers with 16 fractional bits. The texel under it is
    /// (`u >> 16`, `v >> 16`), relative to the source rectangle.
    pub const fn source_position(&self, px: u16, py: u16) -> (i64, i64) {
        let x = ((px as i128) << Fixed::FRAC_BITS) + (1 << (Fixed::FRAC_BITS - 1));
        let y = ((py as i128) << Fixed::FRAC_BITS) + (1 << (Fixed::FRAC_BITS - 1));
        let u = (self.xx.to_bits() as i128 * x + self.xy.to_bits() as i128 * y) >> Fixed::FRAC_BITS;
        let v = (self.yx.to_bits() as i128 * x + self.yy.to_bits() as i128 * y) >> Fixed::FRAC_BITS;
        (
            u as i64 + self.tx.to_bits() as i64,
            v as i64 + self.ty.to_bits() as i64,
        )
    }
}

/// Draws, inside `clip`, the tiles of priority `priority` that layer `layer` of a map holds.
///
/// The screen pixel (`X`, `Y`) shows the map pixel (`X + origin.0`, `Y + origin.1`), which lies
/// in the cell at both coordinates divided by 32, rounded down and wrapped into the size of
/// `map`: the map repeats in both directions. The cell's element of layer `layer` is a tile
/// number, and its entry in `priorities` (0 for a number the table has no entry for, an entry
/// being an element of its first row) is the tile's priority. When that equals `priority`, the
/// tile's texel for the pixel is blended over the pixel in [`BlendMode::Normal`] at opacity
/// 255; otherwise the op leaves the pixel alone, as it does for a `layer` the map does not
/// have and for a map without cells.
///
/// A tile number from 384 names the 32 by 32 block of `tileset` at column
/// `(number - 384) mod 8` and row `(number - 384) div 8`. A number from 48 to 383 names
/// pattern `number mod 48` of the autotile `number div 48 - 1`: each 16 by 16 quarter of the
/// tile is the quarter of the sheet that the pattern table names, in the frame
/// `animation_step` modulo the sheet's number of frames, where a frame is 96 pixels wide. An
/// autotile sheet at most 32 pixels high holds whole tiles instead, one per frame of 32
/// pixels. A sheet narrower than one frame has one frame. Numbers below 48 draw nothing, and
/// so does a tile whose bitmap is absent or whose texel lies outside its bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilemapLayer {
    pub clip: Rect,
    pub tileset: Option<BitmapId>,
    pub autotiles: [Option<BitmapId>; 7],
    /// The tile numbers: one element per cell and layer of the map.
    pub map: TableId,
    /// The priority of each tile number.
    pub priorities: TableId,
    /// The map pixel under the top-left pixel of the screen.
    pub origin: (i32, i32),
    /// The layer of the map, from 0 to 2.
    pub layer: u8,
    /// The priority this op draws, from 0 to 5.
    pub priority: u8,
    /// Counts the animation frames of the autotiles.
    pub animation_step: u64,
}

/// Which of the two planes of a tilemap of RGSS2 an op draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Plane {
    /// Every tile whose entry of the passages table lacks the bit [`tiles_rgss2::STAR`],
    /// with the shadows and the edges under tables.
    Lower,
    /// The tiles of the third layer whose entry has the bit.
    Upper,
}

/// The rule set a [`TilemapPlane`] is evaluated by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaneRules {
    /// The tilemap of RGSS2, as the documentation of [`TilemapPlane`] gives it.
    #[default]
    Rgss2,
    /// The tilemap of RGSS3: what [`TilemapPlane`] says of this rule set, with
    /// [`crate::tiles_rgss3`].
    Rgss3,
}

/// Draws, inside `clip`, one plane of a tilemap of RGSS2 or RGSS3:
/// [`tiles_rgss2::PLANE_COLUMNS`] by
/// [`tiles_rgss2::PLANE_ROWS`] cells of 32 by 32 pixels whose top-left pixel is the screen
/// pixel `corner`. Nothing is drawn outside the plane. The steps below are those of
/// [`PlaneRules::Rgss2`]; what [`PlaneRules::Rgss3`] changes follows them.
///
/// The cell at column `c` and row `r` of the plane shows the cell
/// (`(first_cell.0 + c) mod xsize`, `(first_cell.1 + r) mod ysize`) of `map`: the map repeats
/// in both directions. A map without cells draws nothing, a layer the map does not have is
/// empty, and layers beyond the third are not read.
///
/// What a pixel of a cell becomes is what these steps make of it, in this order. A tile is
/// resolved by [`tiles_rgss2::tile`] at `animation_step`; its entry of `passages` is the
/// element at its number, 0 when there is none or no table. The lower plane draws the tiles
/// whose entry lacks the bit [`tiles_rgss2::STAR`]; the upper plane draws those of the third
/// layer that have it.
///
/// 1. In the first [`tiles_rgss2::OVERHANG`] rows of a cell that is not in the first row of
///    the plane, [`tiles_rgss2::Tile::overhang`] of each drawn tile of the cell above, layer
///    by layer, but only in the rows before the next screen row below the cell's top whose
///    coordinate is a multiple of 8: all eight when the top is at a multiple of 8.
/// 2. Layer by layer, the [`tiles_rgss2::Tile::texels`] of each drawn tile of the cell.
/// 3. In the lower plane only, after the first layer whether its tile was drawn or not: a
///    shadow, [`tiles_rgss2::shadow`] on each colour channel, on the left half of a cell
///    whose tile of the first layer [`tiles_rgss2::takes_shadow`] while the tiles of the
///    first layer of the map cells to its left and to its upper left are both
///    [`tiles_rgss2::is_wall`]; then, in the first eight rows of a cell whose tile of the
///    first layer is neither a wall nor a table, [`tiles_rgss2::Tile::edge`] of the tile of
///    the first layer of the map cell above. These neighbours are cells of the map and are
///    not wrapped: beyond the map's edge there is no tile.
/// 4. The flash: where the element of `flash` at
///    (`(first_cell.0 + c) mod xsize`, `(first_cell.1 + r) mod ysize`), with the sizes of the
///    flash table, gives [`tiles_rgss2::flash_amounts`] at `flash_level`, these are added to
///    the channels, each limited to 255.
///
/// A texel is blended over the pixel as in [`BlendMode::Window`] at opacity 255, and as in
/// [`BlendMode::Normal`] where [`TilemapPlane::is_tail`] says so. A texel outside its sheet
/// and a tile whose sheet is absent draw nothing.
///
/// With [`PlaneRules::Rgss3`], `passages` names the flags table and a tile is resolved by
/// [`crate::tiles_rgss3::tile`] with its entry of that table, so that a table is a tile of A2 whose
/// entry has the bit [`crate::tiles_rgss3::TABLE`]. The map has [`crate::tiles_rgss3::LAYERS`] layers and
/// layers beyond the fourth are not read. The upper plane draws the tiles of the third layer
/// whose entry has the bit [`tiles_rgss2::STAR`] and the lower plane every other tile, those
/// of the first two layers with the bit among them. Step 3 is replaced by this, in the lower
/// plane only, after the second layer and before the third: [`tiles_rgss2::shadow`] on each
/// colour channel where [`crate::tiles_rgss3::shadow_covers`] says so of the cell's element of the
/// layer [`crate::tiles_rgss3::SHADOW_LAYER`]; then, in the first eight rows of a cell whose tile
/// of the second layer is no table and whose tile of the first layer is no
/// [`tiles_rgss2::is_wall`], [`tiles_rgss2::Tile::edge`] of the tile of the second layer of
/// the map cell above, which is not wrapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilemapPlane {
    pub clip: Rect,
    pub plane: Plane,
    pub rules: PlaneRules,
    /// The screen pixel of the plane's top-left pixel.
    pub corner: (i32, i32),
    /// The cell of the map in the plane's first column and row.
    pub first_cell: (u32, u32),
    /// Counts the animation frames of the autotiles of the first sheet.
    pub animation_step: u64,
    /// What a nibble of a flash entry is multiplied by.
    pub flash_level: u8,
    /// The tile numbers: one element per cell and layer of the map.
    pub map: TableId,
    /// The flags of each tile number: the passages table of RGSS2, the flags table of
    /// RGSS3.
    pub passages: Option<TableId>,
    /// The flash entry of each cell.
    pub flash: Option<TableId>,
    /// The sheets, in the order of `Tilemap#bitmaps`.
    pub bitmaps: [Option<BitmapId>; tiles_rgss2::SHEETS],
}

impl TilemapPlane {
    /// Whether screen pixel column `px` is the one pixel that a row of a transfer blends as
    /// in [`BlendMode::Normal`]: the last column of the row's visible part, when that part has
    /// an odd number of columns. The row covers `width` screen columns from `left`, the width
    /// being [`tiles_rgss2::Tile::span`] and `left` the left edge of the tile or of the
    /// quarter that `px` is in; its visible part is what lies inside the clip and a frame of
    /// `frame` pixels, and `px` is a column of it. Every backend must match this.
    pub const fn is_tail(&self, frame: Size, left: i64, width: u32, px: u16) -> bool {
        let clip_low = self.clip.x as i64;
        let clip_high = clip_low + self.clip.width as i64;
        let mut low = if clip_low > 0 { clip_low } else { 0 };
        if left > low {
            low = left;
        }
        let mut high = if clip_high < frame.width as i64 {
            clip_high
        } else {
            frame.width as i64
        };
        if left + (width as i64) < high {
            high = left + width as i64;
        }
        px as i64 == high - 1 && (high - low) & 1 == 1
    }
}

/// Changes the pixels already present inside `clip`: first the tone, then the colour blended
/// over the result with the colour's alpha. No bitmap is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RectEffect {
    /// The rectangle the effect applies to.
    pub clip: Rect,
    pub tone: Tone,
    pub color: Color,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_reports_kind_and_clip() {
        let clip = Rect::new(1, 2, 3, 4);
        let quad = Quad {
            clip,
            bitmap: BitmapId::new(0, 0),
            src: Rect::new(0, 0, 1, 1),
            sampling: Sampling::Affine(Transform::IDENTITY),
            tone: Tone::NEUTRAL,
            color: Color::default(),
            opacity: 255,
            blend: BlendMode::Normal,
        };
        assert_eq!(quad.tone, Tone::new(0, 0, 0, 0));
        assert_eq!(quad.color.a, 0);
        let effect = RectEffect {
            clip,
            tone: Tone::NEUTRAL,
            color: Color::default(),
        };
        let cases = [
            (
                Op::Clear {
                    clip,
                    color: Color::default(),
                },
                OpKind::Clear,
                "Clear",
            ),
            (Op::Quad(quad), OpKind::Quad, "Quad"),
            (Op::RectEffect(effect), OpKind::RectEffect, "RectEffect"),
            (
                Op::TilemapLayer(TilemapLayer {
                    clip,
                    tileset: None,
                    autotiles: [None; 7],
                    map: TableId::new(0),
                    priorities: TableId::new(1),
                    origin: (0, 0),
                    layer: 0,
                    priority: 0,
                    animation_step: 0,
                }),
                OpKind::TilemapLayer,
                "TilemapLayer",
            ),
            (
                Op::TilemapPlane(TilemapPlane {
                    clip,
                    plane: Plane::Upper,
                    rules: PlaneRules::Rgss2,
                    corner: (0, 0),
                    first_cell: (0, 0),
                    animation_step: 0,
                    flash_level: 4,
                    map: TableId::new(0),
                    passages: None,
                    flash: None,
                    bitmaps: [None; 9],
                }),
                OpKind::TilemapPlane,
                "TilemapPlane",
            ),
        ];
        for (op, kind, name) in cases {
            assert_eq!(op.kind(), kind);
            assert_eq!(op.clip(), clip);
            assert_eq!(kind.to_string(), name);
        }
    }

    #[test]
    fn frame_rate_accepts_exactly_10_to_120() {
        assert_eq!(FrameRate::new(9), Err(FrameRateOutOfRange { found: 9 }));
        assert_eq!(FrameRate::new(10), Ok(FrameRate::MIN));
        assert_eq!(FrameRate::new(120), Ok(FrameRate::MAX));
        assert_eq!(FrameRate::new(121), Err(FrameRateOutOfRange { found: 121 }));
        for fps in 0..=u8::MAX {
            let rate = FrameRate::new(fps);
            assert_eq!(rate.is_ok(), (10..=120).contains(&fps), "{fps}");
            assert_eq!(FrameRate::try_from(fps), rate);
            if let Ok(rate) = rate {
                assert_eq!(rate.get(), fps);
                assert_eq!(u8::from(rate), fps);
            }
        }
        assert_eq!(
            FrameRate::new(9).unwrap_err().to_string(),
            "frame rate 9 is outside 10 to 120"
        );
    }

    #[test]
    fn frame_rate_defaults_to_40() {
        assert_eq!(FrameRate::default().get(), 40);
        assert_eq!(FrameRate::DEFAULT, FrameRate::default());
        assert_eq!(FrameRate::DEFAULT.to_string(), "40");
    }

    #[test]
    fn frame_rate_clamps_like_graphics_frame_rate() {
        assert_eq!(FrameRate::clamped(i64::MIN), FrameRate::MIN);
        assert_eq!(FrameRate::clamped(-1), FrameRate::MIN);
        assert_eq!(FrameRate::clamped(9), FrameRate::MIN);
        assert_eq!(FrameRate::clamped(10), FrameRate::MIN);
        assert_eq!(FrameRate::clamped(11).get(), 11);
        assert_eq!(FrameRate::clamped(60).get(), 60);
        assert_eq!(FrameRate::clamped(119).get(), 119);
        assert_eq!(FrameRate::clamped(120), FrameRate::MAX);
        assert_eq!(FrameRate::clamped(121), FrameRate::MAX);
        assert_eq!(
            FrameRate::clamped(376),
            FrameRate::MAX,
            "376 is 120 mod 256"
        );
        assert_eq!(FrameRate::clamped(i64::MAX), FrameRate::MAX);
    }

    fn texel(transform: &Transform, px: u16, py: u16) -> (i64, i64) {
        let (u, v) = transform.source_position(px, py);
        (u >> 16, v >> 16)
    }

    #[test]
    fn placement_maps_pixels_to_texels_one_to_one() {
        let transform = Transform::place_at(10, 20);
        assert_eq!(texel(&transform, 10, 20), (0, 0));
        assert_eq!(texel(&transform, 13, 25), (3, 5));
        assert_eq!(texel(&transform, 9, 20), (-1, 0));
        assert_eq!(texel(&Transform::place_at(-4, -6), 0, 0), (4, 6));
        assert_eq!(texel(&Transform::IDENTITY, 7, 8), (7, 8));
    }

    #[test]
    fn half_scale_map_doubles_the_image() {
        let transform = Transform {
            xx: Fixed::HALF,
            yy: Fixed::HALF,
            ..Transform::IDENTITY
        };
        assert_eq!(texel(&transform, 0, 0), (0, 0));
        assert_eq!(texel(&transform, 1, 1), (0, 0));
        assert_eq!(texel(&transform, 2, 3), (1, 1));
        assert_eq!(texel(&transform, 5, 4), (2, 2));
    }

    #[test]
    fn stretch_steps_round_down() {
        // 16 texels into 24 pixels: the exact ratio 2/3 is not a binary fraction.
        let step = stretch_step(16, 24);
        let texels: Vec<u32> = (0..6).map(|i| stretch_index(i, step)).collect();
        assert_eq!(texels, [0, 0, 1, 1, 2, 3]);
        // An exact binary ratio is exact.
        let step = stretch_step(8, 16);
        assert!((0..16).all(|i| stretch_index(i, step) == i / 2));
        let step = stretch_step(8, 8);
        assert!((0..8).all(|i| stretch_index(i, step) == i));
    }

    #[test]
    fn stretch_indices_stay_inside_the_source_at_the_extremes() {
        for (src, dest) in [
            (u32::MAX, u32::MAX),
            (u32::MAX, 1),
            (1, u32::MAX),
            (5000, 4097),
            (4097, 100_000),
        ] {
            let step = stretch_step(src, dest);
            assert!(stretch_index(dest - 1, step) < src, "{src} into {dest}");
            assert_eq!(stretch_index(0, step), 0);
        }
    }

    #[test]
    fn stretch_sampling_covers_its_destination_only() {
        let sampling = Sampling::Stretch {
            dest: Rect::new(10, 20, 8, 2),
            mirror: false,
        };
        assert_eq!(sampling.source_texel(4, 2, 10, 20), Some((0, 0)));
        assert_eq!(sampling.source_texel(4, 2, 17, 21), Some((3, 1)));
        for (px, py) in [(9, 20), (18, 20), (10, 19), (10, 22)] {
            assert_eq!(sampling.source_texel(4, 2, px, py), None);
        }
        let mirrored = Sampling::Stretch {
            dest: Rect::new(10, 20, 8, 2),
            mirror: true,
        };
        assert_eq!(mirrored.source_texel(4, 2, 10, 20), Some((3, 0)));
        assert_eq!(mirrored.source_texel(4, 2, 17, 21), Some((0, 1)));
        // No pixels, or no texels: nothing.
        let empty = Sampling::Stretch {
            dest: Rect::new(0, 0, 0, 5),
            mirror: false,
        };
        assert_eq!(empty.source_texel(4, 2, 0, 0), None);
        assert_eq!(sampling.source_texel(0, 2, 10, 20), None);
    }

    #[test]
    fn affine_sampling_takes_the_texel_under_the_pixel_centre() {
        let sampling = Sampling::place_at(10, 20);
        assert_eq!(sampling.source_texel(4, 4, 13, 22), Some((3, 2)));
        assert_eq!(sampling.source_texel(4, 4, 14, 22), None);
        assert_eq!(sampling.source_texel(4, 4, 9, 22), None);
    }

    #[test]
    fn extreme_transforms_do_not_overflow() {
        let max = Fixed::from_bits(i32::MAX);
        let min = Fixed::from_bits(i32::MIN);
        let transform = Transform {
            xx: max,
            xy: max,
            yx: min,
            yy: min,
            tx: max,
            ty: min,
        };
        let (u, v) = transform.source_position(u16::MAX, u16::MAX);
        assert!(u > 0);
        assert!(v < 0);
    }

    #[test]
    fn window_tail_is_the_last_column_of_an_odd_visible_part() {
        let frame = Size::new(40, 30);
        let quad = |clip, width, sampling| Quad {
            clip,
            bitmap: BitmapId::new(0, 0),
            src: Rect::new(0, 0, width, 4),
            sampling,
            tone: Tone::NEUTRAL,
            color: Color::default(),
            opacity: 255,
            blend: BlendMode::Window,
        };
        let screen = Rect::from_size(frame);
        let tails = |quad: Quad| -> Vec<u16> {
            (0..frame.width as u16)
                .filter(|&px| quad.is_window_tail(frame, px))
                .collect()
        };
        // Five columns from 10: the fifth. Six columns: none.
        assert_eq!(tails(quad(screen, 5, Sampling::place_at(10, 2))), [14]);
        assert_eq!(tails(quad(screen, 6, Sampling::place_at(10, 2))), []);
        // Cut to an odd part by the clip, on the right and on the left.
        let clip = Rect::new(0, 0, 13, 30);
        assert_eq!(tails(quad(clip, 6, Sampling::place_at(10, 2))), [12]);
        let clip = Rect::new(11, 0, 29, 30);
        assert_eq!(tails(quad(clip, 6, Sampling::place_at(10, 2))), [15]);
        // Cut by the frame: 37..40 and 0..3.
        assert_eq!(tails(quad(screen, 6, Sampling::place_at(37, 2))), [39]);
        assert_eq!(tails(quad(screen, 6, Sampling::place_at(-3, 2))), [2]);
        // A clip that reaches beyond the frame does not count beyond it.
        let clip = Rect::new(-100, -100, 1000, 1000);
        assert_eq!(tails(quad(clip, 6, Sampling::place_at(37, 2))), [39]);
        // Not placed unscaled on whole pixels: no such pixel.
        let half = Transform {
            tx: Fixed::from_bits(-(10 << 16) + 1),
            ..Transform::place_at(10, 2)
        };
        assert_eq!(tails(quad(screen, 5, Sampling::Affine(half))), []);
        let scaled = Transform {
            xx: Fixed::HALF,
            ..Transform::place_at(10, 2)
        };
        assert_eq!(tails(quad(screen, 5, Sampling::Affine(scaled))), []);
        let stretch = Sampling::Stretch {
            dest: Rect::new(10, 2, 5, 4),
            mirror: false,
        };
        assert_eq!(tails(quad(screen, 5, stretch)), []);
    }

    #[test]
    fn plane_tail_is_the_last_column_of_an_odd_visible_row() {
        let frame = Size::new(40, 30);
        let plane = |clip| TilemapPlane {
            clip,
            plane: Plane::Lower,
            rules: PlaneRules::Rgss2,
            corner: (0, 0),
            first_cell: (0, 0),
            animation_step: 0,
            flash_level: 4,
            map: TableId::new(0),
            passages: None,
            flash: None,
            bitmaps: [None; 9],
        };
        let tails = |plane: TilemapPlane, left, width| -> Vec<u16> {
            (0..frame.width as u16)
                .filter(|&px| plane.is_tail(frame, left, width, px))
                .collect()
        };
        let screen = plane(Rect::from_size(frame));
        // Whole rows of 16 and 32 pixels have no such pixel.
        assert_eq!(tails(screen, 3, 16), []);
        assert_eq!(tails(screen, 3, 32), []);
        // Cut by the frame on the left and on the right.
        assert_eq!(tails(screen, -13, 16), [2]);
        assert_eq!(tails(screen, -12, 16), []);
        assert_eq!(tails(screen, 27, 16), [39]);
        // Cut by the clip.
        let clipped = plane(Rect::new(5, 0, 20, 30));
        assert_eq!(tails(clipped, 0, 16), [15]);
        assert_eq!(tails(clipped, 10, 16), [24]);
        assert_eq!(tails(clipped, 6, 16), []);
    }
}
