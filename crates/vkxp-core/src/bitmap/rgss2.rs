//! The bitmap operations RGSS2 adds: blur, radial blur, gradient fill and clearing a
//! rectangle, with the arithmetic measured on the original runtime of VX
//! ([`vkxp_pixel::BITMAP_RGSS2_PROVENANCE`]).

use vkxp_pixel::{
    cleared_pixel, gradient_color, radial_blur_source, radial_blur_turns, truncated_mean,
};
use vkxp_proto::BitmapId;

use super::Bitmaps;
use crate::{BitmapError, Color, Rect};

impl Bitmaps {
    /// `Bitmap#blur` of RGSS2: replaces every channel of every pixel, alpha included, by the
    /// sum of that channel over the 3 by 3 pixels around it divided by 9, rounding down
    /// ([`vkxp_pixel::truncated_mean`]). The pixels are read from the bitmap as it was, and
    /// the edge pixel is repeated beyond the edges. A transparent neighbour counts with the
    /// colour it holds. A bitmap narrower or lower than 3 pixels stays unchanged.
    pub fn blur(&mut self, id: BitmapId) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        let (width, height) = (bitmap.width, bitmap.height);
        if width < 3 || height < 3 {
            return Ok(());
        }
        let before = bitmap.rgba.clone();
        for y in 0..height {
            let rows = [y.saturating_sub(1), y, (y + 1).min(height - 1)];
            for x in 0..width {
                let columns = [x.saturating_sub(1), x, (x + 1).min(width - 1)];
                let mut sum = [0u32; 4];
                for row in rows {
                    for column in columns {
                        let offset = (row * width + column) * 4;
                        for (channel, total) in sum.iter_mut().enumerate() {
                            *total += u32::from(before[offset + channel]);
                        }
                    }
                }
                bitmap.put(x, y, truncated_mean(sum, 9));
            }
        }
        bitmap.mark(bitmap.bounds());
        Ok(())
    }

    /// `Bitmap#radial_blur(angle, division)` of RGSS2: every pixel becomes the mean, each
    /// channel on its own and rounded down, of `division` pixels of the bitmap as it was:
    /// the nearest pixels to its place turned about (`width >> 1`, `height >> 1`) by angles
    /// spread evenly over `angle` degrees, mirrored at the edges
    /// ([`vkxp_pixel::radial_blur_turns`], [`vkxp_pixel::radial_blur_source`]). `angle` is
    /// limited to 0 to 360 and `division` to 2 to 100. A bitmap narrower or lower than 3
    /// pixels stays unchanged.
    pub fn radial_blur(
        &mut self,
        id: BitmapId,
        angle: i32,
        division: i32,
    ) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        let (width, height) = (bitmap.width, bitmap.height);
        if width < 3 || height < 3 {
            return Ok(());
        }
        let turns = radial_blur_turns(angle, division);
        let before = bitmap.rgba.clone();
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0u32; 4];
                for &turn in &turns {
                    // Sides are at most `MAX_BITMAP_PIXELS`, which fits `u32`.
                    let (sx, sy) =
                        radial_blur_source(turn, x as u32, y as u32, width as u32, height as u32);
                    let offset = (sy as usize * width + sx as usize) * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u32::from(before[offset + channel]);
                    }
                }
                bitmap.put(x, y, truncated_mean(sum, turns.len() as u32));
            }
        }
        bitmap.mark(bitmap.bounds());
        Ok(())
    }

    /// `Bitmap#gradient_fill_rect` of RGSS2: sets every pixel inside both `rect` and the
    /// bitmap, alpha included and without blending, to the colour of its column, or of its
    /// row when `vertical`: column `i` of a rectangle `n` wide, counted from the unclipped
    /// edge, gets `(c1 + (i * (c2 - c1)) / (n - 1)) & 255` in every channel
    /// ([`vkxp_pixel::gradient_color`]), the colours taken as stored pixels
    /// ([`Color::to_pixel`]). Nothing is drawn when `n`, the width or the height when
    /// `vertical`, is below 2, and nothing for a width or height of 0 or less.
    pub fn gradient_fill_rect(
        &mut self,
        id: BitmapId,
        rect: Rect,
        from: Color,
        to: Color,
        vertical: bool,
    ) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        let n = i64::from(if vertical { rect.height } else { rect.width });
        if n < 2 {
            return Ok(());
        }
        let Some(area) = bitmap.clip(rect) else {
            return Ok(());
        };
        let (from, to) = (from.to_pixel(), to.to_pixel());
        for y in area.top..area.bottom {
            for x in area.left..area.right {
                let i = if vertical {
                    y as i64 - i64::from(rect.y)
                } else {
                    x as i64 - i64::from(rect.x)
                };
                bitmap.put(x, y, gradient_color(from, to, i, n));
            }
        }
        bitmap.mark(area);
        Ok(())
    }

    /// `Bitmap#clear_rect` of RGSS2: sets every pixel inside both `rect` and the bitmap to
    /// transparent black ([`vkxp_pixel::cleared_pixel`]). A rectangle with a width or height
    /// of 0 or less clears nothing.
    pub fn clear_rect(&mut self, id: BitmapId, rect: Rect) -> Result<(), BitmapError> {
        let bitmap = self.get_mut(id)?;
        if let Some(area) = bitmap.clip(rect) {
            bitmap.fill(area, cleared_pixel());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::Area;
    use super::super::testing::*;
    use super::*;

    /// The lines of a sample file that are not comments.
    fn lines(samples: &'static str) -> Vec<&'static str> {
        samples
            .lines()
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .collect()
    }

    /// Rows of pixels written as `rrggbbaa`.
    fn pixels(rows: &[&str]) -> Vec<u8> {
        rows.iter()
            .flat_map(|row| row.split_whitespace())
            .flat_map(|pixel| u32::from_str_radix(pixel, 16).unwrap().to_be_bytes())
            .collect()
    }

    /// The numbers of a case's first line after its kind.
    fn numbers(words: &[&str]) -> Vec<i32> {
        words.iter().map(|word| word.parse().unwrap()).collect()
    }

    /// A colour written as `r,g,b,a`, as it was given to `Color.new`.
    fn color_of(text: &str) -> Color {
        let parts: Vec<f64> = text.split(',').map(|part| part.parse().unwrap()).collect();
        Color::new(parts[0], parts[1], parts[2], parts[3])
    }

    fn bitmap_of(bitmaps: &mut Bitmaps, width: u32, height: u32, rgba: Vec<u8>) -> BitmapId {
        bitmaps
            .create_from_rgba(width, height, rgba, &defaults())
            .unwrap()
    }

    /// Scenario "Measured samples" of "Blur of RGSS2".
    ///
    /// Provenance: oracle measurement 2026-10-12, RGSS202E, bitmap ops probe.
    #[test]
    fn blur_gives_the_measured_pixels() {
        let lines = lines(include_str!("../../tests/data/rgss2-blur-samples.txt"));
        let mut bitmaps = Bitmaps::new();
        let (mut at, mut cases, mut unchanged) = (0, 0, 0);
        while at < lines.len() {
            let words: Vec<&str> = lines[at].split_whitespace().collect();
            assert_eq!(words[0], "blur", "{}", lines[at]);
            let [width, height] = numbers(&words[1..])[..] else {
                panic!("{}", lines[at]);
            };
            let rows = height as usize;
            let before = pixels(&lines[at + 1..at + 1 + rows]);
            let once = pixels(&lines[at + 1 + rows..at + 1 + 2 * rows]);
            let twice = pixels(&lines[at + 1 + 2 * rows..at + 1 + 3 * rows]);
            let id = bitmap_of(&mut bitmaps, width as u32, height as u32, before.clone());
            bitmaps.blur(id).unwrap();
            assert_eq!(bitmaps.rgba(id).unwrap(), once, "{}", lines[at]);
            bitmaps.blur(id).unwrap();
            assert_eq!(bitmaps.rgba(id).unwrap(), twice, "{}", lines[at]);
            unchanged += usize::from(before == twice);
            cases += 1;
            at += 1 + 3 * rows;
        }
        // One of the cases is lower than 3 pixels.
        assert_eq!((cases, unchanged), (5, 1));
    }

    /// Scenario "Transparent neighbours count with their colour".
    #[test]
    fn blur_counts_transparent_neighbours_with_their_colour() {
        let mut bitmaps = Bitmaps::new();
        let mut rgba = [90, 0, 0, 0].repeat(9);
        rgba[16..20].copy_from_slice(&[0, 0, 0, 255]);
        let id = bitmap_of(&mut bitmaps, 3, 3, rgba);
        bitmaps.blur(id).unwrap();
        assert_eq!(at(bitmaps.rgba(id).unwrap(), 3, 1, 1), [80, 0, 0, 28]);
        // A corner counts itself four times, its two neighbours on the edge twice each
        // and the centre once.
        assert_eq!(at(bitmaps.rgba(id).unwrap(), 3, 0, 0), [80, 0, 0, 28]);
    }

    /// Scenario "Small bitmap".
    #[test]
    fn blur_leaves_a_bitmap_narrower_or_lower_than_3_unchanged() {
        let mut bitmaps = Bitmaps::new();
        for (width, height) in [(2, 10), (10, 2), (1, 1)] {
            let rgba = pattern(width, height, 9);
            let id = bitmap_of(&mut bitmaps, width, height, rgba.clone());
            bitmaps.blur(id).unwrap();
            bitmaps.radial_blur(id, 90, 5).unwrap();
            assert_eq!(bitmaps.rgba(id).unwrap(), rgba, "{width} by {height}");
        }
    }

    #[test]
    fn blur_reads_the_bitmap_as_it_was() {
        // One opaque white pixel in the middle of 5 by 5 spreads to the 3 by 3 around it
        // and no further: a blur in place would carry it on.
        let mut bitmaps = Bitmaps::new();
        let mut rgba = vec![0; 100];
        rgba[48..52].copy_from_slice(&[255; 4]);
        let id = bitmap_of(&mut bitmaps, 5, 5, rgba);
        bitmaps.blur(id).unwrap();
        for y in 0..5 {
            for x in 0..5 {
                let inside = (1..4).contains(&x) && (1..4).contains(&y);
                let expected = if inside { [28; 4] } else { [0; 4] };
                assert_eq!(at(bitmaps.rgba(id).unwrap(), 5, x, y), expected, "{x} {y}");
            }
        }
    }

    /// Scenario "Measured samples" of "Radial blur of RGSS2".
    ///
    /// Provenance: oracle measurement 2026-10-12, RGSS202E, bitmap ops probe.
    #[test]
    fn radial_blur_gives_the_measured_pixels() {
        let lines = lines(include_str!(
            "../../tests/data/rgss2-radial-blur-samples.txt"
        ));
        let mut bitmaps = Bitmaps::new();
        let (mut at, mut cases, mut small) = (0, 0, 0);
        while at < lines.len() {
            let words: Vec<&str> = lines[at].split_whitespace().collect();
            assert_eq!(words[0], "radial", "{}", lines[at]);
            let [width, height, angle, division] = numbers(&words[1..])[..] else {
                panic!("{}", lines[at]);
            };
            let rows = height as usize;
            let before = pixels(&lines[at + 1..at + 1 + rows]);
            let after = pixels(&lines[at + 1 + rows..at + 1 + 2 * rows]);
            small += usize::from(width < 3 || height < 3);
            let id = bitmap_of(&mut bitmaps, width as u32, height as u32, before);
            bitmaps.radial_blur(id, angle, division).unwrap();
            assert_eq!(bitmaps.rgba(id).unwrap(), after, "{}", lines[at]);
            cases += 1;
            at += 1 + 2 * rows;
        }
        assert_eq!((cases, small), (18, 2));
    }

    /// Scenario "Limits".
    #[test]
    fn radial_blur_limits_its_arguments() {
        let mut bitmaps = Bitmaps::new();
        let rgba = pattern(9, 7, 3);
        let blurred = |bitmaps: &mut Bitmaps, angle, division| {
            let id = bitmap_of(bitmaps, 9, 7, rgba.clone());
            bitmaps.radial_blur(id, angle, division).unwrap();
            bitmaps.rgba(id).unwrap().to_vec()
        };
        let limit = blurred(&mut bitmaps, 360, 2);
        assert_ne!(limit, rgba);
        assert_eq!(blurred(&mut bitmaps, 1000, 1), limit);
        assert_eq!(
            blurred(&mut bitmaps, 30, 101),
            blurred(&mut bitmaps, 30, 100)
        );
        // An angle of 0 or less takes every sample from the pixel itself.
        assert_eq!(blurred(&mut bitmaps, -10, 3), rgba);
    }

    /// Scenario "Measured samples" of "Gradient of RGSS2".
    ///
    /// Provenance: oracle measurement 2026-10-12, RGSS202E, bitmap ops probe.
    #[test]
    fn gradient_gives_the_measured_pixels() {
        let lines = lines(include_str!("../../tests/data/rgss2-gradient-samples.txt"));
        let mut bitmaps = Bitmaps::new();
        let (mut at, mut cases, mut vertical_cases, mut unchanged) = (0, 0, 0, 0);
        while at < lines.len() {
            let words: Vec<&str> = lines[at].split_whitespace().collect();
            assert_eq!(words[0], "grad", "{}", lines[at]);
            let [width, height, x, y, w, h] = numbers(&words[1..7])[..] else {
                panic!("{}", lines[at]);
            };
            let (from, to) = (color_of(words[7]), color_of(words[8]));
            // The original takes the gradient as vertical whenever a seventh argument is
            // given, whatever it is; with a `Rect` the last argument works as documented.
            let vertical = match (words[9], words[10]) {
                (":none", _) => false,
                (_, "0") => true,
                (last, _) => !matches!(last, "false" | "nil"),
            };
            let rows = height as usize;
            let before = pixels(&lines[at + 1..at + 1 + rows]);
            let after = pixels(&lines[at + 1 + rows..at + 1 + 2 * rows]);
            unchanged += usize::from(before == after);
            vertical_cases += usize::from(vertical);
            let id = bitmap_of(&mut bitmaps, width as u32, height as u32, before);
            bitmaps
                .gradient_fill_rect(id, Rect::new(x, y, w, h), from, to, vertical)
                .unwrap();
            assert_eq!(bitmaps.rgba(id).unwrap(), after, "{}", lines[at]);
            cases += 1;
            at += 1 + 2 * rows;
        }
        assert_eq!(cases, 26);
        assert!(vertical_cases >= 10 && cases - vertical_cases >= 10);
        // Rectangles with fewer than two steps along the gradient draw nothing.
        assert!(unchanged >= 6, "{unchanged}");
    }

    /// Scenario "End points".
    #[test]
    fn gradient_end_points() {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(6, 2, &defaults()).unwrap();
        let (from, to) = (color([0, 0, 0, 255]), color([255, 10, 0, 0]));
        bitmaps
            .gradient_fill_rect(id, Rect::new(1, 0, 4, 2), from, to, false)
            .unwrap();
        let expected = [
            [0, 0, 0, 0],
            [0, 0, 0, 255],
            [85, 3, 0, 170],
            [170, 6, 0, 85],
            [255, 10, 0, 0],
            [0, 0, 0, 0],
        ];
        for y in 0..2 {
            for (x, pixel) in expected.iter().enumerate() {
                assert_eq!(at(bitmaps.rgba(id).unwrap(), 6, x as u32, y), *pixel);
            }
        }
        // Vertical: the same colours down the rows.
        let down = bitmaps.create(1, 4, &defaults()).unwrap();
        bitmaps
            .gradient_fill_rect(down, Rect::new(0, 0, 1, 4), from, to, true)
            .unwrap();
        assert_eq!(bitmaps.rgba(down).unwrap(), expected[1..5].concat());
    }

    /// Scenario "One column".
    #[test]
    fn gradient_with_one_step_draws_nothing() {
        let mut bitmaps = Bitmaps::new();
        let rgba = pattern(4, 4, 1);
        let id = bitmap_of(&mut bitmaps, 4, 4, rgba.clone());
        let (from, to) = (color([1, 2, 3, 4]), color([5, 6, 7, 8]));
        for (rect, vertical) in [
            (Rect::new(1, 0, 1, 4), false),
            (Rect::new(0, 1, 4, 1), true),
            (Rect::new(0, 0, 0, 4), false),
            (Rect::new(0, 0, -3, 4), false),
            (Rect::new(0, 0, 4, 0), false),
            (Rect::new(0, 0, 4, -1), true),
        ] {
            bitmaps
                .gradient_fill_rect(id, rect, from, to, vertical)
                .unwrap();
            assert_eq!(bitmaps.rgba(id).unwrap(), rgba, "{rect:?}");
        }
    }

    #[test]
    fn gradient_counts_its_steps_from_the_unclipped_edge() {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(2, 1, &defaults()).unwrap();
        let (from, to) = (color([0, 0, 0, 0]), color([90, 90, 90, 90]));
        // Columns 2 and 3 of ten.
        bitmaps
            .gradient_fill_rect(id, Rect::new(-2, -5, 10, 9), from, to, false)
            .unwrap();
        assert_eq!(bitmaps.rgba(id).unwrap(), [[20; 4], [30; 4]].concat());
    }

    /// Scenario "Clipped".
    #[test]
    fn clear_rect_clears_the_part_inside_the_bitmap() {
        let mut bitmaps = Bitmaps::new();
        let id = bitmap_of(&mut bitmaps, 4, 4, [9, 8, 7, 255].repeat(16));
        bitmaps.clear_rect(id, Rect::new(-2, -2, 4, 4)).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                let expected = if x < 2 && y < 2 {
                    [0; 4]
                } else {
                    [9, 8, 7, 255]
                };
                assert_eq!(at(bitmaps.rgba(id).unwrap(), 4, x, y), expected, "{x} {y}");
            }
        }
    }

    #[test]
    fn clear_rect_of_no_size_clears_nothing() {
        let mut bitmaps = Bitmaps::new();
        let rgba = pattern(4, 4, 5);
        let id = bitmap_of(&mut bitmaps, 4, 4, rgba.clone());
        for rect in [
            Rect::new(0, 0, 0, 4),
            Rect::new(0, 0, 4, 0),
            Rect::new(1, 1, -2, 2),
            Rect::new(1, 1, 2, -2),
            Rect::new(4, 0, 2, 2),
        ] {
            bitmaps.clear_rect(id, rect).unwrap();
            assert_eq!(bitmaps.rgba(id).unwrap(), rgba, "{rect:?}");
        }
    }

    #[test]
    fn the_operations_fail_on_a_disposed_bitmap() {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(4, 4, &defaults()).unwrap();
        bitmaps.dispose(id);
        let disposed = Err(BitmapError::Disposed);
        let black = color([0, 0, 0, 255]);
        assert_eq!(bitmaps.blur(id), disposed);
        assert_eq!(bitmaps.radial_blur(id, 10, 2), disposed);
        assert_eq!(bitmaps.clear_rect(id, Rect::new(0, 0, 1, 1)), disposed);
        assert_eq!(
            bitmaps.gradient_fill_rect(id, Rect::new(0, 0, 2, 2), black, black, false),
            disposed
        );
    }

    #[test]
    fn the_operations_mark_what_they_change() {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(8, 8, &defaults()).unwrap();
        let revision = bitmaps.revision(id).unwrap();
        bitmaps.clear_rect(id, Rect::new(1, 2, 3, 4)).unwrap();
        let cleared = Area {
            left: 1,
            top: 2,
            right: 4,
            bottom: 6,
        };
        assert_eq!(bitmaps.get(id).unwrap().changed, Some(cleared));
        let black = color([0, 0, 0, 255]);
        bitmaps
            .gradient_fill_rect(id, Rect::new(6, 6, 9, 9), black, black, true)
            .unwrap();
        let both = Area {
            left: 1,
            top: 2,
            right: 8,
            bottom: 8,
        };
        assert_eq!(bitmaps.get(id).unwrap().changed, Some(both));
        bitmaps.blur(id).unwrap();
        bitmaps.radial_blur(id, 20, 3).unwrap();
        let whole = bitmaps.get(id).unwrap().bounds();
        assert_eq!(bitmaps.get(id).unwrap().changed, Some(whole));
        assert_eq!(bitmaps.revision(id).unwrap(), revision + 4);
    }
}
