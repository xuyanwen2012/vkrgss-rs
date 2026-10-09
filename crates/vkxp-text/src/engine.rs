//! The text engine: family lookup, measuring and rasterising.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use swash::GlyphId;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;

use crate::face::LoadedFace;
use crate::glyph_cache::{Glyph, GlyphCache, GlyphKey};
use crate::index::FontIndex;
use crate::sources::FontSources;
use crate::substitution::{SubstitutionTable, family_key};

/// The families used when no requested name matches, and for characters the selected font
/// lacks, unless the caller names others. Families that are not installed are skipped.
pub const DEFAULT_FALLBACK_FAMILIES: &[&str] = &[
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK TC",
    "Noto Sans CJK KR",
    "Noto Serif CJK SC",
    "Source Han Sans SC",
    "WenQuanYi Micro Hei",
    "DejaVu Sans",
    "Liberation Sans",
];

/// The default bound of the glyph cache, in bytes.
pub const DEFAULT_GLYPH_CACHE_BYTES: usize = 16 * 1024 * 1024;

/// What to draw text with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TextRequest {
    /// Family names in order of preference.
    pub families: Vec<String>,
    /// The height of the line box in pixels.
    pub size: u32,
    pub bold: bool,
    pub italic: bool,
}

impl TextRequest {
    /// A request for upright text of regular weight.
    pub fn new(families: impl IntoIterator<Item = impl Into<String>>, size: u32) -> Self {
        Self {
            families: families.into_iter().map(Into::into).collect(),
            size,
            bold: false,
            italic: false,
        }
    }
}

/// Glyph coverage of a string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextMask {
    pub width: u32,
    pub height: u32,
    /// `width * height` bytes, rows from the top, 0 for no coverage to 255 for full.
    pub coverage: Vec<u8>,
}

/// The face a request resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFace {
    /// The face's family name.
    pub family: String,
    /// The font file.
    pub path: PathBuf,
    /// The face's index in the file, for collections.
    pub index: u32,
    /// The weight the face is used at, 400 for regular and 700 for bold. For a font with a
    /// weight axis this is the weight the axis is set to.
    pub weight: u16,
    pub italic: bool,
}

/// A character resolved against a selected face and style.
#[derive(Debug, Clone, Copy)]
struct CharGlyph {
    /// The loaded face the glyph comes from.
    face: usize,
    glyph: GlyphId,
    /// The glyph's advance in that face's font units.
    advance: f32,
}

/// A glyph of the string being laid out.
#[derive(Debug, Clone, Copy)]
struct Placed {
    face: usize,
    glyph: GlyphId,
    /// The pen position: the sum of the advances before the glyph.
    x: u32,
}

/// Finds fonts for family names and turns strings into coverage masks.
///
/// See the crate documentation for the rules. All methods that draw or measure take `&mut
/// self` because they fill caches; results never depend on what was asked before.
pub struct TextEngine {
    index: FontIndex,
    substitutions: SubstitutionTable,
    /// Family keys of the fallback families, in order.
    fallback_families: Vec<String>,
    /// Contents of the font files in use. `None` for a file that could not be read.
    files: HashMap<PathBuf, Option<Arc<Vec<u8>>>>,
    faces: Vec<LoadedFace>,
    /// Font index position and bold request to the loaded face. Both requests share a face
    /// unless the font has a weight axis.
    face_ids: HashMap<(usize, bool), Option<usize>>,
    /// Requested family names to the selected loaded face, one map per style.
    requests: [HashMap<Vec<String>, Option<usize>>; 4],
    /// Selected face, style and character to the glyph that draws the character.
    chars: HashMap<(usize, u8, char), CharGlyph>,
    /// Characters that neither the selected nor a fallback face had, to the font index
    /// position of the face that has them.
    last_resort: HashMap<char, Option<usize>>,
    glyphs: GlyphCache,
    scaler: ScaleContext,
    /// The glyphs of the string being laid out; kept to reuse its allocation.
    run: Vec<Placed>,
}

impl fmt::Debug for TextEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextEngine")
            .field("families", &self.index.families().len())
            .field("loaded_faces", &self.faces.len())
            .field("glyph_cache_bytes", &self.glyphs.bytes())
            .finish_non_exhaustive()
    }
}

fn style_bits(bold: bool, italic: bool) -> u8 {
    u8::from(bold) | u8::from(italic) << 1
}

impl TextEngine {
    /// Builds an engine from `sources`. Unreadable directories and malformed fonts are
    /// skipped; an engine without any font measures every string as zero pixels wide.
    pub fn new(sources: &FontSources) -> Self {
        let fallback_families = match &sources.fallback_families {
            Some(families) => families.iter().map(|name| family_key(name)).collect(),
            None => DEFAULT_FALLBACK_FAMILIES
                .iter()
                .map(|name| family_key(name))
                .collect(),
        };
        Self {
            index: FontIndex::new(sources),
            substitutions: SubstitutionTable::new(&sources.substitutions),
            fallback_families,
            files: HashMap::new(),
            faces: Vec::new(),
            face_ids: HashMap::new(),
            requests: Default::default(),
            chars: HashMap::new(),
            last_resort: HashMap::new(),
            glyphs: GlyphCache::new(DEFAULT_GLYPH_CACHE_BYTES),
            scaler: ScaleContext::new(),
            run: Vec::new(),
        }
    }

    /// The names of the families the engine has fonts for, sorted. Substituted names are not
    /// listed.
    pub fn families(&self) -> Vec<String> {
        self.index.families()
    }

    /// Whether text in `family` would be drawn with that family or with one of its
    /// substitutes, rather than with a fallback font.
    pub fn is_available(&self, family: &str) -> bool {
        let key = family_key(family);
        self.index.has_family(&key)
            || self
                .substitutions
                .replacements(&key)
                .iter()
                .any(|replacement| self.index.has_family(replacement))
    }

    /// The face `request` selects, before per-character fallback. `None` only when the engine
    /// has no usable font at all.
    pub fn resolve(&mut self, request: &TextRequest) -> Option<ResolvedFace> {
        let selected = self.selected_face(request)?;
        let face = &self.faces[selected];
        let record = self.index.record(face.record);
        Some(ResolvedFace {
            family: record.families.first().cloned().unwrap_or_default(),
            path: record.path.clone(),
            index: record.index,
            weight: face.variable_weight.unwrap_or(record.weight),
            italic: record.italic,
        })
    }

    /// The width and height of `text` in pixels. The height is always the requested size.
    pub fn measure(&mut self, request: &TextRequest, text: &str) -> (u32, u32) {
        let width = self.layout(request, text).map_or(0, |(_, width)| width);
        (width, request.size)
    }

    /// The coverage of `text`: a mask of exactly the size [`TextEngine::measure`] reports.
    pub fn rasterize(&mut self, request: &TextRequest, text: &str) -> TextMask {
        let layout = self.layout(request, text);
        let width = layout.map_or(0, |(_, width)| width);
        let mut mask = TextMask {
            width,
            height: request.size,
            coverage: vec![0; width as usize * request.size as usize],
        };
        let Some((selected, _)) = layout else {
            return mask;
        };
        if mask.coverage.is_empty() {
            return mask;
        }
        let pixels_per_em = self.faces[selected].pixels_per_em(request.size);
        let baseline = self.faces[selected].baseline(request.size);
        let run = std::mem::take(&mut self.run);
        for placed in &run {
            let glyph = self.glyph(placed.face, pixels_per_em, placed.glyph);
            let left = i64::from(placed.x) + i64::from(glyph.left);
            let top = i64::from(baseline) - i64::from(glyph.top);
            blit(&mut mask, &glyph, left, top);
        }
        self.run = run;
        mask
    }

    /// The bytes the glyph cache holds at the moment.
    pub fn glyph_cache_bytes(&self) -> usize {
        self.glyphs.bytes()
    }

    /// Changes the bound of the glyph cache, dropping glyphs if it holds more.
    pub fn set_glyph_cache_limit(&mut self, bytes: usize) {
        self.glyphs.set_limit(bytes);
    }

    /// Fills `self.run` with the glyphs of `text` and returns the selected face and the
    /// width. `None` when the size is zero or the engine has no usable font.
    fn layout(&mut self, request: &TextRequest, text: &str) -> Option<(usize, u32)> {
        self.run.clear();
        if request.size == 0 {
            return None;
        }
        let selected = self.selected_face(request)?;
        let pixels_per_em = self.faces[selected].pixels_per_em(request.size);
        let style = style_bits(request.bold, request.italic);
        let mut x = 0u32;
        for ch in text.chars() {
            // Control characters, line breaks included, take no space and draw nothing.
            if ch.is_control() {
                continue;
            }
            let found = match self.chars.get(&(selected, style, ch)) {
                Some(found) => *found,
                None => {
                    let found = self.find_glyph(selected, request.bold, request.italic, ch);
                    self.chars.insert((selected, style, ch), found);
                    found
                }
            };
            self.run.push(Placed {
                face: found.face,
                glyph: found.glyph,
                x,
            });
            let scale = pixels_per_em / self.faces[found.face].units_per_em;
            let advance = (found.advance * scale).round().max(0.0) as u32;
            x = x.saturating_add(advance);
        }
        Some((selected, x))
    }

    /// The loaded face for the request's family names and style.
    fn selected_face(&mut self, request: &TextRequest) -> Option<usize> {
        let style = usize::from(style_bits(request.bold, request.italic));
        if let Some(&face) = self.requests[style].get(request.families.as_slice()) {
            return face;
        }
        let face = self.select_face(&request.families, request.bold, request.italic);
        self.requests[style].insert(request.families.clone(), face);
        face
    }

    fn select_face(&mut self, names: &[String], bold: bool, italic: bool) -> Option<usize> {
        for name in names {
            let key = family_key(name);
            if let Some(face) = self.family_face(&key, bold, italic) {
                return Some(face);
            }
            for replacement in self.substitutions.replacements(&key).to_vec() {
                if let Some(face) = self.family_face(&replacement, bold, italic) {
                    return Some(face);
                }
            }
        }
        for position in 0..self.fallback_families.len() {
            let key = self.fallback_families[position].clone();
            if let Some(face) = self.family_face(&key, bold, italic) {
                return Some(face);
            }
        }
        // None of the fallback families is installed: any font is better than no text.
        self.index
            .scan_order()
            .to_vec()
            .into_iter()
            .find_map(|record| self.load(record, bold))
    }

    /// The loaded face of the family `key` that suits the style best.
    fn family_face(&mut self, key: &str, bold: bool, italic: bool) -> Option<usize> {
        let record = self.index.pick(key, bold, italic)?;
        self.load(record, bold)
    }

    /// The glyph that draws `ch` when `selected` is the selected face: its own, else the
    /// first fallback family's that has one, else any font's, else its missing-glyph box.
    fn find_glyph(&mut self, selected: usize, bold: bool, italic: bool, ch: char) -> CharGlyph {
        let from = |faces: &[LoadedFace], face: usize| {
            let glyph = faces[face].glyph(ch);
            (glyph != 0).then(|| CharGlyph {
                face,
                glyph,
                advance: faces[face].advance(glyph),
            })
        };
        if let Some(found) = from(&self.faces, selected) {
            return found;
        }
        for position in 0..self.fallback_families.len() {
            let key = self.fallback_families[position].clone();
            if let Some(face) = self.family_face(&key, bold, italic)
                && let Some(found) = from(&self.faces, face)
            {
                return found;
            }
        }
        let record = match self.last_resort.get(&ch) {
            Some(&record) => record,
            None => {
                let record = self.index.find_char(ch);
                self.last_resort.insert(ch, record);
                record
            }
        };
        if let Some(record) = record
            && let Some(face) = self.load(record, bold)
            && let Some(found) = from(&self.faces, face)
        {
            return found;
        }
        CharGlyph {
            face: selected,
            glyph: 0,
            advance: self.faces[selected].advance(0),
        }
    }

    /// Loads the face at font index position `record`. `bold` matters only for a font with a
    /// weight axis, which is set to its own bold design; nothing is synthesised.
    fn load(&mut self, record: usize, bold: bool) -> Option<usize> {
        if let Some(&face) = self.face_ids.get(&(record, bold)) {
            return face;
        }
        let face = self.load_uncached(record, bold);
        self.face_ids.insert((record, bold), face);
        if face.is_none_or(|face| self.faces[face].variable_weight.is_none()) {
            self.face_ids.insert((record, !bold), face);
        }
        face
    }

    fn load_uncached(&mut self, record: usize, bold: bool) -> Option<usize> {
        let (path, index) = {
            let record = self.index.record(record);
            (record.path.clone(), record.index)
        };
        // The whole file is read into memory: mapping it would need `unsafe`.
        let data = self
            .files
            .entry(path)
            .or_insert_with_key(|path| std::fs::read(path).ok().map(Arc::new))
            .clone()?;
        self.faces.push(LoadedFace::new(data, index, record, bold)?);
        Some(self.faces.len() - 1)
    }

    /// The image of a glyph, from the cache or rendered now.
    fn glyph(&mut self, face: usize, pixels_per_em: f32, glyph: GlyphId) -> Arc<Glyph> {
        let key = GlyphKey {
            face,
            pixels_per_em: pixels_per_em.to_bits(),
            glyph,
        };
        if let Some(cached) = self.glyphs.get(key) {
            return cached;
        }
        let loaded = &self.faces[face];
        let mut scaler = self
            .scaler
            .builder(loaded.font())
            .size(pixels_per_em)
            .hint(false)
            .normalized_coords(loaded.coords())
            .build();
        let rendered = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .render(&mut scaler, glyph)
            .and_then(|image| {
                let placement = image.placement;
                let bytes = placement.width as usize * placement.height as usize;
                (image.data.len() == bytes).then_some(Glyph {
                    left: placement.left,
                    top: placement.top,
                    width: placement.width,
                    height: placement.height,
                    coverage: image.data,
                })
            })
            // A glyph without an outline, such as a space, draws nothing.
            .unwrap_or_default();
        let rendered = Arc::new(rendered);
        self.glyphs.insert(key, Arc::clone(&rendered));
        rendered
    }
}

/// Composes `glyph` into `mask` with its top-left corner at (`left`, `top`), keeping the
/// larger coverage where glyphs overlap and dropping what falls outside the mask.
fn blit(mask: &mut TextMask, glyph: &Glyph, left: i64, top: i64) {
    let (mask_width, mask_height) = (i64::from(mask.width), i64::from(mask.height));
    let (glyph_width, glyph_height) = (i64::from(glyph.width), i64::from(glyph.height));
    let x_range = left.max(0)..(left + glyph_width).min(mask_width);
    let y_range = top.max(0)..(top + glyph_height).min(mask_height);
    if x_range.is_empty() {
        return;
    }
    let columns = (x_range.end - x_range.start) as usize;
    for y in y_range {
        let source = ((y - top) * glyph_width + (x_range.start - left)) as usize;
        let target = (y * mask_width + x_range.start) as usize;
        let source = &glyph.coverage[source..source + columns];
        let target = &mut mask.coverage[target..target + columns];
        for (to, from) in target.iter_mut().zip(source) {
            *to = (*to).max(*from);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(width: u32, height: u32) -> TextMask {
        TextMask {
            width,
            height,
            coverage: vec![0; (width * height) as usize],
        }
    }

    fn glyph(width: u32, height: u32, value: u8) -> Glyph {
        Glyph {
            width,
            height,
            coverage: vec![value; (width * height) as usize],
            ..Glyph::default()
        }
    }

    #[test]
    fn overlapping_glyphs_keep_the_larger_coverage() {
        let mut mask = mask(4, 1);
        blit(&mut mask, &glyph(3, 1, 200), 0, 0);
        blit(&mut mask, &glyph(3, 1, 100), 1, 0);
        assert_eq!(mask.coverage, [200, 200, 200, 100]);
        blit(&mut mask, &glyph(2, 1, 255), 2, 0);
        assert_eq!(mask.coverage, [200, 200, 255, 255]);
    }

    #[test]
    fn coverage_outside_the_mask_is_dropped() {
        // A 3x3 glyph whose bytes number its pixels, moved over every edge of a 2x2 mask.
        let numbered = Glyph {
            width: 3,
            height: 3,
            coverage: (1..=9).collect(),
            ..Glyph::default()
        };
        let drawn = |left: i64, top: i64| {
            let mut mask = mask(2, 2);
            blit(&mut mask, &numbered, left, top);
            mask.coverage
        };
        assert_eq!(drawn(0, 0), [1, 2, 4, 5]);
        assert_eq!(drawn(-1, -1), [5, 6, 8, 9]);
        assert_eq!(drawn(1, 1), [0, 0, 0, 1]);
        assert_eq!(drawn(-2, 1), [0, 0, 3, 0]);
        for (left, top) in [(2, 0), (0, 2), (-3, 0), (0, -3), (i64::MAX / 2, 0)] {
            assert_eq!(drawn(left, top), [0; 4], "at ({left}, {top})");
        }
        // A mask without pixels takes nothing.
        let mut empty = mask(0, 2);
        blit(&mut empty, &numbered, 0, 0);
        assert!(empty.coverage.is_empty());
    }
}
