//! Text: the rasteriser interface, and drawing and measuring text in a bitmap.
//!
//! This crate lays text out and blends it; turning a string into glyph coverage is the job of
//! a [`TextRasterizer`] that the embedder supplies.

use vkxp_pixel::{blend_normal, effective_alpha, stretch_index};
use vkxp_proto::BitmapId;

use crate::bitmap::Area;
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

/// Turns text into glyph coverage. Implemented for real fonts outside this crate.
pub trait TextRasterizer {
    /// The width and height of `text` in `font`, in pixels. An empty string has width 0 and
    /// the height of one line.
    fn measure(&self, font: &Font, text: &str) -> (u32, u32);

    /// The coverage of `text` in `font`: a mask of exactly the size [`TextRasterizer::measure`]
    /// reports.
    fn rasterize(&self, font: &Font, text: &str) -> Mask;
}

/// Horizontal placement of text inside its rectangle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// The width text of natural width `natural` is drawn with in a rectangle of width `width`,
/// and the distance of its left edge from the rectangle's.
///
/// Text that fits keeps its width and is placed by `align`. Wider text is squeezed to the
/// rectangle's width, but to no less than 60 percent of its natural width (rounded up), and
/// starts at the rectangle's left edge.
///
/// PLACEHOLDER: the 60 percent limit and the placement of text that still does not fit are not
/// yet checked against the original runtime.
fn horizontal_layout(natural: u32, width: u32, align: Align) -> (u32, u32) {
    if natural <= width {
        let free = width - natural;
        let offset = match align {
            Align::Left => 0,
            Align::Center => free / 2,
            Align::Right => free,
        };
        return (natural, offset);
    }
    // ceil(0.6 * natural), which is below `natural` and so fits `u32`.
    let limit = (u64::from(natural) * 3).div_ceil(5) as u32;
    (width.max(limit), 0)
}

impl Bitmaps {
    /// Draws `text` inside `rect` with the bitmap's font.
    ///
    /// The text is centred vertically (the free height halved, rounded down) and placed
    /// horizontally by `align`. Text wider than the rectangle is squeezed to fit, to no less
    /// than 60 percent of its natural width, by taking mask columns with
    /// [`vkxp_pixel::stretch_index`]; what still does not fit is clipped, as is everything
    /// outside the rectangle or the bitmap.
    ///
    /// Each mask value `c` draws the font's colour with alpha
    /// `effective_alpha(c, colour alpha)`, blended as a block transfer blends
    /// ([`vkxp_pixel::blend_normal`]). A pixel whose alpha comes to 0 is left alone.
    ///
    /// PLACEHOLDER: how squeezed text is resampled is not yet checked against the original
    /// runtime.
    pub fn draw_text(
        &mut self,
        id: BitmapId,
        rect: Rect,
        text: &str,
        align: Align,
        rasterizer: &dyn TextRasterizer,
    ) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        let Some(clip) = bitmap.clip(rect) else {
            return Ok(());
        };
        let [red, green, blue, color_alpha] = bitmap.font.color.to_pixel();
        // With alpha 0 every glyph pixel is drawn with alpha 0, which changes nothing.
        if color_alpha == 0 {
            return Ok(());
        }
        let mask = rasterizer.rasterize(&bitmap.font, text);
        if mask.width == 0 || mask.height == 0 {
            return Ok(());
        }

        // The rectangle is not empty here, so its sizes are positive.
        let (drawn_width, offset) = horizontal_layout(mask.width, rect.width.unsigned_abs(), align);
        let left = i64::from(rect.x) + i64::from(offset);
        let top =
            i64::from(rect.y) + (i64::from(rect.height) - i64::from(mask.height)).div_euclid(2);

        // The part of the text that lies inside the rectangle and the bitmap.
        let span = |start: i64, len: u32, low: usize, high: usize| {
            let first = start.max(low as i64);
            let end = (start + i64::from(len)).min(high as i64);
            (first < end).then_some((first as usize, end as usize))
        };
        let Some((first_column, end_column)) = span(left, drawn_width, clip.left, clip.right)
        else {
            return Ok(());
        };
        let Some((first_row, end_row)) = span(top, mask.height, clip.top, clip.bottom) else {
            return Ok(());
        };

        for y in first_row..end_row {
            let mask_y = (y as i64 - top) as u32;
            for x in first_column..end_column {
                let mask_x = stretch_index((x as i64 - left) as u32, mask.width, drawn_width);
                let alpha = effective_alpha(mask.get(mask_x, mask_y), color_alpha);
                // No coverage draws nothing, also where the bitmap is transparent.
                if alpha == 0 {
                    continue;
                }
                let source = [red, green, blue, alpha];
                let blended = blend_normal(bitmap.pixel(x, y), source, 255);
                bitmap.put(x, y, blended);
            }
        }
        bitmap.mark(Area {
            left: first_column,
            top: first_row,
            right: end_column,
            bottom: end_row,
        });
        Ok(())
    }

    /// The size `text` has in the bitmap's font, as a rectangle at the origin. Nothing is
    /// drawn. An empty string has width 0 and the height of one line.
    pub fn text_size(
        &self,
        id: BitmapId,
        text: &str,
        rasterizer: &dyn TextRasterizer,
    ) -> Result<Rect, BitmapError> {
        let (width, height) = rasterizer.measure(&self.get(id)?.font, text);
        let clamp = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
        Ok(Rect::new(0, 0, clamp(width), clamp(height)))
    }
}

#[cfg(test)]
mod tests {
    use vkxp_pixel::Rgba;

    use super::*;
    use crate::bitmap::testing::*;
    use crate::test_support::CellRasterizer;
    use crate::{Color, FontDefaults};

    /// Defaults whose font the stand-in draws with cells of 5 by 10 pixels.
    fn small() -> FontDefaults {
        let mut defaults = defaults();
        defaults.size = 10;
        defaults
    }

    /// A string of natural width 100 at the font of [`small`].
    const WIDE: &str = "ABCDEFGHIJKLMNOPQRST";

    /// What `draw_text` must leave in a bitmap that held `before`: the model of the layout
    /// rules, written with signed arithmetic and no clipping shortcuts.
    #[allow(clippy::too_many_arguments)]
    fn model(
        before: &[u8],
        (width, height): (u32, u32),
        rect: Rect,
        mask: &Mask,
        (drawn_width, offset): (u32, u32),
        color: Rgba,
    ) -> Vec<u8> {
        let mut after = before.to_vec();
        let left = rect.x + offset as i32;
        let top = rect.y + (rect.height - mask.height() as i32).div_euclid(2);
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let in_rect = (rect.x..rect.x + rect.width).contains(&x)
                    && (rect.y..rect.y + rect.height).contains(&y);
                let in_text = (left..left + drawn_width as i32).contains(&x)
                    && (top..top + mask.height() as i32).contains(&y);
                if !(in_rect && in_text) {
                    continue;
                }
                let mask_x = stretch_index((x - left) as u32, mask.width(), drawn_width);
                let coverage = mask.get(mask_x, (y - top) as u32);
                let source = [
                    color[0],
                    color[1],
                    color[2],
                    effective_alpha(coverage, color[3]),
                ];
                if source[3] == 0 {
                    continue;
                }
                let offset = ((y as u32 * width + x as u32) * 4) as usize;
                let target: Rgba = before[offset..offset + 4].try_into().unwrap();
                after[offset..offset + 4].copy_from_slice(&blend_normal(target, source, 255));
            }
        }
        after
    }

    /// The columns of row `y` that are not transparent.
    fn drawn_columns(rgba: &[u8], width: u32, y: u32) -> Vec<u32> {
        (0..width)
            .filter(|&x| at(rgba, width, x, y)[3] != 0)
            .collect()
    }

    /// The rows of column `x` that are not transparent.
    fn drawn_rows(rgba: &[u8], width: u32, height: u32, x: u32) -> Vec<u32> {
        (0..height)
            .filter(|&y| at(rgba, width, x, y)[3] != 0)
            .collect()
    }

    #[test]
    fn horizontal_layout_rules() {
        assert_eq!(horizontal_layout(10, 30, Align::Left), (10, 0));
        assert_eq!(horizontal_layout(10, 30, Align::Center), (10, 10));
        assert_eq!(horizontal_layout(10, 30, Align::Right), (10, 20));
        assert_eq!(horizontal_layout(10, 31, Align::Center), (10, 10));
        assert_eq!(horizontal_layout(10, 10, Align::Right), (10, 0));
        for align in [Align::Left, Align::Center, Align::Right] {
            assert_eq!(horizontal_layout(100, 80, align), (80, 0));
            assert_eq!(horizontal_layout(100, 60, align), (60, 0));
            assert_eq!(horizontal_layout(100, 40, align), (60, 0));
            // 60 percent of 101 is 60.6, which rounds up.
            assert_eq!(horizontal_layout(101, 1, align), (61, 0));
        }
        assert_eq!(horizontal_layout(u32::MAX, 1, Align::Left).0, 2_576_980_377);
    }

    #[test]
    fn alignment_places_the_left_edge() {
        let rasterizer = CellRasterizer;
        for (align, left) in [(Align::Left, 3), (Align::Center, 13), (Align::Right, 23)] {
            let mut bitmaps = Bitmaps::new();
            let id = bitmaps.create(40, 14, &small()).unwrap();
            // "AB" is 10 wide; the rectangle leaves 20 pixels free.
            bitmaps
                .draw_text(id, Rect::new(3, 2, 30, 10), "AB", align, &rasterizer)
                .unwrap();
            let rgba = bitmaps.rgba(id).unwrap();
            for y in 2..12 {
                let expected: Vec<u32> = (left..left + 10).collect();
                assert_eq!(drawn_columns(rgba, 40, y), expected, "{align:?}, row {y}");
            }
            assert!(drawn_columns(rgba, 40, 1).is_empty());
            assert!(drawn_columns(rgba, 40, 12).is_empty());
        }
    }

    #[test]
    fn vertical_centring() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(12, 40, &small()).unwrap();
        // Text of height 10 in a rectangle of height 30 that starts at row 4.
        bitmaps
            .draw_text(id, Rect::new(0, 4, 12, 30), "A", Align::Left, &rasterizer)
            .unwrap();
        let rows: Vec<u32> = (14..24).collect();
        assert_eq!(drawn_rows(bitmaps.rgba(id).unwrap(), 12, 40, 0), rows);

        // An odd free height is halved rounding down.
        let odd = bitmaps.create(12, 40, &small()).unwrap();
        bitmaps
            .draw_text(odd, Rect::new(0, 0, 12, 13), "A", Align::Left, &rasterizer)
            .unwrap();
        let rows: Vec<u32> = (1..11).collect();
        assert_eq!(drawn_rows(bitmaps.rgba(odd).unwrap(), 12, 40, 0), rows);
    }

    #[test]
    fn text_taller_than_its_rectangle_is_clipped_around_its_middle() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(12, 20, &small()).unwrap();
        let rect = Rect::new(1, 5, 5, 3);
        bitmaps
            .draw_text(id, rect, "A", Align::Left, &rasterizer)
            .unwrap();
        let font = bitmaps.font(id).unwrap().clone();
        let mask = rasterizer.rasterize(&font, "A");
        let rgba = bitmaps.rgba(id).unwrap();
        assert_eq!(drawn_rows(rgba, 12, 20, 1), [5, 6, 7]);
        // (3 - 10) / 2 rounded down is -4: rectangle row 0 shows mask row 4.
        for row in 0..3 {
            assert_eq!(at(rgba, 12, 1, 5 + row)[3], mask.get(0, 4 + row));
        }
    }

    #[test]
    fn squeezing_to_the_rectangle() {
        let rasterizer = CellRasterizer;
        for align in [Align::Left, Align::Center, Align::Right] {
            let mut bitmaps = Bitmaps::new();
            let id = bitmaps.create(120, 12, &small()).unwrap();
            let font = bitmaps.font(id).unwrap().clone();
            let mask = rasterizer.rasterize(&font, WIDE);
            assert_eq!(mask.width(), 100);
            bitmaps
                .draw_text(id, Rect::new(10, 1, 80, 10), WIDE, align, &rasterizer)
                .unwrap();
            let rgba = bitmaps.rgba(id).unwrap();
            let expected: Vec<u32> = (10..90).collect();
            assert_eq!(drawn_columns(rgba, 120, 5), expected, "{align:?}");
            for i in 0..80 {
                // The font colour is opaque, so on a transparent bitmap alpha is coverage.
                let coverage = mask.get(stretch_index(i, 100, 80), 4);
                assert_eq!(at(rgba, 120, 10 + i, 5)[3], coverage, "column {i}");
            }
        }
    }

    #[test]
    fn squeeze_limit_clips_what_does_not_fit() {
        let rasterizer = CellRasterizer;
        for align in [Align::Left, Align::Center, Align::Right] {
            let mut bitmaps = Bitmaps::new();
            let id = bitmaps.create(120, 12, &small()).unwrap();
            let font = bitmaps.font(id).unwrap().clone();
            let mask = rasterizer.rasterize(&font, WIDE);
            bitmaps
                .draw_text(id, Rect::new(10, 1, 40, 10), WIDE, align, &rasterizer)
                .unwrap();
            let rgba = bitmaps.rgba(id).unwrap();
            // Drawn 60 wide from the rectangle's left edge; only its first 40 columns appear.
            let expected: Vec<u32> = (10..50).collect();
            assert_eq!(drawn_columns(rgba, 120, 5), expected, "{align:?}");
            for i in 0..40 {
                let coverage = mask.get(stretch_index(i, 100, 60), 4);
                assert_eq!(at(rgba, 120, 10 + i, 5)[3], coverage, "column {i}");
            }
        }
    }

    #[test]
    fn zero_coverage_leaves_the_destination() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let before = pattern(15, 10, 8);
        let id = bitmaps
            .create_from_rgba(15, 10, before.clone(), &small())
            .unwrap();
        // The stand-in gives a space no coverage at all.
        bitmaps
            .draw_text(id, Rect::new(0, 0, 15, 10), " A ", Align::Left, &rasterizer)
            .unwrap();
        let rgba = bitmaps.rgba(id).unwrap();
        for y in 0..10 {
            for x in 0..15 {
                let unchanged = at(rgba, 15, x, y) == at(&before, 15, x, y);
                assert_eq!(unchanged, !(5..10).contains(&x), "({x}, {y})");
            }
        }
    }

    #[test]
    fn zero_colour_alpha_leaves_the_destination() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let before = pattern(15, 10, 8);
        let id = bitmaps
            .create_from_rgba(15, 10, before.clone(), &small())
            .unwrap();
        bitmaps.font_mut(id).unwrap().color = Color::new(255.0, 255.0, 255.0, 0.0);
        bitmaps
            .draw_text(id, Rect::new(0, 0, 15, 10), "ABC", Align::Left, &rasterizer)
            .unwrap();
        assert_eq!(bitmaps.rgba(id).unwrap(), before);
    }

    #[test]
    fn coverage_scales_the_colour_alpha_and_blends_source_over() {
        let rasterizer = CellRasterizer;
        let color: Rgba = [200, 100, 50, 128];
        let cases = [
            (Rect::new(2, 1, 30, 14), "ABC", Align::Center, (15, 7)),
            (Rect::new(-3, -2, 12, 9), "ABCD", Align::Right, (12, 0)),
            (Rect::new(20, 4, 100, 12), "AB", Align::Left, (10, 0)),
        ];
        for (rect, text, align, layout) in cases {
            let mut bitmaps = Bitmaps::new();
            let before = pattern(36, 18, 8);
            let id = bitmaps
                .create_from_rgba(36, 18, before.clone(), &small())
                .unwrap();
            bitmaps.font_mut(id).unwrap().color = crate::bitmap::testing::color(color);
            let font = bitmaps.font(id).unwrap().clone();
            let mask = rasterizer.rasterize(&font, text);
            bitmaps
                .draw_text(id, rect, text, align, &rasterizer)
                .unwrap();
            let expected = model(&before, (36, 18), rect, &mask, layout, color);
            assert!(bitmaps.rgba(id).unwrap() == expected, "{rect:?}");
            assert!(expected != before, "{rect:?} draws something");
        }
    }

    #[test]
    fn text_uses_the_bitmaps_own_font() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(40, 40, &small()).unwrap();
        bitmaps.font_mut(id).unwrap().size = 20;
        bitmaps.font_mut(id).unwrap().color = Color::rgb(1.0, 2.0, 3.0);
        bitmaps
            .draw_text(id, Rect::new(0, 0, 40, 20), "A", Align::Left, &rasterizer)
            .unwrap();
        let rgba = bitmaps.rgba(id).unwrap();
        // A size of 20 gives the stand-in cells of 10 by 20.
        assert_eq!(drawn_columns(rgba, 40, 0), (0..10).collect::<Vec<u32>>());
        assert_eq!(drawn_rows(rgba, 40, 40, 0), (0..20).collect::<Vec<u32>>());
        assert_eq!(at(rgba, 40, 0, 0)[..3], [1, 2, 3]);
    }

    #[test]
    fn empty_rectangles_and_strings_draw_nothing() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(20, 20, &small()).unwrap();
        for rect in [
            Rect::new(0, 0, 0, 10),
            Rect::new(0, 0, 10, 0),
            Rect::new(5, 5, -5, -5),
            Rect::new(20, 0, 10, 10),
            Rect::new(0, -10, 10, 10),
        ] {
            bitmaps
                .draw_text(id, rect, "AB", Align::Left, &rasterizer)
                .unwrap();
        }
        bitmaps
            .draw_text(id, Rect::new(0, 0, 20, 20), "", Align::Center, &rasterizer)
            .unwrap();
        assert_eq!(bitmaps.rgba(id).unwrap(), [0; 1600]);
    }

    #[test]
    fn text_on_a_disposed_bitmap_fails() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(4, 4, &small()).unwrap();
        bitmaps.dispose(id);
        assert_eq!(
            bitmaps.draw_text(id, Rect::new(0, 0, 4, 4), "A", Align::Left, &rasterizer),
            Err(BitmapError::Disposed)
        );
        assert_eq!(
            bitmaps.text_size(id, "A", &rasterizer),
            Err(BitmapError::Disposed)
        );
    }

    #[test]
    fn measurement_matches_drawing() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(40, 20, &small()).unwrap();
        let size = bitmaps.text_size(id, "Hello", &rasterizer).unwrap();
        assert_eq!(size, Rect::new(0, 0, 25, 10));
        // Measuring draws nothing.
        assert_eq!(bitmaps.rgba(id).unwrap(), [0; 3200]);

        let rect = Rect::new(7, 4, size.width, size.height);
        bitmaps
            .draw_text(id, rect, "Hello", Align::Left, &rasterizer)
            .unwrap();
        let font = bitmaps.font(id).unwrap().clone();
        let mask = rasterizer.rasterize(&font, "Hello");
        let rgba = bitmaps.rgba(id).unwrap();
        for y in 0..20 {
            for x in 0..40 {
                // Unsqueezed, the mask appears pixel for pixel inside the rectangle, and no
                // glyph pixel falls outside it.
                let inside = (7..32).contains(&x) && (4..14).contains(&y);
                let expected = if inside { mask.get(x - 7, y - 4) } else { 0 };
                assert_eq!(at(rgba, 40, x, y)[3], expected, "({x}, {y})");
            }
        }
    }

    #[test]
    fn empty_string_measures_one_line() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(4, 4, &small()).unwrap();
        assert_eq!(
            bitmaps.text_size(id, "", &rasterizer),
            Ok(Rect::new(0, 0, 0, 10))
        );
        bitmaps.font_mut(id).unwrap().size = 24;
        let font = bitmaps.font(id).unwrap().clone();
        assert_eq!(
            bitmaps.text_size(id, "", &rasterizer),
            Ok(Rect::new(
                0,
                0,
                0,
                CellRasterizer::line_height(&font) as i32
            ))
        );
        assert_eq!(CellRasterizer::line_height(&font), 24);
    }
}
