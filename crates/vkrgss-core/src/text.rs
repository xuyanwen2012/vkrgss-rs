//! Text: the rasteriser interface, and drawing and measuring text in a bitmap.
//!
//! This crate lays text out and blends it; measuring a string and turning it into glyph
//! coverage is the job of a [`TextRasterizer`] that the embedder supplies.
//!
//! The layout rules are those measured on the original runtime. Text is placed by its drawn
//! width, the measured width plus the last glyph's right overhang. Text wider than its
//! rectangle is not scaled: it is set again in a narrower form of the font, at the first of
//! four steps that fits, or of five in RGSS2 ([`Squeeze`]). A font with a shadow, which
//! RGSS1 has not, draws the text twice: a black layer one pixel to the right and down, then
//! the text.
//!
//! [`TextRules`] names the runtime whose rules a draw follows. RGSS3 differs from the two
//! others in most of the above: it squeezes in one step ([`squeeze_width_rgss3`]), blends the
//! whole rectangle it was given and not the cell of the text, and draws a font with an
//! outline in four more passes; see [`Bitmaps::draw_text_as`].

use vkrgss_pixel::{blend_normal, effective_alpha};
use vkrgss_proto::BitmapId;

use crate::bitmap::{Area, Bitmap};
use crate::{BitmapError, Bitmaps, Font, Rect};

/// Glyph coverage of a piece of text: one byte per pixel in row order, 0 for untouched and
/// 255 for fully covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    width: u32,
    height: u32,
    coverage: Vec<u8>,
}

impl Mask {
    /// A mask of `width` by `height` pixels.
    ///
    /// # Panics
    ///
    /// If `coverage` does not hold exactly one byte per pixel.
    pub fn new(width: u32, height: u32, coverage: Vec<u8>) -> Mask {
        assert_eq!(
            coverage.len() as u64,
            u64::from(width) * u64::from(height),
            "a mask holds one byte per pixel"
        );
        Mask {
            width,
            height,
            coverage,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// All coverage values in row order.
    pub fn coverage(&self) -> &[u8] {
        &self.coverage
    }

    /// The coverage at (`x`, `y`).
    ///
    /// # Panics
    ///
    /// If the position is outside the mask.
    pub fn get(&self, x: u32, y: u32) -> u8 {
        assert!(x < self.width && y < self.height, "outside the mask");
        self.coverage[y as usize * self.width as usize + x as usize]
    }
}

/// How a string is set in a font: what placing and squeezing it needs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextMetrics {
    /// The height of the text, which is not always the font's size.
    pub height: u32,
    /// The advance of each character, in order. Empty for an empty string.
    pub advances: Vec<u32>,
    /// How far the first glyph reaches left of the pen.
    pub left_overhang: u32,
    /// How far the last glyph reaches right of its advance.
    pub right_overhang: u32,
    /// The font's average character width, at least 1.
    pub average_width: u32,
    /// The pixels a synthesised bold adds to every advance and to the average width; they
    /// are not scaled when text is squeezed.
    pub emboldened: u32,
}

impl TextMetrics {
    /// The measured width: the advances plus the first glyph's left overhang.
    pub fn width(&self) -> u32 {
        total(&self.advances).saturating_add(self.left_overhang)
    }
}

fn total(advances: &[u32]) -> u32 {
    advances.iter().fold(0, |sum, &a| sum.saturating_add(a))
}

/// Measures text and turns it into glyph coverage. Implemented for real fonts outside this
/// crate.
pub trait TextRasterizer {
    /// How `text` is set in `font`.
    fn metrics(&self, font: &Font, text: &str) -> TextMetrics;

    /// The coverage of `text` in `font` with the characters set `advances` apart, one advance
    /// per character, and every glyph `condense.0 / condense.1` as wide as it naturally is.
    ///
    /// The mask is the cell of the text: as high as the text, and as wide as the advances
    /// plus the overhangs of the first and the last glyph as drawn. With the advances of
    /// [`TextRasterizer::metrics`] and no condensing these are the overhangs it reports.
    fn rasterize(
        &self,
        font: &Font,
        text: &str,
        advances: &[u32],
        condense: Option<(u32, u32)>,
    ) -> Mask;
}

/// Horizontal placement of text inside its rectangle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// The distance of the left edge of text `drawn` wide from that of a rectangle `width` wide
/// that holds it.
fn align_offset(drawn: u32, width: u32, align: Align) -> u32 {
    let free = width.saturating_sub(drawn);
    match align {
        Align::Left => 0,
        Align::Center => free / 2,
        Align::Right => free,
    }
}

/// The step numbers of squeezing, in the order they are tried: 90, 80, 70 and 60 percent.
pub const SQUEEZE_STEPS: [u32; 4] = [9, 8, 7, 6];

/// Which runtime's rule narrows a text that is wider than its rectangle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Squeeze {
    /// The four steps of RGSS1: [`SQUEEZE_STEPS`].
    #[default]
    Rgss1,
    /// The loop of RGSS2, which gives five steps: 90, 80, 70, 60 and 50 percent.
    Rgss2,
}

/// The runtime whose measured rules a text is drawn by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextRules {
    /// RGSS1: the text cell, squeezed by [`Squeeze::Rgss1`].
    #[default]
    Rgss1,
    /// RGSS2: the text cell, squeezed by [`Squeeze::Rgss2`].
    Rgss2,
    /// RGSS3: the work rectangle, the one-step squeeze and the outline.
    Rgss3,
}

impl From<Squeeze> for TextRules {
    fn from(squeeze: Squeeze) -> TextRules {
        match squeeze {
            Squeeze::Rgss1 => TextRules::Rgss1,
            Squeeze::Rgss2 => TextRules::Rgss2,
        }
    }
}

/// The average character width and the height of the stock System font, whose metrics RGSS3
/// squeezes by instead of those of the text's font.
///
/// PLACEHOLDER: the values of Wine, under which the original runtime was measured. A Windows
/// of another language has another System font.
const SYSTEM_FONT_AVERAGE_WIDTH: f64 = 7.0;
const SYSTEM_FONT_HEIGHT: f64 = 16.0;

/// The width and height of the work surface of RGSS3, which bound the work rectangle of a
/// draw.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, edge probes (a rectangle 800
/// wide touches 640 columns, one 1200 high 480 rows).
const WORK_SURFACE: (i64, i64) = (640, 480);

/// The average character width RGSS3 asks its font for when a text of the drawn width
/// `drawn` (the outline's 2 included) at the font size `size` goes into a rectangle `width`
/// wide; `None` when the text fits and is not squeezed.
///
/// The original computes, in double precision and in this order, `r = width / drawn - 0.15`,
/// at least 0.6, and `trunc(size * r * A / H)` with the average character width `A` and the
/// height `H` of the System font. There is one step and no check that the text then fits.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1 and RGSS300 3.0.0.1, gdi probe
/// (24 sweeps, 2330 rectangle widths); the code is at 0x1001f360 of version 3.0.1.1.
pub fn squeeze_width_rgss3(size: u32, width: u32, drawn: u32) -> Option<u32> {
    if drawn <= width {
        return None;
    }
    let mut r = f64::from(width) / f64::from(drawn) - 0.15;
    if r < 0.6 {
        r = 0.6;
    }
    // `as` truncates toward zero.
    Some((f64::from(size) * r * SYSTEM_FONT_AVERAGE_WIDTH / SYSTEM_FONT_HEIGHT) as u32)
}

impl TextMetrics {
    /// The advances of this text squeezed to step `step` (one of [`SQUEEZE_STEPS`]) of a
    /// font of size `size`, and the fraction of its natural width each glyph is condensed
    /// to: what [`TextRasterizer::rasterize`] is asked for when the text is too wide.
    ///
    /// The narrower font has the average character width
    /// `k = (size * step * ave) div (10 * th)` for the natural average `ave` and the text
    /// height `th`, and each advance is `ceil(natural * k / ave)`. The pixel a synthesised
    /// bold adds is taken off before and put back after, and the division is by the average
    /// without it.
    ///
    /// PLACEHOLDER: the rounding up and the factor `size / th` are those of Wine's GDI,
    /// under which the original runtime was measured; whether they are the runtime's or
    /// Wine's is not determined.
    pub fn squeezed(&self, size: u32, step: u32) -> (Vec<u32>, (u32, u32)) {
        let average = u64::from(self.average_width.max(1));
        let height = u64::from(self.height.max(1));
        self.narrowed(u64::from(size) * u64::from(step) * average / (10 * height))
    }

    /// The narrower forms of this text in a font of size `size` that RGSS2 tries, in order:
    /// the advances and the condensing of each, as [`TextMetrics::squeezed`] gives them.
    ///
    /// The original runs `r = 1.0; while the text is wider than the rectangle and r > 0.6:
    /// r -= 0.1` in double precision and asks for a font of the average character width
    /// `trunc(size * r * ave / th)` at each turn. The values of `r` are 0.9, 0.8,
    /// 0.7000000000000001, 0.6000000000000001 and 0.5000000000000001: the fourth is still
    /// above 0.6, which gives a fifth step where RGSS1 has four. The caller stops at the
    /// first form that fits; the last is what it draws when none does.
    ///
    /// Provenance: oracle measurement 2026-10-12, RGSS202E, squeeze probe (36 sweeps, every
    /// rectangle width); the loop is at 0x100059ef of version 2.0.2.1. How an advance
    /// follows from the average width is the placeholder of [`TextMetrics::squeezed`].
    pub fn squeezed_rgss2(&self, size: u32) -> Vec<(Vec<u32>, (u32, u32))> {
        let average = f64::from(self.average_width.max(1));
        let height = f64::from(self.height.max(1));
        let mut forms = Vec::new();
        let mut r = 1.0_f64;
        while r > 0.6 {
            r -= 0.1;
            // `as` truncates toward zero.
            forms.push(self.narrowed((f64::from(size) * r * average / height) as u64));
        }
        forms
    }

    /// The narrower forms `squeeze` tries for this text in a font of size `size`, in order.
    fn squeezed_forms(&self, size: u32, squeeze: Squeeze) -> Vec<(Vec<u32>, (u32, u32))> {
        match squeeze {
            Squeeze::Rgss1 => SQUEEZE_STEPS
                .iter()
                .map(|&step| self.squeezed(size, step))
                .collect(),
            Squeeze::Rgss2 => self.squeezed_rgss2(size),
        }
    }

    /// This text in a font of the average character width `k`: see
    /// [`TextMetrics::squeezed`].
    fn narrowed(&self, k: u64) -> (Vec<u32>, (u32, u32)) {
        let bold = u64::from(self.emboldened);
        let average = u64::from(self.average_width.max(1));
        let regular = average.saturating_sub(bold).max(1);
        let advances = self
            .advances
            .iter()
            .map(|&natural| {
                let natural = u64::from(natural).saturating_sub(bold);
                ((natural * k).div_ceil(regular) + bold).min(u64::from(u32::MAX)) as u32
            })
            .collect();
        let fraction = |value: u64| value.min(u64::from(u32::MAX)) as u32;
        (advances, (fraction(k), fraction(regular)))
    }
}

/// Blends one layer of a text onto `bitmap`: the part `cell` of the text cell `mask`, whose
/// top left corner is at `origin`, moved by `shift` pixels to the right and down, in the
/// colour `rgb` at the alpha `color_alpha`, as a block transfer blends. `cell` is
/// `(left, top, right, bottom)` in the coordinates of the bitmap before the shift; the layer
/// is clipped by the bitmap. Returns the area it touched.
fn blend_layer(
    bitmap: &mut Bitmap,
    mask: &Mask,
    origin: (i64, i64),
    cell: (i64, i64, i64, i64),
    shift: i64,
    rgb: [u8; 3],
    color_alpha: u8,
) -> Option<Area> {
    let span = |first: i64, end: i64, size: usize| {
        let first = (first + shift).max(0);
        let end = (end + shift).min(size as i64);
        (first < end).then_some((first as usize, end as usize))
    };
    let (left, right) = span(cell.0, cell.2, bitmap.width)?;
    let (top, bottom) = span(cell.1, cell.3, bitmap.height)?;
    for y in top..bottom {
        let mask_y = (y as i64 - shift - origin.1) as u32;
        for x in left..right {
            let mask_x = (x as i64 - shift - origin.0) as u32;
            let alpha = effective_alpha(mask.get(mask_x, mask_y), color_alpha);
            let source = [rgb[0], rgb[1], rgb[2], alpha];
            let blended = blend_normal(bitmap.pixel(x, y), source, 255);
            bitmap.put(x, y, blended);
        }
    }
    Some(Area {
        left,
        top,
        right,
        bottom,
    })
}

/// The work rectangle of a draw of RGSS3: the coverage of a text put at a place of the
/// bitmap, and the pixels of it a pass blends.
struct WorkRectangle<'a> {
    mask: &'a Mask,
    /// Where pixel (0, 0) of the mask lies in the bitmap.
    origin: (i64, i64),
    /// The pixels `(u, v)` a pass reads, `columns.0 <= u < columns.1` and
    /// `rows.0 <= v < rows.1`; they may lie outside the mask, where the coverage is 0.
    columns: (i64, i64),
    rows: (i64, i64),
}

impl WorkRectangle<'_> {
    /// One pass of RGSS3: every pixel of the work rectangle blended onto the pixel `shift`
    /// away from its place, in the colour `rgb` at the alpha `alpha`, as a block transfer
    /// blends, also where the coverage is 0. The pass is cut by the bitmap, and a pass of
    /// alpha 0 does nothing. Returns the area it touched.
    fn pass(
        &self,
        bitmap: &mut Bitmap,
        shift: (i64, i64),
        rgb: [u8; 3],
        alpha: u8,
    ) -> Option<Area> {
        if alpha == 0 {
            return None;
        }
        // The pixels of the bitmap the pass reaches, and from them the pixels it reads.
        let span = |range: (i64, i64), origin: i64, shift: i64, size: usize| {
            let first = (origin + range.0 + shift).max(0);
            let end = (origin + range.1 + shift).min(size as i64);
            (first < end).then_some((first as usize, end as usize))
        };
        let (left, right) = span(self.columns, self.origin.0, shift.0, bitmap.width)?;
        let (top, bottom) = span(self.rows, self.origin.1, shift.1, bitmap.height)?;
        let (mask_width, mask_height) = (i64::from(self.mask.width), i64::from(self.mask.height));
        for y in top..bottom {
            let v = y as i64 - shift.1 - self.origin.1;
            for x in left..right {
                let u = x as i64 - shift.0 - self.origin.0;
                let coverage = if u < mask_width && v < mask_height {
                    self.mask.get(u as u32, v as u32)
                } else {
                    0
                };
                let source = [rgb[0], rgb[1], rgb[2], effective_alpha(coverage, alpha)];
                let blended = blend_normal(bitmap.pixel(x, y), source, 255);
                bitmap.put(x, y, blended);
            }
        }
        Some(Area {
            left,
            top,
            right,
            bottom,
        })
    }
}

impl Bitmaps {
    /// Draws `text` inside `rect` with the bitmap's font by the rules of a runtime.
    ///
    /// With [`TextRules::Rgss1`] and [`TextRules::Rgss2`] this is
    /// [`Bitmaps::draw_text_squeezed`] with the squeeze of that runtime. With
    /// [`TextRules::Rgss3`] the draw follows the rules measured on the original runtime of
    /// VX Ace, all in integers that truncate toward zero:
    ///
    /// - The text is placed by `E` and `th`: its drawn width (the measured width plus the
    ///   last glyph's right overhang) and its height, each 2 larger when the font has the
    ///   outline. A text whose `E` exceeds the width `w` of the rectangle is set again once,
    ///   in a font of the average character width [`squeeze_width_rgss3`] gives, and `E` is
    ///   then the width of that; it is drawn fitting or not. An `E` of 0 or less draws
    ///   nothing.
    /// - Its top is `y + max(0, (h - th) / 2)`. Its left is `x` when left aligned,
    ///   `x + max(0, (w - E + o) / 2)` when centred, with `o` 1 for a font with the outline,
    ///   and `max(x, x + w - E)` when right aligned.
    /// - What is blended is the work rectangle, not the cell of the text: the pixels
    ///   `(u, v)` with `0 <= u < min(w, 640)` and `0 <= v < min(h, 480)` at
    ///   `(left + u, top + v)`, with the coverage the text has there and 0 where it has
    ///   none. It reaches right of the rectangle when the text is centred or right aligned
    ///   and below it when the text is centred vertically; only the bitmap cuts it.
    /// - A pass at `(dx, dy)` in a colour at an alpha `a` blends every such pixel onto the
    ///   pixel `(dx, dy)` away, as a block transfer blends ([`vkrgss_pixel::blend_normal`]),
    ///   with the source alpha `effective_alpha(coverage, a)`; a pass of alpha 0 does
    ///   nothing. Without the outline the passes are the shadow, at (1, 1) in black, if the
    ///   font has one, and the text at (0, 0), both at the alpha of the font colour.
    /// - With the outline only the pixels with `1 <= u < w - 1` and `1 <= v < h - 1` are
    ///   blended, the mask staying where it is, and four passes come first, in the out
    ///   colour at the alpha `out alpha * colour alpha / 255`: at (-1, -1), (1, -1), (1, 1)
    ///   and (-1, 1). Then the shadow, then the text.
    ///
    /// A rectangle without width or height and an empty string draw nothing. Nothing is
    /// kept between draws.
    ///
    /// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1 and RGSS300 3.0.0.1, draw
    /// probe (1136 draws, every pixel as predicted from the glyph coverage) and edge probes.
    pub fn draw_text_as(
        &mut self,
        id: BitmapId,
        rect: Rect,
        text: &str,
        align: Align,
        rules: TextRules,
        rasterizer: &dyn TextRasterizer,
    ) -> Result<(), BitmapError> {
        match rules {
            TextRules::Rgss1 => {
                self.draw_text_in_cell(id, rect, text, align, Squeeze::Rgss1, rasterizer)
            }
            TextRules::Rgss2 => {
                self.draw_text_in_cell(id, rect, text, align, Squeeze::Rgss2, rasterizer)
            }
            TextRules::Rgss3 => self.draw_text_rgss3(id, rect, text, align, rasterizer),
        }
    }

    /// [`Bitmaps::draw_text_as`] with [`TextRules::Rgss3`].
    fn draw_text_rgss3(
        &mut self,
        id: BitmapId,
        rect: Rect,
        text: &str,
        align: Align,
        rasterizer: &dyn TextRasterizer,
    ) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        if rect.width <= 0 || rect.height <= 0 {
            return Ok(());
        }
        let [red, green, blue, color_alpha] = bitmap.font.color.to_pixel();
        // Every pass has the alpha of the colour as a factor.
        if color_alpha == 0 {
            return Ok(());
        }
        let outline = bitmap.font.outline;
        let shadow = bitmap.font.shadow;
        let [out_red, out_green, out_blue, out_alpha] = bitmap.font.out_color.to_pixel();
        let metrics = rasterizer.metrics(&bitmap.font, text);
        if metrics.advances.is_empty() {
            return Ok(());
        }

        // The rectangle is not empty here, so its sizes are positive.
        let width = rect.width.unsigned_abs();
        let grown = if outline { 2 } else { 0 };
        let natural = metrics
            .width()
            .saturating_add(metrics.right_overhang)
            .saturating_add(grown);
        let size = bitmap.font.size.max(0).unsigned_abs();
        let mask = match squeeze_width_rgss3(size, width, natural) {
            None => rasterizer.rasterize(&bitmap.font, text, &metrics.advances, None),
            Some(asked) => {
                let (advances, condense) = metrics.narrowed(u64::from(asked));
                rasterizer.rasterize(&bitmap.font, text, &advances, Some(condense))
            }
        };
        let drawn = i64::from(mask.width) + i64::from(grown);
        if drawn <= 0 {
            return Ok(());
        }
        let text_height = i64::from(metrics.height) + i64::from(grown);

        let (x, y) = (i64::from(rect.x), i64::from(rect.y));
        let (w, h) = (i64::from(rect.width), i64::from(rect.height));
        let left = match align {
            Align::Left => x,
            Align::Center => x + ((w - drawn + i64::from(outline)) / 2).max(0),
            Align::Right => x.max(x + w - drawn),
        };
        let top = y + ((h - text_height) / 2).max(0);

        let border = i64::from(outline);
        let work = WorkRectangle {
            mask: &mask,
            origin: (left, top),
            columns: (border, w.min(WORK_SURFACE.0).min(w - border)),
            rows: (border, h.min(WORK_SURFACE.1).min(h - border)),
        };
        let mut touched: Option<Area> = None;
        let mut pass = |shift: (i64, i64), rgb: [u8; 3], alpha: u8| {
            if let Some(area) = work.pass(bitmap, shift, rgb, alpha) {
                touched = Some(touched.map_or(area, |all| all.union(area)));
            }
        };
        if outline {
            let alpha = (u32::from(out_alpha) * u32::from(color_alpha) / 255) as u8;
            for shift in [(-1, -1), (1, -1), (1, 1), (-1, 1)] {
                pass(shift, [out_red, out_green, out_blue], alpha);
            }
        }
        if shadow {
            pass((1, 1), [0, 0, 0], color_alpha);
        }
        pass((0, 0), [red, green, blue], color_alpha);
        if let Some(area) = touched {
            bitmap.mark(area);
        }
        Ok(())
    }

    /// Draws `text` inside `rect` with the bitmap's font, squeezed by the rule of RGSS1:
    /// [`Bitmaps::draw_text_squeezed`] with [`Squeeze::Rgss1`].
    pub fn draw_text(
        &mut self,
        id: BitmapId,
        rect: Rect,
        text: &str,
        align: Align,
        rasterizer: &dyn TextRasterizer,
    ) -> Result<(), BitmapError> {
        self.draw_text_squeezed(id, rect, text, align, Squeeze::Rgss1, rasterizer)
    }

    /// Draws `text` inside `rect` with the bitmap's font.
    ///
    /// The text cell is as high as the text and as wide as the text is drawn: its measured
    /// width plus the last glyph's right overhang. Its top is at
    /// `y + max(0, (h - th) div 2)` for a text height `th`, so a rectangle lower than the
    /// text shows the top of the cell. It is placed horizontally by `align`, with its drawn
    /// width.
    ///
    /// Text drawn wider than the rectangle is squeezed: set again with the advances and
    /// condensed glyphs of the first step of `squeeze` whose drawn width fits, and aligned
    /// by that width: 90, 80, 70 and 60 percent in RGSS1, and 50 percent after them in RGSS2
    /// ([`TextMetrics::squeezed_rgss2`]). Text that does not fit at the last step is drawn
    /// at that step from the rectangle's left edge. Everything outside the rectangle or the
    /// bitmap is clipped.
    ///
    /// The whole cell is blended as a block transfer blends ([`vkrgss_pixel::blend_normal`]):
    /// each mask value `c` draws the font's colour with alpha
    /// `effective_alpha(c, colour alpha)`, also where that is 0, which gives a transparent
    /// pixel of the cell the font's colour. A font colour of alpha 0 draws nothing.
    ///
    /// A font with a shadow ([`Font::shadow`], RGSS2) first blends the same part of the cell
    /// in black, with the same alpha, one pixel to the right and one down, and then the
    /// text over it. The shadow is clipped by the bitmap only: it may lie one pixel right of
    /// or below the rectangle, and a part of the cell left of or above the bitmap still
    /// casts its shadow into it.
    ///
    /// Provenance of the shadow: oracle measurement 2026-10-12, RGSS202E, text probe 2 (936
    /// shadowed draws, every pixel as predicted from the glyph coverage).
    pub fn draw_text_squeezed(
        &mut self,
        id: BitmapId,
        rect: Rect,
        text: &str,
        align: Align,
        squeeze: Squeeze,
        rasterizer: &dyn TextRasterizer,
    ) -> Result<(), BitmapError> {
        self.draw_text_as(id, rect, text, align, squeeze.into(), rasterizer)
    }

    /// [`Bitmaps::draw_text_squeezed`]: the draw of RGSS1 and of RGSS2.
    fn draw_text_in_cell(
        &mut self,
        id: BitmapId,
        rect: Rect,
        text: &str,
        align: Align,
        squeeze: Squeeze,
        rasterizer: &dyn TextRasterizer,
    ) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        if rect.width <= 0 || rect.height <= 0 {
            return Ok(());
        }
        let shadow = bitmap.font.shadow;
        // What the draw can reach: the rectangle, and one pixel more for a shadow.
        let reach = Rect::new(
            rect.x,
            rect.y,
            rect.width.saturating_add(i32::from(shadow)),
            rect.height.saturating_add(i32::from(shadow)),
        );
        if bitmap.clip(reach).is_none() {
            return Ok(());
        }
        let [red, green, blue, color_alpha] = bitmap.font.color.to_pixel();
        if color_alpha == 0 {
            return Ok(());
        }
        let metrics = rasterizer.metrics(&bitmap.font, text);
        if metrics.advances.is_empty() || metrics.height == 0 {
            return Ok(());
        }

        // The rectangle is not empty here, so its sizes are positive.
        let width = rect.width.unsigned_abs();
        let drawn = metrics.width().saturating_add(metrics.right_overhang);
        let (mask, offset) = if drawn <= width {
            let mask = rasterizer.rasterize(&bitmap.font, text, &metrics.advances, None);
            let offset = align_offset(mask.width, width, align);
            (mask, offset)
        } else {
            let size = bitmap.font.size.max(0).unsigned_abs();
            let forms = metrics.squeezed_forms(size, squeeze);
            let mut squeezed = None;
            for (index, (advances, condense)) in forms.iter().enumerate() {
                let last = index + 1 == forms.len();
                // The overhangs only add to the advances.
                if total(advances) > width && !last {
                    continue;
                }
                let mask = rasterizer.rasterize(&bitmap.font, text, advances, Some(*condense));
                if mask.width <= width {
                    let offset = align_offset(mask.width, width, align);
                    squeezed = Some((mask, offset));
                    break;
                }
                if last {
                    squeezed = Some((mask, 0));
                }
            }
            let Some(squeezed) = squeezed else {
                return Ok(());
            };
            squeezed
        };
        if mask.width == 0 || mask.height == 0 {
            return Ok(());
        }

        let left = i64::from(rect.x) + i64::from(offset);
        let free = i64::from(rect.height) - i64::from(mask.height);
        let top = i64::from(rect.y) + (free / 2).max(0);

        // The part of the text cell that lies inside the rectangle.
        let cell = (
            left,
            top,
            (left + i64::from(mask.width)).min(i64::from(rect.x) + i64::from(rect.width)),
            (top + i64::from(mask.height)).min(i64::from(rect.y) + i64::from(rect.height)),
        );
        let origin = (left, top);
        let shadowed = if shadow {
            blend_layer(bitmap, &mask, origin, cell, 1, [0, 0, 0], color_alpha)
        } else {
            None
        };
        let inked = blend_layer(
            bitmap,
            &mask,
            origin,
            cell,
            0,
            [red, green, blue],
            color_alpha,
        );
        match (shadowed, inked) {
            (Some(first), Some(second)) => bitmap.mark(first.union(second)),
            (Some(area), None) | (None, Some(area)) => bitmap.mark(area),
            (None, None) => {}
        }
        Ok(())
    }

    /// The size `text` has in the bitmap's font, as a rectangle at the origin: the advances
    /// plus the first glyph's left overhang, and the text height. Nothing is drawn. An
    /// empty string has width 0 and height 0. This is the size in every runtime: in RGSS3
    /// neither the outline's 2 nor the shadow is counted.
    pub fn text_size(
        &self,
        id: BitmapId,
        text: &str,
        rasterizer: &dyn TextRasterizer,
    ) -> Result<Rect, BitmapError> {
        let metrics = rasterizer.metrics(&self.get(id)?.font, text);
        if metrics.advances.is_empty() {
            return Ok(Rect::new(0, 0, 0, 0));
        }
        let clamp = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
        Ok(Rect::new(
            0,
            0,
            clamp(metrics.width()),
            clamp(metrics.height),
        ))
    }
}

#[cfg(test)]
mod rgss3_tests;
#[cfg(test)]
mod tests;
