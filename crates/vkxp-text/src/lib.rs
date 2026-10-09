//! Font lookup, substitution table, glyph rasterisation.
//!
//! Side: guest.
//!
//! A [`TextEngine`] finds a font for the family names a game asks for and turns a string into
//! an 8-bit coverage mask at a pixel size. It knows nothing of `vkxp-core`: the binding adapts
//! it to core's rasteriser interface.
//!
//! ```no_run
//! use vkxp_text::{FontSources, TextEngine, TextRequest};
//!
//! let mut engine = TextEngine::new(&FontSources::system());
//! let request = TextRequest::new(["楷体_GB2312", "黑体"], 22);
//! let (width, height) = engine.measure(&request, "开始游戏");
//! let mask = engine.rasterize(&request, "开始游戏");
//! assert_eq!((mask.width, mask.height), (width, height));
//! ```
//!
//! The caller's own fonts, substitutions and line boxes go into the sources:
//!
//! ```no_run
//! use vkxp_text::{FontSources, LineBox, TextEngine};
//!
//! let sources = FontSources {
//!     directories: vec!["/home/player/.config/vkxp/fonts".into()],
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
//! Within the family, a bold, italic or bold-italic request selects the face of exactly that
//! weight class and slant when the family has one and the regular face otherwise; nothing is
//! synthesised. A font with a weight axis is set to weight 400, or 700 for a bold request.
//!
//! A character the selected face lacks is drawn from the first fallback family that has it,
//! then from any font that has it, at the selected face's pixels per em and on its baseline.
//! A character no font has is drawn as the selected face's missing-glyph box. Control
//! characters, line breaks included, take no space and draw nothing.
//!
//! # Metrics
//!
//! PLACEHOLDER: the rule below is a guess at what the original runtime passes to GDI, pending
//! measurement against it. It is confined to `line_metrics` in `face.rs`.
//!
//! The requested size is the height of the line box the text is set in: the selected face is
//! scaled so that the ascent plus descent of that line box equals the size. The baseline lies
//! the rounded ascent below the top of the mask, and the mask is always as high as the size;
//! ink beyond it is clipped. A string's width is the sum of its characters' advances, each
//! rounded to a whole pixel on its own. There is no kerning, shaping or hinting.
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
//! PLACEHOLDER: most built-in line boxes are from memory of the Windows fonts' metric tables;
//! the list in `line_box.rs` marks each.
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
mod index;
mod line_box;
mod sources;
mod substitution;

pub use engine::{
    DEFAULT_FALLBACK_FAMILIES, DEFAULT_GLYPH_CACHE_BYTES, ResolvedFace, TextEngine, TextMask,
    TextRequest,
};
pub use line_box::{LineBox, default_line_box};
pub use sources::{FontSources, SystemFonts};
pub use substitution::default_substitutions;
