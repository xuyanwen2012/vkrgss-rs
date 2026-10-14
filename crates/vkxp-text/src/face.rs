//! A font face loaded for measuring and rendering.

use std::sync::Arc;

use swash::zeno::Point;
use swash::{CacheKey, FontRef, GlyphId, NormalizedCoord, Setting, tag_from_bytes};

use crate::hinted::{FaceMetrics, HintingEngine, Hints};
use crate::line_box::LineBox;

/// The weights a variable font is set to for a regular and for a bold request.
const REGULAR_WEIGHT: f32 = 400.0;
const BOLD_WEIGHT: f32 = 700.0;

/// A face with its file contents and the values read from it once.
pub(crate) struct LoadedFace {
    data: Arc<Vec<u8>>,
    /// The face's index in the file, for collections.
    index: u32,
    offset: u32,
    key: CacheKey,
    /// Variation coordinates: empty for a font without a weight axis.
    coords: Vec<NormalizedCoord>,
    /// The weight a variable font is set to; `None` for a font without a weight axis.
    pub(crate) variable_weight: Option<u16>,
    /// The face's position in the font index.
    pub(crate) record: usize,
    pub(crate) units_per_em: u16,
    /// Distance from the baseline to the top of the line box, in font units.
    pub(crate) ascent: u16,
    /// Distance from the baseline to the bottom of the line box, in font units, positive.
    pub(crate) descent: u16,
    /// The average character width the font states, in font units.
    pub(crate) average_width: u16,
    /// Whether the OS/2 panose proportion is monospaced (9).
    pub(crate) monospaced: bool,
    /// The glyphs hinted for the metric rule of RGSS3.
    hints: Hints,
}

impl LoadedFace {
    /// Loads face `index` of the file contents `data`; `None` when the data is not a usable
    /// font.
    ///
    /// A font with a weight axis is set to regular weight, or to bold weight for `bold`,
    /// within the range of the axis. The default instance of such a font is often its
    /// thinnest design, which is not what a request for regular text means.
    pub(crate) fn new(data: Arc<Vec<u8>>, index: u32, record: usize, bold: bool) -> Option<Self> {
        let font = FontRef::from_index(&data, index as usize)?;
        let units_per_em = font.metrics(&[]).units_per_em;
        if units_per_em == 0 {
            return None;
        }
        let tag = tag_from_bytes(b"wght");
        let (coords, variable_weight) = match font.variations().find_by_tag(tag) {
            Some(axis) => {
                let wanted = if bold { BOLD_WEIGHT } else { REGULAR_WEIGHT };
                let value = wanted.clamp(axis.min_value(), axis.max_value());
                let coords = font
                    .variations()
                    .normalized_coords([Setting { tag, value }])
                    .collect();
                (coords, Some(value as u16))
            }
            None => (Vec::new(), None),
        };
        let (ascent, descent) = line_metrics(&font, units_per_em);
        let average_width = average_width(&font, units_per_em);
        let monospaced = monospaced(&font);
        let (offset, key) = (font.offset, font.key);
        let hints = Hints::new(
            skrifa::FontRef::from_index(&data, index).ok().as_ref(),
            &coords,
        );
        Some(Self {
            data,
            index,
            offset,
            key,
            coords,
            variable_weight,
            record,
            units_per_em,
            ascent,
            descent,
            average_width,
            monospaced,
            hints,
        })
    }

    pub(crate) fn font(&self) -> FontRef<'_> {
        FontRef {
            data: &self.data,
            offset: self.offset,
            key: self.key,
        }
    }

    pub(crate) fn coords(&self) -> &[NormalizedCoord] {
        &self.coords
    }

    /// The glyph for `ch`, 0 when the face has none.
    pub(crate) fn glyph(&self, ch: char) -> GlyphId {
        self.font().charmap().map(ch)
    }

    /// The advance width of `glyph` in font units.
    pub(crate) fn advance(&self, glyph: GlyphId) -> u32 {
        let advance = self.font().glyph_metrics(&self.coords).advance_width(glyph);
        advance.round().max(0.0) as u32
    }

    /// The engine the face is hinted with under the metric rule of RGSS3.
    pub(crate) fn hinting_engine(&self) -> HintingEngine {
        self.hints.engine()
    }

    /// What the metric rule of RGSS3 reads of the face besides its glyphs.
    pub(crate) fn metrics(&self) -> FaceMetrics {
        FaceMetrics {
            units_per_em: self.units_per_em,
            average_width: self.average_width,
            monospaced: self.monospaced,
        }
    }

    /// The hinted advance of `glyph` at `pixels_per_em` in 26.6; `None` when the glyph
    /// cannot be drawn.
    pub(crate) fn hinted_advance(&mut self, pixels_per_em: u32, glyph: GlyphId) -> Option<i32> {
        let font = skrifa::FontRef::from_index(&self.data, self.index).ok()?;
        self.hints.advance(&font, pixels_per_em, glyph)
    }

    /// The points of the hinted outline of `glyph` at `pixels_per_em`, in the order of the
    /// points of its unhinted outline.
    pub(crate) fn hinted_points(
        &mut self,
        pixels_per_em: u32,
        glyph: GlyphId,
    ) -> Option<Vec<Point>> {
        let font = skrifa::FontRef::from_index(&self.data, self.index).ok()?;
        self.hints.points(&font, pixels_per_em, glyph)
    }

    /// The face's own line box.
    pub(crate) fn line_box(&self) -> LineBox {
        LineBox {
            ascent: self.ascent,
            descent: self.descent,
            units_per_em: self.units_per_em,
        }
    }
}

fn u16_at(table: &[u8], at: usize) -> Option<u16> {
    table
        .get(at..at + 2)
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
}

/// The OS/2 `xAvgCharWidth`, which GDI reports as the average character width; half an em for
/// a font that does not state one.
fn average_width(font: &FontRef<'_>, units_per_em: u16) -> u16 {
    font.table(tag_from_bytes(b"OS/2"))
        .and_then(|table| u16_at(table, 2))
        .map(|width| width as i16)
        .filter(|&width| width > 0)
        .map_or(units_per_em / 2, |width| width as u16)
}

/// Whether the OS/2 panose proportion (`bProportion`) is 9, monospaced.
fn monospaced(font: &FontRef<'_>) -> bool {
    font.table(tag_from_bytes(b"OS/2"))
        .and_then(|table| table.get(35))
        .is_some_and(|&proportion| proportion == 9)
}

/// The ascent and descent, in font units, of the line box that the requested size refers to.
///
/// The requested size is the cell height asked of GDI, for which Windows uses the OS/2
/// `usWinAscent` and `usWinDescent`; `hhea` stands in for a font without them.
fn line_metrics(font: &FontRef<'_>, units_per_em: u16) -> (u16, u16) {
    let positive = |ascent: i32, descent: i32| {
        let ascent = u16::try_from(ascent.max(0)).ok()?;
        let descent = u16::try_from(descent.max(0)).ok()?;
        (u32::from(ascent) + u32::from(descent) > 0).then_some((ascent, descent))
    };
    let win = font
        .table(tag_from_bytes(b"OS/2"))
        .and_then(|table| positive(u16_at(table, 74)?.into(), u16_at(table, 76)?.into()));
    let hhea = || {
        font.table(tag_from_bytes(b"hhea")).and_then(|table| {
            let ascent = u16_at(table, 4)? as i16;
            let descent = u16_at(table, 6)? as i16;
            positive(ascent.into(), -i32::from(descent))
        })
    };
    win.or_else(hhea).unwrap_or_else(|| {
        let ascent = (u32::from(units_per_em) * 4 / 5) as u16;
        (ascent, units_per_em - ascent)
    })
}
