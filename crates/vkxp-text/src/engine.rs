//! The text engine: family lookup, measuring and rasterising.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use swash::GlyphId;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::{Format, Transform};

use crate::face::LoadedFace;
use crate::glyph_cache::{Glyph, GlyphCache, GlyphKey};
use crate::index::FontIndex;
use crate::line_box::{Cell, LineBox, LineBoxTable, mul_div};
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

/// Glyph coverage of a string: the cell of the text, as high as the text height and as wide as
/// the advances plus the overhangs of the first and the last glyph.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextMask {
    pub width: u32,
    pub height: u32,
    /// The column the pen starts at: how far the first glyph reaches left of it.
    pub left_overhang: u32,
    /// `width * height` bytes, rows from the top, 0 for no coverage to 255 for full.
    pub coverage: Vec<u8>,
}

/// How a string is set at a size: what is needed to place and to squeeze it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextLayout {
    /// The whole pixels per em the font is set at.
    pub pixels_per_em: u32,
    /// The text height: the rounded ascent plus the rounded descent.
    pub height: u32,
    /// Distance from the top of the text to the baseline.
    pub ascent: u32,
    /// The advance of each character, in order.
    pub advances: Vec<u32>,
    /// How far the first glyph reaches left of the pen.
    pub left_overhang: u32,
    /// How far the last glyph reaches right of its advance.
    pub right_overhang: u32,
    /// The font's average character width at this size, at least 1.
    pub average_width: u32,
    /// The pixels a synthesised bold adds to every advance and to the average width: 1, or
    /// 0 when nothing is synthesised.
    pub emboldened: u32,
}

impl TextLayout {
    /// The measured width: the advances plus the first glyph's left overhang.
    pub fn width(&self) -> u32 {
        let advances = self
            .advances
            .iter()
            .fold(0u32, |sum, &a| sum.saturating_add(a));
        advances.saturating_add(self.left_overhang)
    }
}

/// The slant of a synthesised italic: how far a point moves right per unit of height.
///
/// PLACEHOLDER: the value is not measured; the overhangs it gives differ from those of the
/// original runtime under Wine.
const SYNTHETIC_SLANT: f32 = 0.25;

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

/// What a request's family names and style select.
#[derive(Debug, Clone, Copy)]
struct Selected {
    /// The loaded face.
    face: usize,
    /// The line box of the family the face replaces, when the face was found through the
    /// substitution table and that family's line box is known. `None` sets the text in the
    /// face's own line box.
    line_box: Option<LineBox>,
}

/// A character resolved against a selected face and style.
#[derive(Debug, Clone, Copy)]
struct CharGlyph {
    /// The loaded face the glyph comes from.
    face: usize,
    glyph: GlyphId,
    /// The advance in the selected face's font units: the glyph's own, or that of the
    /// selected face's missing glyph for a character it lacks.
    advance: u32,
    /// Whether the selected face lacks the character.
    lacking: bool,
}

/// A glyph of the string being laid out.
#[derive(Debug, Clone, Copy)]
struct Placed {
    face: usize,
    glyph: GlyphId,
    /// The natural advance in pixels.
    advance: u32,
    /// Whether the glyph is emboldened and slanted, its face having no such style.
    bold: bool,
    italic: bool,
}

/// What a request selects at its size.
#[derive(Debug, Clone, Copy)]
struct Setting {
    cell: Cell,
    average_width: u32,
    emboldened: u32,
}

/// Finds fonts for family names and turns strings into coverage masks.
///
/// See the crate documentation for the rules. All methods that draw or measure take `&mut
/// self` because they fill caches; results never depend on what was asked before.
pub struct TextEngine {
    index: FontIndex,
    substitutions: SubstitutionTable,
    line_boxes: LineBoxTable,
    /// Whether a replacement keeps its own line box.
    own_metrics: bool,
    /// Family keys of the fallback families, in order.
    fallback_families: Vec<String>,
    /// Contents of the font files in use. `None` for a file that could not be read.
    files: HashMap<PathBuf, Option<Arc<Vec<u8>>>>,
    faces: Vec<LoadedFace>,
    /// Font index position and bold request to the loaded face. Both requests share a face
    /// unless the font has a weight axis.
    face_ids: HashMap<(usize, bool), Option<usize>>,
    /// Requested family names to what they select, one map per style.
    requests: [HashMap<Vec<String>, Option<Selected>>; 4],
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
            line_boxes: LineBoxTable::new(&sources.line_boxes),
            own_metrics: sources.own_metrics,
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
        let face = &self.faces[selected.face];
        let record = self.index.record(face.record);
        Some(ResolvedFace {
            family: record.families.first().cloned().unwrap_or_default(),
            path: record.path.clone(),
            index: record.index,
            weight: face.variable_weight.unwrap_or(record.weight),
            italic: record.italic,
        })
    }

    /// The width and height of `text` in pixels: the advances plus the first glyph's left
    /// overhang, and the text height. An empty string measures (0, 0).
    pub fn measure(&mut self, request: &TextRequest, text: &str) -> (u32, u32) {
        let layout = self.layout(request, text);
        if layout.advances.is_empty() {
            return (0, 0);
        }
        (layout.width(), layout.height)
    }

    /// How `text` is set. Without a usable font everything is 0 and there are no advances.
    pub fn layout(&mut self, request: &TextRequest, text: &str) -> TextLayout {
        let Some(setting) = self.set(request, text) else {
            return TextLayout::default();
        };
        let run = std::mem::take(&mut self.run);
        let pixels_per_em = setting.cell.pixels_per_em;
        let mut overhang = |placed: Option<&Placed>| match placed {
            Some(placed) => {
                let glyph = self.glyph(placed.key(pixels_per_em, (1, 1)));
                (glyph.left_overhang(), glyph.right_overhang(placed.advance))
            }
            None => (0, 0),
        };
        let layout = TextLayout {
            pixels_per_em,
            height: setting.cell.ascent + setting.cell.descent,
            ascent: setting.cell.ascent,
            left_overhang: overhang(run.first()).0,
            right_overhang: overhang(run.last()).1,
            advances: run.iter().map(|placed| placed.advance).collect(),
            average_width: setting.average_width,
            emboldened: setting.emboldened,
        };
        self.run = run;
        layout
    }

    /// The coverage of `text` at its natural advances: [`TextEngine::rasterize_run`] with
    /// the advances of [`TextEngine::layout`].
    pub fn rasterize(&mut self, request: &TextRequest, text: &str) -> TextMask {
        let advances = self.layout(request, text).advances;
        self.rasterize_run(request, text, &advances, None)
    }

    /// The coverage of `text` with the characters set `advances` apart, one advance per
    /// character, and every glyph `condense.0 / condense.1` as wide as it naturally is.
    ///
    /// The mask is as high as the text height and holds the advances and the overhangs of
    /// the first and the last glyph as drawn; other ink beyond it is clipped.
    pub fn rasterize_run(
        &mut self,
        request: &TextRequest,
        text: &str,
        advances: &[u32],
        condense: Option<(u32, u32)>,
    ) -> TextMask {
        let Some(setting) = self.set(request, text) else {
            return TextMask::default();
        };
        let condense = match condense {
            Some((numerator, denominator)) if numerator != denominator => (numerator, denominator),
            _ => (1, 1),
        };
        let run = std::mem::take(&mut self.run);
        let glyphs: Vec<(Arc<Glyph>, u32)> = run
            .iter()
            .zip(advances)
            .map(|(placed, &advance)| {
                let key = placed.key(setting.cell.pixels_per_em, condense);
                (self.glyph(key), advance)
            })
            .collect();
        self.run = run;

        let left_overhang = glyphs.first().map_or(0, |(glyph, _)| glyph.left_overhang());
        let right_overhang = glyphs
            .last()
            .map_or(0, |(glyph, advance)| glyph.right_overhang(*advance));
        let width = glyphs
            .iter()
            .fold(left_overhang, |sum, (_, advance)| {
                sum.saturating_add(*advance)
            })
            .saturating_add(right_overhang);
        let height = setting.cell.ascent + setting.cell.descent;
        let mut mask = TextMask {
            width,
            height,
            left_overhang,
            coverage: vec![0; width as usize * height as usize],
        };
        if mask.coverage.is_empty() {
            return mask;
        }
        let mut pen = i64::from(left_overhang);
        for (glyph, advance) in &glyphs {
            let top = i64::from(setting.cell.ascent) - i64::from(glyph.top);
            blit(&mut mask, glyph, pen + i64::from(glyph.left), top);
            pen += i64::from(*advance);
        }
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

    /// Fills `self.run` with the glyphs of `text`, one for each character, and returns how
    /// the request is set. `None` when the engine has no usable font.
    fn set(&mut self, request: &TextRequest, text: &str) -> Option<Setting> {
        self.run.clear();
        let selection = self.selected_face(request)?;
        let selected = selection.face;
        let face = &self.faces[selected];
        let units_per_em = u32::from(face.units_per_em);
        let cell = selection
            .line_box
            .unwrap_or_else(|| face.line_box())
            .cell(request.size);
        let pixels_per_em = cell.pixels_per_em;
        let (has_bold, _) = self.face_style(selected);
        let emboldened = u32::from(request.bold && !has_bold);
        let average_width = mul_div(face.average_width.into(), pixels_per_em, units_per_em);
        let style = style_bits(request.bold, request.italic);
        for ch in text.chars() {
            let found = match self.chars.get(&(selected, style, ch)) {
                Some(found) => *found,
                None => {
                    let found = self.find_glyph(selected, request.bold, request.italic, ch);
                    self.chars.insert((selected, style, ch), found);
                    found
                }
            };
            let mut advance = advance_pixels(found.advance, pixels_per_em, units_per_em)
                .saturating_add(emboldened);
            // As measured: a character outside the Basic Multilingual Plane is two units to
            // GDI, each of which the font lacks.
            if found.lacking && u32::from(ch) > 0xFFFF {
                advance = advance.saturating_mul(2);
            }
            let (glyph_bold, glyph_italic) = self.face_style(found.face);
            self.run.push(Placed {
                face: found.face,
                glyph: found.glyph,
                advance,
                bold: request.bold && !glyph_bold,
                italic: request.italic && !glyph_italic,
            });
        }
        Some(Setting {
            cell,
            average_width: average_width.max(1) + emboldened,
            emboldened,
        })
    }

    /// Whether a loaded face is bold and whether it is italic.
    fn face_style(&self, face: usize) -> (bool, bool) {
        let loaded = &self.faces[face];
        let record = self.index.record(loaded.record);
        let bold = loaded
            .variable_weight
            .map_or(record.is_bold(), |weight| weight >= 600);
        (bold, record.italic)
    }

    /// The loaded face and line box for the request's family names and style.
    fn selected_face(&mut self, request: &TextRequest) -> Option<Selected> {
        let style = usize::from(style_bits(request.bold, request.italic));
        if let Some(&face) = self.requests[style].get(request.families.as_slice()) {
            return face;
        }
        let face = self.select_face(&request.families, request.bold, request.italic);
        self.requests[style].insert(request.families.clone(), face);
        face
    }

    fn select_face(&mut self, names: &[String], bold: bool, italic: bool) -> Option<Selected> {
        let own = |face| Selected {
            face,
            line_box: None,
        };
        for name in names {
            let key = family_key(name);
            if let Some(face) = self.family_face(&key, bold, italic) {
                return Some(own(face));
            }
            for replacement in self.substitutions.replacements(&key).to_vec() {
                if let Some(face) = self.family_face(&replacement, bold, italic) {
                    if self.own_metrics {
                        return Some(own(face));
                    }
                    // The replacement is set in the line box of the name it stands in for.
                    return Some(Selected {
                        face,
                        line_box: self.line_boxes.get(&key),
                    });
                }
            }
        }
        for position in 0..self.fallback_families.len() {
            let key = self.fallback_families[position].clone();
            if let Some(face) = self.family_face(&key, bold, italic) {
                return Some(own(face));
            }
        }
        // None of the fallback families is installed: any font is better than no text.
        self.index
            .scan_order()
            .to_vec()
            .into_iter()
            .find_map(|record| self.load(record, bold))
            .map(own)
    }

    /// The loaded face of the family `key` that suits the style best.
    fn family_face(&mut self, key: &str, bold: bool, italic: bool) -> Option<usize> {
        let record = self.index.pick(key, bold, italic)?;
        self.load(record, bold)
    }

    /// The glyph that draws `ch` when `selected` is the selected face: its own, else the
    /// first fallback family's that has one, else any font's, else its missing-glyph box.
    /// A glyph of another face is set at the advance of the selected face's missing glyph,
    /// and a control character is not looked for in other faces.
    ///
    /// PLACEHOLDER: the original draws the missing glyph for a character the font lacks.
    fn find_glyph(&mut self, selected: usize, bold: bool, italic: bool, ch: char) -> CharGlyph {
        let own = self.faces[selected].glyph(ch);
        if own != 0 {
            return CharGlyph {
                face: selected,
                glyph: own,
                advance: self.faces[selected].advance(own),
                lacking: false,
            };
        }
        let missing = CharGlyph {
            face: selected,
            glyph: 0,
            advance: self.faces[selected].advance(0),
            lacking: true,
        };
        if ch.is_control() {
            return missing;
        }
        let from = |faces: &[LoadedFace], face: usize| {
            let glyph = faces[face].glyph(ch);
            (glyph != 0).then_some(CharGlyph {
                face,
                glyph,
                ..missing
            })
        };
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
        missing
    }

    /// Loads the face at font index position `record`. `bold` matters only for a font with a
    /// weight axis, which is set to its own bold design.
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
    fn glyph(&mut self, key: GlyphKey) -> Arc<Glyph> {
        if let Some(cached) = self.glyphs.get(key) {
            return cached;
        }
        let (numerator, denominator) = key.condense;
        // Nothing can be drawn at no size or no width.
        if key.pixels_per_em == 0 || numerator == 0 || denominator == 0 {
            return Arc::new(Glyph::default());
        }
        let loaded = &self.faces[key.face];
        let mut scaler = self
            .scaler
            .builder(loaded.font())
            .size(key.pixels_per_em as f32)
            .hint(false)
            .normalized_coords(loaded.coords())
            .build();
        let slant = if key.italic { SYNTHETIC_SLANT } else { 0.0 };
        let transform = (key.italic || numerator != denominator).then(|| {
            Transform::new(
                numerator as f32 / denominator as f32,
                0.0,
                slant,
                1.0,
                0.0,
                0.0,
            )
        });
        let rendered = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .transform(transform)
            .render(&mut scaler, key.glyph)
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
        let rendered = Arc::new(if key.bold {
            embolden(&rendered)
        } else {
            rendered
        });
        self.glyphs.insert(key, Arc::clone(&rendered));
        rendered
    }
}

impl Placed {
    fn key(&self, pixels_per_em: u32, condense: (u32, u32)) -> GlyphKey {
        GlyphKey {
            face: self.face,
            pixels_per_em,
            glyph: self.glyph,
            bold: self.bold,
            italic: self.italic,
            condense,
        }
    }
}

/// The advance of `units` font units in whole pixels at `pixels_per_em`, rounded the way
/// FreeType rounds it, which is twice: the scale to 16.16 fixed point, the advance to
/// 26.6 and then to whole pixels.
fn advance_pixels(units: u32, pixels_per_em: u32, units_per_em: u32) -> u32 {
    let units_per_em = u64::from(units_per_em.max(1));
    let scale = (u64::from(pixels_per_em) * 64 * 65536 * 2 + units_per_em) / (2 * units_per_em);
    let advance = (u64::from(units) * scale + 0x8000) >> 16;
    ((advance + 32) >> 6).min(u64::from(u32::MAX)) as u32
}

/// `glyph` one pixel bolder: every pixel also covers its right neighbour.
fn embolden(glyph: &Glyph) -> Glyph {
    if glyph.width == 0 || glyph.height == 0 {
        return Glyph::default();
    }
    let (old, new) = (glyph.width as usize, glyph.width as usize + 1);
    let mut coverage = vec![0; new * glyph.height as usize];
    for (from, to) in glyph.coverage.chunks(old).zip(coverage.chunks_mut(new)) {
        for (x, &value) in from.iter().enumerate() {
            to[x] = to[x].max(value);
            to[x + 1] = value;
        }
    }
    Glyph {
        left: glyph.left,
        top: glyph.top,
        width: glyph.width + 1,
        height: glyph.height,
        coverage,
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
            left_overhang: 0,
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

    #[test]
    fn advances_round_twice() {
        // 812 units of 1000 at 8 pixels per em are 6.496 pixels: 6.5 in 26.6, then 7.
        assert_eq!(advance_pixels(812, 8, 1000), 7);
        assert_eq!(advance_pixels(1000, 15, 1000), 15);
        assert_eq!(advance_pixels(555, 8, 1000), 4);
        assert_eq!(advance_pixels(1000, 0, 1000), 0);
    }

    #[test]
    fn emboldening_widens_by_one_pixel() {
        let thin = Glyph {
            left: 2,
            top: 5,
            width: 3,
            height: 2,
            coverage: vec![0, 200, 0, 100, 0, 50],
        };
        let bold = embolden(&thin);
        assert_eq!((bold.left, bold.top, bold.width, bold.height), (2, 5, 4, 2));
        assert_eq!(bold.coverage, [0, 200, 200, 0, 100, 100, 50, 50]);
        assert_eq!(embolden(&Glyph::default()).width, 0);
    }
}
