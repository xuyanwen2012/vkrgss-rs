//! Font lookup, substitution table, glyph rasterisation.
//!
//! Side: guest.
//!
//! A [`TextEngine`] finds a font for the family names a game asks for and turns a string into
//! an 8-bit coverage mask at a pixel size. It knows nothing of `vkrgss-core`: the binding adapts
//! it to core's rasteriser interface.
//!
//! ```no_run
//! use vkrgss_text::{FontSources, TextEngine, TextRequest};
//!
//! let mut engine = TextEngine::new(&FontSources::system());
//! let request = TextRequest::new(["楷体_GB2312", "黑体"], 22);
//! let (width, height) = engine.measure(&request, "开始游戏");
//! let mask = engine.rasterize(&request, "开始游戏");
//! assert!(mask.width >= width);
//! assert_eq!(mask.height, height);
//! ```
//!
//! The caller's own fonts, substitutions and line boxes go into the sources:
//!
//! ```no_run
//! use vkrgss_text::{FontSources, LineBox, TextEngine};
//!
//! let sources = FontSources {
//!     directories: vec!["/home/player/.config/vkrgss/fonts".into()],
//!     substitutions: vec![("My Game Font".into(), vec!["Noto Sans CJK SC".into()])],
//!     line_boxes: vec![(
//!         "My Game Font".into(),
//!         LineBox { ascent: 880, descent: 120, units_per_em: 1000 },
//!     )],
//!     ..FontSources::system()
//! };
//! let engine = TextEngine::new(&sources);
//! assert!(engine.is_available("My Game Font"));
//! ```
//!
//! # Sources
//!
//! [`FontSources`] names where the fonts come from: the fonts installed on the system (found
//! through its font configuration), directories of the caller, or both. Directories are
//! searched recursively; collections and variable fonts are supported. A family found in a
//! caller directory hides the system's faces of the same family name, and an earlier caller
//! directory hides a later one. A face hidden under every family name it declares is not
//! used at all, not even as a fallback; a face that keeps a name of its own stays in use. The engine does not read the configuration file: the caller
//! passes the directories, extra substitutions, line boxes and fallback families.
//!
//! For a given set of font files the results are the same on every run. A font file is read
//! into memory the first time one of its faces is used.
//!
//! # Lookup order
//!
//! A request names families in order of preference. Names are compared without regard to
//! letter case, against every family name a font declares in its `name` table: typographic
//! (name ID 16) and legacy (name ID 1) names, in every language, so localised names such as
//! `黑体` match. For each requested name in turn:
//!
//! 1. the family of that name;
//! 2. each replacement the substitution table lists for the name, the caller's entries before
//!    the built-in ones ([`default_substitutions`]), which cover the common Windows families.
//!
//! When no name yields a font, the first installed fallback family is used
//! ([`DEFAULT_FALLBACK_FAMILIES`] unless the caller names others), and failing that any font.
//! [`TextEngine::is_available`] answers steps 1 and 2 only.
//!
//! RGSS3 asks another question of a name, which changes nothing of the lookup:
//! [`TextEngine::has_family_exactly`] is true for a name that equals a declared family name
//! character for character, letter case included, or a name of the substitution table as the
//! table spells it when one of its replacements is installed. From a list of names
//! [`TextEngine::first_existing`] chooses the first that exists so, and the first of the list
//! when none does; the chosen name is then looked up as above.
//!
//! Within the family, a bold, italic or bold-italic request selects the face of exactly that
//! weight class and slant when the family has one and the regular face otherwise. A font with
//! a weight axis is set to weight 400, or 700 for a bold request. A style the selected face
//! does not have is synthesised as GDI was measured to do it: bold by emboldening the
//! outline by a 24th of the em and making every advance, and the average width, one pixel
//! wider; italic by slanting the outline by a quarter, which keeps the advances and gives
//! the glyphs overhangs. The overhangs of such glyphs are those of the box GDI takes them
//! from: the box of the emboldened outline, slanted at its corners.
//!
//! A character the selected face lacks is drawn from the first fallback family that has it,
//! then from any font that has it, at the selected face's pixels per em and on its baseline,
//! and keeps the advance it has in that font, as a linked font does on Windows. A character
//! no font has is drawn as the selected face's missing-glyph box. A control character, line
//! breaks included, is measured and drawn as the selected face's missing glyph, also where
//! the face maps it to a glyph of its own, and none starts a new line. A character no font
//! has that lies outside the Basic Multilingual Plane is as wide as two missing glyphs.
//!
//! PLACEHOLDER: the original runtime was measured under Wine, which has no font linking and
//! draws the missing glyph for every character the font lacks; drawing it from another font
//! is kept because it is better for play.
//!
//! # Metrics
//!
//! The rules are those measured on the original runtime under Wine's GDI, which rasterises
//! with FreeType ([`TextEngine::layout`] reports the results as a [`TextLayout`]).
//!
//! The requested size is the height of the cell asked of GDI. The face is set at a whole
//! number of pixels per em: `round(upm * size / box)`, one less when `round(box * ppem / upm)`
//! would exceed the size, where `box` is the ascent plus descent of the line box. The text
//! height is the rounded ascent plus the rounded descent at that pixels per em, which is not
//! always the size, and the baseline lies the rounded ascent below the top of the mask.
//!
//! PLACEHOLDER: the decrement in the choice of pixels per em is Wine's; it is confined to
//! `LineBox::pixels_per_em` in `line_box.rs`.
//!
//! Advances, overhangs, the average width and the glyph images follow the engine's
//! [`MetricRules`], which [`TextEngine::set_metric_rules`] sets apart from the mask rules.
//! With the plain rule, the default, each advance is rounded to a whole pixel at that pixels
//! per em the way FreeType rounds it, the average width is the OS/2 `xAvgCharWidth` scaled
//! and rounded, and glyphs are not hinted. With the rule of RGSS3 each glyph is hinted at
//! that pixels per em as Wine's FreeType hints it for an anti-aliased target
//! ([`TextEngine::hinting_engine`]: the automatic hinter for a TrueType font without `fpgm`
//! whose `prep` is at most 7 bytes, the font's own engine otherwise), and GDI's answers under
//! Wine are taken from the hinted glyph: the hinted advance rounded up, a full-width glyph of
//! a font whose panose says monospaced twice the scaled average width, the overhangs from the
//! control box of the hinted outline (on whole pixels, or for a synthesised bold that of the
//! emboldened outline as it is, its corners slanted for a synthesised italic), the average
//! width `MulFix(xAvgCharWidth, MulDiv(ppem, 65536, upm))`, and the glyph images rasterised
//! from the hinted outline. Pixels per em and text heights are the same with both rules.
//!
//! A string's width is the sum of its characters' advances plus the amount by which the
//! first glyph reaches left of the pen (its left overhang); an empty string measures 0 by 0.
//! There is no kerning or shaping. A mask is as high as the text and holds the
//! advances and the overhangs of the first and the last glyph; other ink beyond it is
//! clipped. [`TextEngine::rasterize_run`] sets a string with advances of the caller and
//! glyphs condensed horizontally, which is how text is squeezed into a narrow rectangle.
//!
//! The line box is the selected face's own, from the OS/2 `usWinAscent` and `usWinDescent` as
//! Windows takes them, falling back to `hhea`, with one exception. A face that stands in for
//! a requested name through the substitution table is set in the line box of that name when
//! it is known ([`LineBox`]): the caller's entries before the built-in ones
//! ([`default_line_box`]), which cover the Windows families whose metrics are established.
//! The replacement then has the em size the named font would have had, so a full-width
//! character is as wide as in the original; other characters keep the replacement's own
//! advances relative to its em, and ink that the replaced family's line box does not hold,
//! such as the descenders of a larger replacement, is clipped. The line box belongs to the
//! requested name, not to the replacement. A font found under the requested name, a
//! replacement for a name without a known line box, and a fallback font keep their own.
//!
//! With [`FontSources::own_metrics`] set there is no exception: a replacement is set in its
//! own line box like any other font. That is how GDI sizes a font it is given, so it is the
//! rule to compare against the original when both draw with the same font file.
//!
//! PLACEHOLDER: most built-in line boxes are from memory of the Windows fonts' metric tables;
//! the list in `line_box.rs` marks each.
//!
//! # Mask rules
//!
//! A mask is made by the engine's [`MaskRules`], which [`TextEngine::set_mask_rules`] sets.
//! With the plain rule, the default, a mask value is the coverage of the rasteriser and the
//! larger value stays where glyphs overlap. With the two rules of RGSS3 the coverage first
//! becomes one of 65 levels and the mask holds `trunc(level * 255 / 64)`, as measured on the
//! original runtime of VX Ace, and a later glyph replaces what an earlier one wrote: where
//! its own level is above 0 (DLL 3.0.1.1), or over the whole of its image (DLL 3.0.0.1).
//! Sizes and advances are the same with every rule.
//!
//! PLACEHOLDER: the step from the rasteriser's 256 values to the 65 levels stands for GDI's
//! own 65-level answer; see `coverage_level` in `engine.rs`.
//!
//! # Caching
//!
//! Resolved requests, character-to-glyph mappings and rendered glyphs are cached. The glyph
//! images are bounded by total bytes ([`DEFAULT_GLYPH_CACHE_BYTES`], see
//! [`TextEngine::set_glyph_cache_limit`]) and the least recently used are dropped first.

#![forbid(unsafe_code)]

mod engine;
mod face;
mod glyph_cache;
mod hinted;
mod index;
mod line_box;
mod sources;
mod substitution;

pub use engine::{
    DEFAULT_FALLBACK_FAMILIES, DEFAULT_GLYPH_CACHE_BYTES, MaskRules, MetricRules, ResolvedFace,
    TextEngine, TextLayout, TextMask, TextRequest,
};
pub use hinted::HintingEngine;
pub use line_box::{LineBox, default_line_box};
pub use sources::{FontSources, SystemFonts};
pub use substitution::default_substitutions;
