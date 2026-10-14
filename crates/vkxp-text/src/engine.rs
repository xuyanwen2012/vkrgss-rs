//! The text engine: family lookup, measuring and rasterising.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use swash::GlyphId;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::{Format, Mask, Origin, Point, Transform, Vector};

use crate::face::LoadedFace;
use crate::glyph_cache::{Glyph, GlyphCache, GlyphKey};
use crate::hinted::{self, HintingEngine};
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

/// The rule a mask is made by: how the coverage of a glyph becomes a mask value and what a
/// glyph does to the values earlier glyphs wrote where they overlap. Measured sizes and
/// advances do not depend on it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MaskRules {
    /// The coverage as the rasteriser gives it, and the larger value where glyphs overlap:
    /// the masks of RGSS1 and RGSS2.
    #[default]
    Plain,
    /// RGSS3 as its DLL 3.0.1.1 makes a mask: the coverage in 65 levels, a level `v` held as
    /// `trunc(v * 255 / 64)`, and a glyph replacing earlier values wherever its own level is
    /// above 0.
    ///
    /// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1 (235 draws against RGSS2,
    /// 220 whole draws against a mask made from GDI's answers).
    Rgss3,
    /// RGSS3 as its DLL 3.0.0.1 makes a mask: the levels of [`MaskRules::Rgss3`], and a
    /// glyph replacing every pixel of its own box, also where its level is 0.
    ///
    /// Provenance: the same measurement on RGSS300 3.0.0.1.
    Rgss3Early,
}

/// The rule text is measured and set by: where advances, overhangs, the average width and
/// the glyph images come from. It does not depend on the [`MaskRules`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MetricRules {
    /// The unhinted outline: advances of the face rounded as FreeType rounds them, the
    /// overhangs of vkxp's box model, the average width rounded: the metrics of RGSS1 and
    /// RGSS2.
    #[default]
    Plain,
    /// RGSS3 as its DLL measures text under Wine's GDI: glyphs hinted as FreeType hints them
    /// for an anti-aliased target, the hinted advance rounded up with Wine's rule for
    /// full-width glyphs of a fixed-pitch font, the overhangs from the control box of the
    /// hinted outline, the average width as Wine's `tmAveCharWidth`, and glyph images from
    /// the hinted outline.
    ///
    /// Provenance: the `text_size` table of VL Gothic and VL PGothic, 28392 values measured
    /// on RGSS301 3.0.1.1 and RGSS300 3.0.0.1 under Wine 11.0 on 2026-10-13, all reproduced.
    Rgss3,
}

/// The level from 0 to 64 that stands for GDI's answer for a pixel of coverage `coverage`.
///
/// PLACEHOLDER: this is how Wine's GDI, under which the rule was measured, is believed to
/// reduce the 256 levels of its rasteriser; it was not checked against Wine's source. What
/// is established is only that the original has 65 levels.
fn coverage_level(coverage: u8) -> u8 {
    (u32::from(coverage) * 65 / 256) as u8
}

/// The mask value RGSS3 holds for the level `level` of 0 to 64: `trunc(level * 255 / 64)`.
fn scaled_level(level: u8) -> u8 {
    (u32::from(level.min(64)) * 255 / 64) as u8
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

/// The strength of a synthesised bold as a fraction of the pixels per em: the outline grows
/// by twice this, a 24th of the em, to the right and upwards, and a little beyond at sharp
/// corners.
///
/// PLACEHOLDER: the strength Wine's GDI emboldens with, under which the original runtime
/// was measured.
const SYNTHETIC_BOLD: f32 = 1.0 / 48.0;

/// How far below a pixel row, in pixels, the bottom of a glyph that ends less than a pixel
/// below the baseline may lie and still count as ending on that row, where the box GDI
/// slants comes from the hinted glyph; a deeper bottom is on the nearest row.
///
/// PLACEHOLDER: fitted to the left overhangs measured in italic with Noto Sans CJK SC under
/// Wine's GDI, not derived from its hinter.
const SHALLOW_BOTTOM: f32 = 0.25;

/// The slant of a synthesised italic: how far a point moves right per unit of height.
///
/// PLACEHOLDER: the slant of Wine's GDI, under which the original runtime was measured; it
/// is the value that fits the measured overhangs.
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
    /// The glyph's advance in the font units of `face`.
    advance: u32,
    /// Whether the glyph is the selected face's missing glyph.
    missing: bool,
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
    mask_rules: MaskRules,
    metric_rules: MetricRules,
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
            mask_rules: MaskRules::default(),
            metric_rules: MetricRules::default(),
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

    /// Whether `name` exists exactly, as `Font.exist?` of RGSS3 asks: whether it equals,
    /// character for character and with regard to letter case, a family name an available
    /// font declares, or a name of the substitution table as the table spells it when one of
    /// its replacements is available.
    ///
    /// The second half is this engine's: the original tests against the fonts Windows has,
    /// and the table is what stands for them here. The vertical variants of GDI, whose
    /// names begin with `@`, are not known and answer false.
    pub fn has_family_exactly(&self, name: &str) -> bool {
        self.index.has_family_exactly(name)
            || (self.substitutions.spells(name)
                && self
                    .substitutions
                    .replacements(&family_key(name))
                    .iter()
                    .any(|replacement| self.index.has_family(replacement)))
    }

    /// The position of the name a name list of RGSS3 stands for: the first that exists
    /// exactly ([`TextEngine::has_family_exactly`]), the first of the list when none does,
    /// and `None` for an empty list.
    ///
    /// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, api probe.
    pub fn first_existing<S: AsRef<str>>(&self, names: &[S]) -> Option<usize> {
        if names.is_empty() {
            return None;
        }
        let existing = names
            .iter()
            .position(|name| self.has_family_exactly(name.as_ref()));
        Some(existing.unwrap_or(0))
    }

    /// The rule masks are made by.
    pub fn mask_rules(&self) -> MaskRules {
        self.mask_rules
    }

    /// Sets the rule masks are made by from now on. Nothing cached depends on it.
    pub fn set_mask_rules(&mut self, rules: MaskRules) {
        self.mask_rules = rules;
    }

    /// The rule text is measured and set by.
    pub fn metric_rules(&self) -> MetricRules {
        self.metric_rules
    }

    /// Sets the rule text is measured and set by from now on. What is cached for one rule
    /// is kept apart from what is cached for the other.
    pub fn set_metric_rules(&mut self, rules: MetricRules) {
        self.metric_rules = rules;
    }

    /// The engine the face `request` selects is hinted with under [`MetricRules::Rgss3`].
    /// `None` only when the engine has no usable font at all.
    pub fn hinting_engine(&mut self, request: &TextRequest) -> Option<HintingEngine> {
        let selected = self.selected_face(request)?;
        Some(self.faces[selected.face].hinting_engine())
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
        let hinted = self.metric_rules == MetricRules::Rgss3;
        let mut overhang = |placed: Option<&Placed>| match placed {
            Some(placed) => {
                let glyph = self.glyph(placed.key(pixels_per_em, (1, 1), hinted));
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
    /// the first and the last glyph as drawn; other ink beyond it is clipped. The glyphs are
    /// composed by the engine's [`MaskRules`].
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
        let hinted = self.metric_rules == MetricRules::Rgss3;
        let glyphs: Vec<(Arc<Glyph>, u32)> = run
            .iter()
            .zip(advances)
            .map(|(placed, &advance)| {
                let key = placed.key(setting.cell.pixels_per_em, condense, hinted);
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
            blit(
                &mut mask,
                glyph,
                pen + i64::from(glyph.left),
                top,
                self.mask_rules,
            );
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
        let average_width = match self.metric_rules {
            MetricRules::Plain => mul_div(face.average_width.into(), pixels_per_em, units_per_em),
            MetricRules::Rgss3 => face.metrics().average_width(pixels_per_em),
        };
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
            let (glyph_bold, glyph_italic) = self.face_style(found.face);
            // A glyph of a fallback face keeps its own advance, at the same pixels per em.
            let glyph_units = u32::from(self.faces[found.face].units_per_em);
            let natural = match self.metric_rules {
                MetricRules::Plain => advance_pixels(found.advance, pixels_per_em, glyph_units),
                MetricRules::Rgss3 => {
                    let face = &mut self.faces[found.face];
                    let hinted = face.hinted_advance(pixels_per_em, found.glyph);
                    face.metrics().advance(hinted, pixels_per_em)
                }
            };
            let mut advance = natural.saturating_add(u32::from(request.bold && !glyph_bold));
            // As measured: a character outside the Basic Multilingual Plane is two units to
            // GDI, each of which is a missing glyph.
            if found.missing && u32::from(ch) > 0xFFFF {
                advance = advance.saturating_mul(2);
            }
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
    /// A glyph of another face keeps that face's advance.
    /// A control character is the selected face's missing glyph whatever the face maps it
    /// to.
    ///
    /// PLACEHOLDER: the original, measured under Wine, draws the missing glyph for a
    /// character the font lacks.
    fn find_glyph(&mut self, selected: usize, bold: bool, italic: bool, ch: char) -> CharGlyph {
        let missing = CharGlyph {
            face: selected,
            glyph: 0,
            advance: self.faces[selected].advance(0),
            missing: true,
        };
        if ch.is_control() {
            return missing;
        }
        let own = self.faces[selected].glyph(ch);
        if own != 0 {
            return CharGlyph {
                face: selected,
                glyph: own,
                advance: self.faces[selected].advance(own),
                missing: false,
            };
        }
        let from = |faces: &[LoadedFace], face: usize| {
            let glyph = faces[face].glyph(ch);
            (glyph != 0).then_some(CharGlyph {
                face,
                glyph,
                advance: faces[face].advance(glyph),
                missing: false,
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
        if key.hinted {
            // A glyph whose hinted points do not match its outline is drawn plain.
            let rendered = match self.hinted_glyph(key) {
                Some(rendered) => Arc::new(rendered),
                None => self.glyph(GlyphKey {
                    hinted: false,
                    ..key
                }),
            };
            self.glyphs.insert(key, Arc::clone(&rendered));
            return rendered;
        }
        let loaded = &self.faces[key.face];
        let mut scaler = self
            .scaler
            .builder(loaded.font())
            .size(key.pixels_per_em as f32)
            .hint(false)
            .normalized_coords(loaded.coords())
            .build();
        let strength = if key.bold {
            key.pixels_per_em as f32 * SYNTHETIC_BOLD
        } else {
            0.0
        };
        let slant = if key.italic { SYNTHETIC_SLANT } else { 0.0 };
        let condense = numerator as f32 / denominator as f32;
        let transform = (key.italic || numerator != denominator)
            .then(|| Transform::new(condense, 0.0, slant, 1.0, 0.0, 0.0));

        // The box GDI takes the overhangs from: that of the outline's points, emboldened,
        // with its corners condensed and slanted.
        let extent = scaler
            .scale_outline(key.glyph)
            .filter(|outline| !outline.points().is_empty())
            .map(|mut outline| {
                if key.bold {
                    outline.embolden(strength, strength);
                }
                let bounds = outline.bounds();
                let (mut min, mut max) = (bounds.min, bounds.max);
                min.x *= condense;
                max.x *= condense;
                // As measured: the box of a glyph that is not emboldened is on whole
                // pixels before it is slanted (GDI has it from the hinted glyph, whose
                // overshoot below the baseline is gone), that of an emboldened one is not.
                if !key.bold {
                    let bias = if min.y < -1.0 { 0.5 } else { SHALLOW_BOTTOM };
                    min = Vector::new(min.x.floor(), (min.y + bias).floor());
                    max = max.ceil();
                }
                let left = min.x + (slant * min.y).min(slant * max.y);
                let right = max.x + (slant * min.y).max(slant * max.y);
                (left.floor() as i32, right.ceil() as i32)
            });
        let rendered = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .embolden(strength)
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
                    extent,
                    coverage: image.data,
                })
            })
            // A glyph without an outline, such as a space, draws nothing.
            .unwrap_or_default();
        let rendered = Arc::new(rendered);
        self.glyphs.insert(key, Arc::clone(&rendered));
        rendered
    }

    /// The image of a glyph by [`MetricRules::Rgss3`]: `swash`'s outline of the glyph with
    /// all its points moved to those of the hinted outline, emboldened, slanted and condensed
    /// as by the plain rule, and its box from that outline. `None` where the hinted outline
    /// has another number of points than `swash`'s.
    fn hinted_glyph(&mut self, key: GlyphKey) -> Option<Glyph> {
        let (numerator, denominator) = key.condense;
        // A glyph `skrifa` gives no hinted draw for draws nothing.
        let Some(hinted_points) = self.faces[key.face].hinted_points(key.pixels_per_em, key.glyph)
        else {
            return Some(Glyph::default());
        };
        let loaded = &self.faces[key.face];
        let mut scaler = self
            .scaler
            .builder(loaded.font())
            .size(key.pixels_per_em as f32)
            .hint(false)
            .normalized_coords(loaded.coords())
            .build();
        let Some(mut outline) = scaler
            .scale_outline(key.glyph)
            .filter(|outline| !outline.points().is_empty())
        else {
            // A glyph without an outline, such as a space, draws nothing.
            return Some(Glyph::default());
        };
        // Both are draws of `skrifa` with FreeType's path style, so the points come in the
        // same order.
        if outline.points().len() != hinted_points.len() {
            return None;
        }
        outline.points_mut().copy_from_slice(&hinted_points);
        if key.bold {
            let strength = key.pixels_per_em as f32 * SYNTHETIC_BOLD;
            outline.embolden(strength, strength);
        }
        let condense = numerator as f32 / denominator as f32;
        let bounds = outline.bounds();
        let scaled = |point: Point| Point::new(point.x * condense, point.y);
        let extent = hinted::extent(
            scaled(bounds.min),
            scaled(bounds.max),
            !key.bold,
            key.italic,
        );
        let slant = if key.italic { SYNTHETIC_SLANT } else { 0.0 };
        if key.italic || numerator != denominator {
            outline.transform(&Transform::new(condense, 0.0, slant, 1.0, 0.0, 0.0));
        }
        // zeno places a mask with a bottom-left origin by its size, which it has computed
        // only once asked for it; before that the top it gives is the bottom of the glyph.
        let (coverage, placement) = Mask::new(outline.path())
            .format(Format::Alpha)
            .origin(Origin::BottomLeft)
            .inspect(|_, _, _| {})
            .render();
        let bytes = placement.width as usize * placement.height as usize;
        if coverage.len() != bytes {
            return Some(Glyph::default());
        }
        Some(Glyph {
            left: placement.left,
            top: placement.top,
            width: placement.width,
            height: placement.height,
            extent: Some(extent),
            coverage,
        })
    }
}

impl Placed {
    fn key(&self, pixels_per_em: u32, condense: (u32, u32), hinted: bool) -> GlyphKey {
        GlyphKey {
            face: self.face,
            pixels_per_em,
            glyph: self.glyph,
            bold: self.bold,
            italic: self.italic,
            condense,
            hinted,
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

/// Composes `glyph` into `mask` with its top-left corner at (`left`, `top`) by `rules`,
/// dropping what falls outside the mask. With [`MaskRules::Plain`] the larger coverage stays
/// where glyphs overlap. With the rules of RGSS3 the glyph's values are its scaled levels,
/// and they replace what the mask held: wherever the level is above 0, or with
/// [`MaskRules::Rgss3Early`] over the whole image of the glyph, which stands for the black
/// box of GDI.
fn blit(mask: &mut TextMask, glyph: &Glyph, left: i64, top: i64, rules: MaskRules) {
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
            match rules {
                MaskRules::Plain => *to = (*to).max(*from),
                MaskRules::Rgss3 => {
                    let level = coverage_level(*from);
                    if level > 0 {
                        *to = scaled_level(level);
                    }
                }
                MaskRules::Rgss3Early => *to = scaled_level(coverage_level(*from)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FontSources;

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
        blit(&mut mask, &glyph(3, 1, 200), 0, 0, MaskRules::Plain);
        blit(&mut mask, &glyph(3, 1, 100), 1, 0, MaskRules::Plain);
        assert_eq!(mask.coverage, [200, 200, 200, 100]);
        blit(&mut mask, &glyph(2, 1, 255), 2, 0, MaskRules::Plain);
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
            blit(&mut mask, &numbered, left, top, MaskRules::Plain);
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
        blit(&mut empty, &numbered, 0, 0, MaskRules::Plain);
        assert!(empty.coverage.is_empty());
    }

    #[test]
    fn the_levels_of_rgss3_are_scaled_as_measured() {
        let scaled: Vec<u8> = [0, 1, 32, 49, 56, 63, 64]
            .into_iter()
            .map(scaled_level)
            .collect();
        assert_eq!(scaled, [0, 3, 127, 195, 223, 251, 255]);
    }

    #[test]
    fn a_larger_coverage_never_gives_a_smaller_level() {
        assert_eq!(coverage_level(0), 0);
        assert_eq!(coverage_level(255), 64);
        for coverage in 1..=255u8 {
            assert!(coverage_level(coverage) >= coverage_level(coverage - 1));
            assert!(coverage_level(coverage) <= 64);
        }
    }

    #[test]
    fn the_end_points_of_the_coverage_stay_with_rgss3() {
        let ends = Glyph {
            width: 2,
            height: 1,
            coverage: vec![0, 255],
            ..Glyph::default()
        };
        for rules in [MaskRules::Rgss3, MaskRules::Rgss3Early] {
            let mut mask = mask(2, 1);
            blit(&mut mask, &ends, 0, 0, rules);
            assert_eq!(mask.coverage, [0, 255], "{rules:?}");
        }
    }

    /// A glyph of full coverage 4 wide, then one whose box lies over its columns 1 and 2
    /// with a level between 0 and 64 in the first and 0 in the second.
    fn overlapped(rules: MaskRules) -> Vec<u8> {
        let mut mask = mask(4, 1);
        blit(&mut mask, &glyph(4, 1, 255), 0, 0, rules);
        let later = Glyph {
            width: 2,
            height: 1,
            coverage: vec![128, 0],
            ..Glyph::default()
        };
        blit(&mut mask, &later, 1, 0, rules);
        mask.coverage
    }

    #[test]
    fn later_ink_replaces_earlier_ink_in_rgss3() {
        // Coverage 128 is level 32, held as 127.
        assert_eq!(overlapped(MaskRules::Rgss3), [255, 127, 255, 255]);
        // The plain rule keeps the larger value.
        assert_eq!(overlapped(MaskRules::Plain), [255, 255, 255, 255]);
    }

    #[test]
    fn the_box_of_a_glyph_erases_in_the_early_rgss3() {
        // Outside the box of the second glyph the first one stays.
        assert_eq!(overlapped(MaskRules::Rgss3Early), [255, 127, 0, 255]);
    }

    /// The columns of an image `columns` wide and `height` high that hold ink.
    fn inked(columns: u32, height: u32, coverage: &[u8]) -> Vec<u32> {
        (0..columns)
            .filter(|&x| (0..height).any(|y| coverage[(y * columns + x) as usize] > 0))
            .collect()
    }

    // Requirement: Glyph images of RGSS3

    #[test]
    fn hinted_ink_lies_inside_the_box() {
        let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts");
        let mut engine = TextEngine::new(&FontSources::directories_only([fonts]));
        engine.set_metric_rules(MetricRules::Rgss3);
        for family in ["Liberation Sans", "Noto Sans Symbols 2"] {
            for size in [12, 24, 48] {
                for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                    let request = TextRequest {
                        bold,
                        italic,
                        ..TextRequest::new([family], size)
                    };
                    for ch in "Hello World".chars() {
                        let text = ch.to_string();
                        let context = format!("{ch:?} of {family} at {size}, {bold} {italic}");
                        let mask = engine.rasterize(&request, &text);
                        let setting = engine.set(&request, &text).unwrap();
                        let placed = engine.run[0];
                        let key = placed.key(setting.cell.pixels_per_em, (1, 1), true);
                        let glyph = engine.glyph(key);
                        let Some((left, right)) = glyph.extent else {
                            assert!(!mask.coverage.iter().any(|&c| c > 0), "{context}");
                            continue;
                        };
                        for x in inked(glyph.width, glyph.height, &glyph.coverage) {
                            let x = glyph.left + x as i32;
                            assert!(left <= x && x < right, "{context}: image column {x}");
                        }
                        for x in inked(mask.width, mask.height, &mask.coverage) {
                            let x = x as i32 - mask.left_overhang as i32;
                            assert!(left <= x && x < right, "{context}: mask column {x}");
                        }
                    }
                }
            }
        }
    }

    /// The rows of an image `columns` wide and `height` high that hold ink, from the top.
    fn inked_rows(columns: u32, height: u32, coverage: &[u8]) -> Vec<u32> {
        (0..height)
            .filter(|&y| (0..columns).any(|x| coverage[(y * columns + x) as usize] > 0))
            .collect()
    }

    /// The first and the last row of ink of `glyph`, as the rows above the baseline (y up)
    /// whose pixels they are; `None` for a glyph without ink.
    fn ink_rows(glyph: &Glyph) -> Option<(i32, i32)> {
        let rows = inked_rows(glyph.width, glyph.height, &glyph.coverage);
        let row = |index: u32| glyph.top - index as i32;
        Some((row(*rows.first()?), row(*rows.last()?)))
    }

    /// A copy of Liberation Sans in `directory` without `fpgm` and `prep`, which FreeType
    /// hints with its automatic hinter.
    fn auto_hinted_copy(directory: &std::path::Path) {
        let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts");
        let mut font = std::fs::read(fonts.join("LiberationSans-Regular.ttf")).unwrap();
        let tables = usize::from(u16::from_be_bytes([font[4], font[5]]));
        for record in (0..tables).map(|position| 12 + 16 * position) {
            if matches!(&font[record..record + 4], b"fpgm" | b"prep") {
                font[record + 3] = b'~';
            }
        }
        std::fs::write(directory.join("LiberationSans-Regular.ttf"), font).unwrap();
    }

    /// Draws `H`, `g`, `j`, `M` and `!` of `family`, the only font in `directory`, at 12, 20
    /// and 24 pixels per em by both metric rules, and checks that each hinted image is
    /// whole: its ink inside its hinted box, its first and last row of ink within a pixel of
    /// the plain image's.
    fn hinted_glyphs_are_whole(directory: &std::path::Path, family: &str, hinting: HintingEngine) {
        let mut engine = TextEngine::new(&FontSources {
            fallback_families: Some(Vec::new()),
            ..FontSources::directories_only([directory])
        });
        let request = TextRequest::new([family], 24);
        assert_eq!(engine.hinting_engine(&request), Some(hinting), "{family}");
        for ch in "HgjM!".chars() {
            engine.set(&request, &ch.to_string()).unwrap();
            let placed = engine.run[0];
            for pixels_per_em in [12, 20, 24] {
                let context = format!("{ch:?} of {family} at {pixels_per_em} pixels per em");
                let plain = engine.glyph(placed.key(pixels_per_em, (1, 1), false));
                let hinted = engine.glyph(placed.key(pixels_per_em, (1, 1), true));
                let (plain_first, plain_last) = ink_rows(&plain).expect(&context);
                let (first, last) = ink_rows(&hinted).expect(&context);
                assert!(
                    first.abs_diff(plain_first) <= 1 && last.abs_diff(plain_last) <= 1,
                    "{context}: rows {first}..={last}, plain {plain_first}..={plain_last}"
                );
                // The hinted box: that of the hinted outline's points on whole pixels.
                let points = engine.faces[placed.face]
                    .hinted_points(pixels_per_em, placed.glyph)
                    .expect(&context);
                let floor = |value: f32| value.floor() as i32;
                let ceil = |value: f32| value.ceil() as i32;
                let bottom = floor(points.iter().map(|point| point.y).fold(f32::MAX, f32::min));
                let top = ceil(points.iter().map(|point| point.y).fold(f32::MIN, f32::max));
                let left = floor(points.iter().map(|point| point.x).fold(f32::MAX, f32::min));
                let right = ceil(points.iter().map(|point| point.x).fold(f32::MIN, f32::max));
                // A row `y` holds the pixels from `y - 1` up to `y`, so the last row is
                // above the bottom of the box.
                assert!(
                    first <= top && last > bottom,
                    "{context}: rows {first}..={last}, box rows {bottom}..{top}"
                );
                for x in inked(hinted.width, hinted.height, &hinted.coverage) {
                    let x = hinted.left + x as i32;
                    assert!(
                        left <= x && x < right,
                        "{context}: column {x}, box {left}..{right}"
                    );
                }
            }
        }
    }

    // Requirement: Hinted glyph images are whole

    #[test]
    fn auto_hinted_glyphs_are_whole() {
        let directory = tempfile::tempdir().unwrap();
        auto_hinted_copy(directory.path());
        hinted_glyphs_are_whole(
            directory.path(),
            "Liberation Sans",
            HintingEngine::Automatic,
        );
    }

    #[test]
    fn interpreted_and_cff_hinted_glyphs_are_whole() {
        let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        hinted_glyphs_are_whole(&tests.join("fonts"), "Liberation Sans", HintingEngine::Font);
        hinted_glyphs_are_whole(
            &tests.join("fonts-cff"),
            "Source Code Pro",
            HintingEngine::Font,
        );
    }

    #[test]
    fn advances_round_twice() {
        // 812 units of 1000 at 8 pixels per em are 6.496 pixels: 6.5 in 26.6, then 7.
        assert_eq!(advance_pixels(812, 8, 1000), 7);
        assert_eq!(advance_pixels(1000, 15, 1000), 15);
        assert_eq!(advance_pixels(555, 8, 1000), 4);
        assert_eq!(advance_pixels(1000, 0, 1000), 0);
    }
}
