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
//! # Sources
//!
//! [`FontSources`] names where the fonts come from: the fonts installed on the system (found
//! through its font configuration), directories of the caller, or both. Directories are
//! searched recursively; collections and variable fonts are supported. A family found in a
//! caller directory hides the system's faces of the same family name, and an earlier caller
//! directory hides a later one. The engine does not read the configuration file: the caller
//! passes the directories, extra substitutions and fallback families.
//!
//! For a given set of font files the results are the same on every run. A font file is read
//! into memory the first time one of its faces is used.
//!
//! # Lookup order
//!
//! A request names families in order of preference. Names are compared without regard to
//! letter case, against every family name a font declares, localised ones such as `黑体`
//! included. For each requested name in turn:
//!
//! 1. the family of that name;
//! 2. each replacement the substitution table lists for the name, the caller's entries before
//!    the built-in ones ([`default_substitutions`]), which cover the common Windows families.
//!
//! When no name yields a font, the first installed fallback family is used
//! ([`DEFAULT_FALLBACK_FAMILIES`] unless the caller names others), and failing that any font.
//! [`TextEngine::is_available`] answers steps 1 and 2 only.
//!
//! Within the family, a bold or italic request selects the face of that weight class and
//! slant when the family has one and the regular face otherwise; nothing is synthesised. A
//! font with a weight axis is set to weight 400, or 700 for a bold request.
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
//! The requested size is the height of the selected face's line box: the face is scaled so
//! that its ascent plus descent equals the size, taking the OS/2 `usWinAscent` and
//! `usWinDescent` as Windows does and falling back to `hhea`. The baseline lies the rounded
//! ascent below the top of the mask, and the mask is always as high as the size; ink beyond
//! it is clipped. A string's width is the sum of its characters' advances, each rounded to a
//! whole pixel on its own. There is no kerning, shaping or hinting, and substitute fonts do
//! not have the advance widths of the fonts they replace.
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
mod sources;
mod substitution;

pub use engine::{
    DEFAULT_FALLBACK_FAMILIES, DEFAULT_GLYPH_CACHE_BYTES, ResolvedFace, TextEngine, TextMask,
    TextRequest,
};
pub use sources::{FontSources, SystemFonts};
pub use substitution::default_substitutions;
