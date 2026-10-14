//! The metric rule of RGSS3: glyphs hinted at whole pixels per em as Wine's FreeType hints
//! them, and the arithmetic Wine's GDI does on them.
//!
//! Provenance: the `text_size` table of VL Gothic and VL PGothic measured on RGSS301 3.0.1.1
//! and RGSS300 3.0.0.1 under Wine 11.0 on 2026-10-13 (28392 values, outside the
//! repository), and the drawn widths of the sweeps of the same measurement.

use std::collections::HashMap;

use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{
    AdjustedMetrics, DrawSettings, Engine, GlyphStyles, HintingInstance, HintingOptions,
    OutlineGlyphCollection, OutlinePen, SmoothMode, Target,
};
use skrifa::{FontRef, MetadataProvider, Tag};
use swash::GlyphId;
use swash::zeno::Point;

/// The engine a face is hinted with under the metric rule of RGSS3: the one FreeType chooses
/// when a glyph is loaded without load flags, as Wine's GDI loads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HintingEngine {
    /// FreeType's automatic hinter.
    Automatic,
    /// The font's own hinting: the TrueType interpreter, or the PostScript hinter of a CFF
    /// font.
    Font,
}

/// The longest `prep` table with which FreeType still auto-hints a TrueType font that has no
/// `fpgm` table.
///
/// Provenance: probed on FreeType 2.14.3, the library Wine 11.0 rasterised with, on copies
/// of VL Gothic: a `prep` of 0, 6 and 7 bytes was auto-hinted, one of 8 bytes was not.
const LONGEST_AUTO_HINTED_PREP: usize = 7;

/// The slant of a synthesised italic in 16.16 fixed point: a quarter.
const SLANT: i64 = 0x4000;

/// The hinting engine FreeType chooses for `font`: the automatic hinter for TrueType outlines
/// without an `fpgm` table and with a `prep` table of at most 7 bytes, the font's own
/// otherwise. Whatever `maxSizeOfInstructions` says makes no difference.
pub(crate) fn choose_engine(font: &FontRef<'_>) -> HintingEngine {
    let length = |tag: &[u8; 4]| font.table_data(Tag::new(tag)).map_or(0, |data| data.len());
    let true_type = font.table_data(Tag::new(b"glyf")).is_some();
    if true_type && length(b"fpgm") == 0 && length(b"prep") <= LONGEST_AUTO_HINTED_PREP {
        HintingEngine::Automatic
    } else {
        HintingEngine::Font
    }
}

/// The hinting of one face: its engine, and the instances and advances made on first use.
pub(crate) struct Hints {
    engine: HintingEngine,
    coords: Vec<NormalizedCoord>,
    /// The glyph styles of the automatic hinter, computed once for the face.
    styles: Option<GlyphStyles>,
    /// Instances by pixels per em; `None` where `skrifa` made none (the font's programs
    /// failed).
    instances: HashMap<u32, Option<HintingInstance>>,
    /// Hinted advances in 26.6 by pixels per em and glyph.
    advances: HashMap<(u32, GlyphId), i32>,
}

impl Hints {
    /// The hinting of `font` set to the variation coordinates `coords`, as F2Dot14 bits.
    pub(crate) fn new(font: Option<&FontRef<'_>>, coords: &[i16]) -> Self {
        Self {
            engine: font.map_or(HintingEngine::Font, choose_engine),
            coords: coords
                .iter()
                .map(|&coord| NormalizedCoord::from_bits(coord))
                .collect(),
            styles: None,
            instances: HashMap::new(),
            advances: HashMap::new(),
        }
    }

    pub(crate) fn engine(&self) -> HintingEngine {
        self.engine
    }

    /// The hinted advance of `glyph` at `pixels_per_em` in 26.6, as FreeType gives it. A
    /// glyph `skrifa` gives no hinted draw or advance for has none, 0.
    pub(crate) fn advance(
        &mut self,
        font: &FontRef<'_>,
        pixels_per_em: u32,
        glyph: GlyphId,
    ) -> i32 {
        if let Some(&advance) = self.advances.get(&(pixels_per_em, glyph)) {
            return advance;
        }
        let advance = self
            .draw(font, pixels_per_em, glyph, &mut Points::default())
            .and_then(|metrics| metrics.advance_width)
            .map_or(0, to_26_6);
        self.advances.insert((pixels_per_em, glyph), advance);
        advance
    }

    /// The points of the hinted outline of `glyph` at `pixels_per_em`, y up, in the order a
    /// draw gives them: that of the points of `swash`'s outline of the same glyph. `None`
    /// where `skrifa` gives no hinted draw.
    pub(crate) fn points(
        &mut self,
        font: &FontRef<'_>,
        pixels_per_em: u32,
        glyph: GlyphId,
    ) -> Option<Vec<Point>> {
        let mut points = Points::default();
        self.draw(font, pixels_per_em, glyph, &mut points)?;
        Some(points.0)
    }

    fn draw(
        &mut self,
        font: &FontRef<'_>,
        pixels_per_em: u32,
        glyph: GlyphId,
        pen: &mut impl OutlinePen,
    ) -> Option<AdjustedMetrics> {
        let outlines = font.outline_glyphs();
        let outline = outlines.get(skrifa::GlyphId::from(glyph))?;
        let instance = self
            .instances
            .entry(pixels_per_em)
            .or_insert_with(|| {
                let engine = match self.engine {
                    HintingEngine::Automatic => Engine::Auto(Some(
                        self.styles
                            .get_or_insert_with(|| GlyphStyles::new(&outlines))
                            .clone(),
                    )),
                    HintingEngine::Font => Engine::Interpreter,
                };
                instance(&outlines, pixels_per_em, &self.coords, engine)
            })
            .as_ref()?;
        outline
            .draw(DrawSettings::hinted(instance, false), pen)
            .ok()
    }
}

/// A hinting instance for an anti-aliased target, as FreeType's normal target behaves.
fn instance(
    outlines: &OutlineGlyphCollection<'_>,
    pixels_per_em: u32,
    coords: &[NormalizedCoord],
    engine: Engine,
) -> Option<HintingInstance> {
    let options = HintingOptions {
        engine,
        target: Target::Smooth {
            mode: SmoothMode::Normal,
            symmetric_rendering: true,
            preserve_linear_metrics: false,
        },
    };
    let size = Size::new(pixels_per_em as f32);
    HintingInstance::new(outlines, size, LocationRef::new(coords), options).ok()
}

/// Collects the points of an outline in the order they are drawn.
#[derive(Default)]
struct Points(Vec<Point>);

impl OutlinePen for Points {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Point::new(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Point::new(x, y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.extend([Point::new(cx0, cy0), Point::new(x, y)]);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0
            .extend([Point::new(cx0, cy0), Point::new(cx1, cy1), Point::new(x, y)]);
    }

    fn close(&mut self) {}
}

/// Pixels as 26.6 fixed point, to the nearest 1/64.
fn to_26_6(pixels: f32) -> i32 {
    (pixels * 64.0).round() as i32
}

/// A 26.6 value rounded down to whole pixels.
fn floor_pixels(value: i64) -> i64 {
    value.div_euclid(64)
}

/// A 26.6 value rounded up to whole pixels.
fn ceil_pixels(value: i64) -> i64 {
    (value + 63).div_euclid(64)
}

/// FreeType's `FT_MulFix`: `a * b / 65536`, rounded on the magnitude.
fn mul_fix(a: i64, b: i64) -> i64 {
    let magnitude = ((a.unsigned_abs() * b.unsigned_abs() + 0x8000) >> 16) as i64;
    if (a < 0) != (b < 0) {
        -magnitude
    } else {
        magnitude
    }
}

/// FreeType's `FT_MulDiv` for non-negative values: `a * b / c`, rounded.
fn mul_div(a: u64, b: u64, c: u64) -> u64 {
    let c = c.max(1);
    (a * b + c / 2) / c
}

/// What the rule of RGSS3 reads of a face besides its glyphs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FaceMetrics {
    pub(crate) units_per_em: u16,
    /// The OS/2 `xAvgCharWidth`, in font units.
    pub(crate) average_width: u16,
    /// Whether the OS/2 panose says the face is monospaced: GDI reports it as fixed pitch.
    pub(crate) monospaced: bool,
}

impl FaceMetrics {
    /// The scale of font units to pixels at `pixels_per_em`, 16.16, as Wine computes it.
    fn scale(&self, pixels_per_em: u32) -> i64 {
        mul_div(
            u64::from(pixels_per_em),
            65536,
            u64::from(self.units_per_em),
        ) as i64
    }

    /// GDI's `tmAveCharWidth` under Wine at `pixels_per_em`, at least 1: `MulFix(a, s)`.
    pub(crate) fn average_width(&self, pixels_per_em: u32) -> u32 {
        let width = mul_fix(i64::from(self.average_width), self.scale(pixels_per_em));
        width.clamp(1, i64::from(u32::MAX)) as u32
    }

    /// GDI's `gmCellIncX` under Wine of a glyph whose hinted advance is `hinted` (26.6) at
    /// `pixels_per_em`: the hinted advance rounded up; in a fixed-pitch face a glyph as wide
    /// as twice the average width at that scale is twice the scaled average width.
    pub(crate) fn advance(&self, hinted: i32, pixels_per_em: u32) -> u32 {
        let advance = ceil_pixels(i64::from(hinted)).max(0);
        if self.monospaced {
            let scale = self.scale(pixels_per_em);
            let average = i64::from(self.average_width);
            if advance == mul_fix(2 * average, scale) {
                return (2 * mul_fix(average, scale)).clamp(0, i64::from(u32::MAX)) as u32;
            }
        }
        advance.min(i64::from(u32::MAX)) as u32
    }
}

/// The box the overhangs of a glyph are taken from, in pixel columns relative to the pen:
/// the control box from `min` to `max` (pixels, y up), on whole pixels outwards when
/// `snapped`, its corners slanted by `x + y / 4` when `italic`, the left edge rounded down
/// and the right edge rounded up.
pub(crate) fn extent(min: Point, max: Point, snapped: bool, italic: bool) -> (i32, i32) {
    let (left, right) = slanted_edges(min, max, snapped, italic);
    let column = |value: i64| value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    (column(floor_pixels(left)), column(ceil_pixels(right)))
}

/// The left and the right edge, 26.6, of the box of [`extent`] before they are rounded.
fn slanted_edges(min: Point, max: Point, snapped: bool, italic: bool) -> (i64, i64) {
    let fixed = |value: f32| i64::from(to_26_6(value));
    let (mut x_min, mut y_min, mut x_max, mut y_max) =
        (fixed(min.x), fixed(min.y), fixed(max.x), fixed(max.y));
    if snapped {
        x_min = floor_pixels(x_min) * 64;
        y_min = floor_pixels(y_min) * 64;
        x_max = ceil_pixels(x_max) * 64;
        y_max = ceil_pixels(y_max) * 64;
    }
    if !italic {
        return (x_min, x_max);
    }
    let corners = [
        (x_min, y_min),
        (x_min, y_max),
        (x_max, y_min),
        (x_max, y_max),
    ]
    .map(|(x, y)| x + mul_fix(y, SLANT));
    let left = corners.iter().copied().min().unwrap_or(x_min);
    let right = corners.iter().copied().max().unwrap_or(x_max);
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VL Gothic: 1000 units per em, `xAvgCharWidth` 500, panose proportion 9.
    const VL_GOTHIC: FaceMetrics = FaceMetrics {
        units_per_em: 1000,
        average_width: 500,
        monospaced: true,
    };

    #[test]
    fn freetype_arithmetic_rounds_on_the_magnitude() {
        assert_eq!(mul_fix(1000, 1245), 19);
        assert_eq!(mul_fix(-64, SLANT), -16);
        assert_eq!(mul_fix(64, SLANT), 16);
        assert_eq!(mul_div(19, 65536, 1000), 1245);
        assert_eq!(mul_div(5, 65536, 1000), 328);
    }

    // Requirement: Advances of RGSS3

    #[test]
    fn a_full_width_glyph_of_a_fixed_pitch_face_is_twice_the_average_width() {
        // `あ` of VL Gothic, hinted 19 pixels at 19 pixels per em and 5 at 5, measured 18
        // and 6 at the sizes 24 and 6.
        assert_eq!(VL_GOTHIC.advance(19 * 64, 19), 18);
        assert_eq!(VL_GOTHIC.advance(5 * 64, 5), 6);
        // A half-width glyph keeps its hinted advance.
        assert_eq!(VL_GOTHIC.advance(10 * 64, 19), 10);
    }

    #[test]
    fn a_proportional_face_keeps_its_hinted_advances() {
        let proportional = FaceMetrics {
            monospaced: false,
            ..VL_GOTHIC
        };
        assert_eq!(proportional.advance(19 * 64, 19), 19);
        assert_eq!(proportional.advance(5 * 64, 5), 5);
    }

    #[test]
    fn hinted_advances_are_rounded_up() {
        let proportional = FaceMetrics {
            monospaced: false,
            ..VL_GOTHIC
        };
        assert_eq!(proportional.advance(9 * 64 + 1, 19), 10);
        assert_eq!(proportional.advance(9 * 64, 19), 9);
        assert_eq!(proportional.advance(-64, 19), 0);
    }

    // Requirement: Average width of RGSS3

    #[test]
    fn the_average_width_of_vl_gothic_at_size_24() {
        assert_eq!(VL_GOTHIC.average_width(19), 9);
        // The plain rule rounds 9.5 to 10.
        assert_eq!(crate::line_box::mul_div(500, 19, 1000), 10);
        // At least 1.
        assert_eq!(VL_GOTHIC.average_width(1), 1);
    }

    // Requirement: Glyph box of RGSS3

    #[test]
    fn the_slanted_box_of_a_glyph_that_is_not_emboldened() {
        // `j` of VL Gothic at size 6, hinted.
        let (min, max) = (Point::new(0.0, -1.0), Point::new(2.0, 5.0));
        assert_eq!(slanted_edges(min, max, true, true).0, -16);
        let (left, _) = extent(min, max, true, true);
        assert_eq!(left, -1);
        // Upright, the box keeps its whole pixels.
        assert_eq!(extent(min, max, true, false), (0, 2));
    }

    #[test]
    fn the_slanted_box_of_an_emboldened_glyph() {
        // The same `j` emboldened: its box is not on whole pixels.
        let (min, max) = (Point::new(0.515625, -1.0), Point::new(1.90625, 5.03125));
        assert_eq!(slanted_edges(min, max, false, true).0, 17);
        let (left, right) = extent(min, max, false, true);
        assert_eq!(left, 0);
        // 1.90625 + 5.03125 / 4, rounded up.
        assert_eq!(right, 4);
    }

    #[test]
    fn an_unslanted_box_is_rounded_outwards() {
        let (min, max) = (Point::new(-0.25, -0.5), Point::new(7.5, 9.0));
        assert_eq!(extent(min, max, true, false), (-1, 8));
        assert_eq!(extent(min, max, false, false), (-1, 8));
    }
}
