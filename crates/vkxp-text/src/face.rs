//! A font face loaded for measuring and rendering.

use std::sync::Arc;

use swash::{CacheKey, FontRef, GlyphId, NormalizedCoord, Setting, tag_from_bytes};

/// The weights a variable font is set to for a regular and for a bold request.
const REGULAR_WEIGHT: f32 = 400.0;
const BOLD_WEIGHT: f32 = 700.0;

/// A face with its file contents and the values read from it once.
pub(crate) struct LoadedFace {
    data: Arc<Vec<u8>>,
    offset: u32,
    key: CacheKey,
    /// Variation coordinates: empty for a font without a weight axis.
    coords: Vec<NormalizedCoord>,
    /// The weight a variable font is set to; `None` for a font without a weight axis.
    pub(crate) variable_weight: Option<u16>,
    /// The face's position in the font index.
    pub(crate) record: usize,
    pub(crate) units_per_em: f32,
    /// Distance from the baseline to the top of the line box, in font units.
    pub(crate) ascent: f32,
    /// Distance from the baseline to the bottom of the line box, in font units, positive.
    pub(crate) descent: f32,
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
        let units_per_em = f32::from(font.metrics(&[]).units_per_em);
        if units_per_em <= 0.0 {
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
        let (offset, key) = (font.offset, font.key);
        Some(Self {
            data,
            offset,
            key,
            coords,
            variable_weight,
            record,
            units_per_em,
            ascent,
            descent,
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
    pub(crate) fn advance(&self, glyph: GlyphId) -> f32 {
        self.font().glyph_metrics(&self.coords).advance_width(glyph)
    }

    /// Pixels per em at which the face's line box is `size` pixels high.
    pub(crate) fn pixels_per_em(&self, size: u32) -> f32 {
        size as f32 * self.units_per_em / (self.ascent + self.descent)
    }

    /// Distance in whole pixels from the top of a line box `size` pixels high to the baseline.
    pub(crate) fn baseline(&self, size: u32) -> i32 {
        (size as f32 * self.ascent / (self.ascent + self.descent)).round() as i32
    }
}

/// The ascent and descent, in font units, of the line box that the requested size refers to.
///
/// PLACEHOLDER: this assumes the original runtime asks GDI for a font by cell height, for
/// which Windows uses the OS/2 `usWinAscent` and `usWinDescent`. Whether `Font.size` is the
/// cell height or the em height is still to be measured against the original; the answer
/// changes only this function.
fn line_metrics(font: &FontRef<'_>, units_per_em: f32) -> (f32, f32) {
    let u16_at = |table: &[u8], at: usize| {
        table
            .get(at..at + 2)
            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
    };
    let win = font.table(tag_from_bytes(b"OS/2")).and_then(|table| {
        let ascent = f32::from(u16_at(table, 74)?);
        let descent = f32::from(u16_at(table, 76)?);
        (ascent + descent > 0.0).then_some((ascent, descent))
    });
    let hhea = || {
        font.table(tag_from_bytes(b"hhea")).and_then(|table| {
            let ascent = f32::from(u16_at(table, 4)? as i16);
            let descent = -f32::from(u16_at(table, 6)? as i16);
            (ascent + descent > 0.0).then_some((ascent, descent))
        })
    };
    win.or_else(hhea)
        .unwrap_or((units_per_em * 0.8, units_per_em * 0.2))
}
